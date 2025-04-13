//! Implementation of the compiler inside the `Jsompiler`.
use super::{
  JFunc, JFuncResult, JResult, JValue, Jsompiler, Json, Section,
  utility::{format_err, obj_json},
};
use core::{error::Error, fmt::Write as _};
use std::{
  fs::File,
  io::{self, BufWriter, Write as _},
};
#[expect(clippy::single_call_fn, reason = "function pointer")]
impl Jsompiler {
  /// Assert condition.
  #[expect(clippy::panic_in_result_fn, reason = "panic don't occurred")]
  fn assert(&self, cond: bool, text: &str, obj: &Json) -> Result<(), Box<dyn Error>> {
    if cond {
      assert!(cond, "{text}");
      Ok(())
    } else {
      Err(format_err(text, obj.pos, obj.line, &self.source).into())
    }
  }
  /// Utility functions for binary operations
  fn binary_op(
    &mut self,
    args: &[Json],
    function: &mut String,
    mnemonic: &str,
    error_context: &str,
  ) -> JFuncResult {
    self.assert(
      args.len() >= 3,
      &format!("'{error_context}' requires at least two arguments"),
      args.first().ok_or("Unreachable (binary_op)")?,
    )?;
    let result = self.eval_args(args.get(1..).ok_or("Unreachable (binary_op)")?, function)?;
    match *result.first().ok_or("Unreachable (binary_op)")? {
      JValue::Int(int) => writeln!(function, "  mov rax, {int}")?,
      JValue::IntVar(ref iv) => writeln!(function, "  mov rax, qword ptr {iv}[rip]")?,
      ref invalid @ (JValue::Array(_)
      | JValue::ArrayVar(_)
      | JValue::Bool(_)
      | JValue::BoolVar(..)
      | JValue::Float(_)
      | JValue::FloatVar(_)
      | JValue::FuncVar { .. }
      | JValue::Null
      | JValue::Object(_)
      | JValue::ObjectVar(_)
      | JValue::String(_)
      | JValue::StringVar(_)) => {
        return Err(
          format!("'{error_context}' requires integer operands, but got '{invalid}'").into(),
        );
      }
    }
    for arg in result.get(1..).ok_or("Unreachable (binary_op)")? {
      match *arg {
        JValue::Int(int) => writeln!(function, "  {mnemonic} rax, {int}")?,
        JValue::IntVar(ref var) => writeln!(function, "  {mnemonic} rax, qword ptr {var}[rip]")?,
        JValue::Array(_)
        | JValue::ArrayVar(_)
        | JValue::Bool(_)
        | JValue::BoolVar(..)
        | JValue::Float(_)
        | JValue::FloatVar(_)
        | JValue::FuncVar { .. }
        | JValue::Null
        | JValue::Object(_)
        | JValue::ObjectVar(_)
        | JValue::String(_)
        | JValue::StringVar(_) => {
          return Err(
            format!("'{error_context}' requires integer operands, but got '{arg}'").into(),
          );
        }
      }
    }
    let ret = self.get_name()?;
    writeln!(self.sect.bss, "  .lcomm {ret}, 8")?;
    writeln!(function, "  mov qword ptr {ret}[rip], rax")?;
    Ok(JValue::IntVar(ret))
  }
  /// Builds the assembly code from the parsed JSON.
  ///
  /// This function is the main entry point for the compilation process. It takes the parsed JSON,
  /// sets up the initial function table, evaluates the JSON, and writes the resulting assembly
  /// code to a file.
  ///
  /// # Arguments
  ///
  /// * `parsed` - The parsed JSON object.
  /// * `json_file` - The name of the original JSON file.
  /// * `filename` - The name of the file to write the assembly code to.
  ///
  /// # Returns
  ///
  /// * `Ok(Json)` - The result of the evaluation.
  /// * `Err(Box<dyn Error>)` - If an error occurred during the compilation process.
  ///
  /// # Errors
  ///
  /// * `Box<dyn Error>` - If an error occurred during the compilation process.
  #[inline]
  pub fn build(
    &mut self,
    parsed: String,
    json_file: &str,
    filename: &str,
  ) -> Result<(), Box<dyn Error>> {
    let json = self.parse(parsed)?;
    self.seed = 0;
    self.register("=", Jsompiler::f_set_local);
    self.register("$", Jsompiler::f_get_local);
    self.register("+", Jsompiler::f_plus);
    self.register("-", Jsompiler::f_minus);
    self.register("message", Jsompiler::f_message);
    self.register("begin", Jsompiler::f_begin);
    let mut main_func = String::new();
    self.sect = Section::default();
    let result = self.eval(&json, &mut main_func)?;
    writeln!(
      main_func,
      "  {}
  call [qword ptr __imp_ExitProcess[rip]]
  .seh_endproc",
      match result.value {
        JValue::Int(int) => format!("mov rcx, {int}"),
        JValue::IntVar(ref var) => format!("mov rcx, qword ptr {var}[rip]"),
        JValue::Array(_)
        | JValue::ArrayVar(_)
        | JValue::Bool(_)
        | JValue::BoolVar(..)
        | JValue::Float(_)
        | JValue::FloatVar(_)
        | JValue::FuncVar { .. }
        | JValue::Null
        | JValue::Object(_)
        | JValue::ObjectVar(_)
        | JValue::String(_)
        | JValue::StringVar(_) => String::from("xor ecx, ecx"),
      }
    )?;
    self.write_file(&main_func, filename, json_file)?;
    Ok(())
  }
  /// Create compile error.
  fn err_compile(&self, text: &str, obj: &Json) -> JResult {
    Err(format_err(text, obj.pos, obj.line, &self.source).into())
  }
  /// Create function error.
  fn err_func(&self, text: &str, obj: &Json) -> JFuncResult {
    Err(format_err(text, obj.pos, obj.line, &self.source).into())
  }
  /// Evaluates a JSON object.
  fn eval(&mut self, parsed: &Json, function: &mut String) -> JResult {
    let JValue::Array(ref list) = parsed.value else {
      return Ok(parsed.clone());
    };
    self.assert(!list.is_empty(), "An function call cannot be an empty list.", parsed)?;
    match list.first().ok_or("Unreachable (eval)")?.value {
      JValue::String(ref cmd) => {
        if cmd == "lambda" {
          let mut func_buffer = String::new();
          let result = Ok(self.eval_lambda(parsed, &mut func_buffer)?);
          self.sect.text.push_str(&func_buffer);
          result
        } else if let Some(func) = self.f_table.get(cmd.as_str()) {
          Ok(obj_json(func(self, list, function)?, list.first().ok_or("Unreachable (eval)")?))
        } else {
          self.err_compile(
            &format!("Function {cmd} is undefined."),
            list.first().ok_or("Unreachable (eval)")?,
          )
        }
      }
      JValue::Array(_) => {
        let mut func_buffer = String::new();
        let tmp = self.vars.clone();
        let lambda =
          self.eval_lambda(list.first().ok_or("Unreachable (eval)")?, &mut func_buffer)?;
        let JValue::FuncVar { name: ref n, ret: ref re, .. } = lambda.value else {
          return self.err_compile("InternalError: 'lambda' don't return lambda object.", &lambda);
        };
        self.sect.text.push_str(&func_buffer);
        writeln!(function, "  call {n}")?;
        self.vars = tmp;
        match **re {
          JValue::IntVar(_) => {
            let na = self.get_name()?;
            writeln!(self.sect.bss, "  .lcomm {na}, 8")?;
            writeln!(function, "  mov qword ptr {na}[rip], rax")?;
            Ok(obj_json(JValue::IntVar(na), &lambda))
          }
          JValue::Int(_) => {
            let na = self.get_name()?;
            writeln!(self.sect.bss, "  .lcomm {na}, 8")?;
            writeln!(function, "  mov qword ptr {na}[rip], rax")?;
            Ok(obj_json(JValue::IntVar(na), &lambda))
          }
          JValue::Array(_)
          | JValue::ArrayVar(_)
          | JValue::Bool(_)
          | JValue::BoolVar(..)
          | JValue::Float(_)
          | JValue::FloatVar(_)
          | JValue::FuncVar { .. }
          | JValue::Null
          | JValue::Object(_)
          | JValue::ObjectVar(_)
          | JValue::String(_)
          | JValue::StringVar(_) => Ok(Json::default()),
        }
      }
      JValue::ArrayVar(_)
      | JValue::Bool(_)
      | JValue::BoolVar(..)
      | JValue::Float(_)
      | JValue::FloatVar(_)
      | JValue::FuncVar { .. }
      | JValue::Int(_)
      | JValue::IntVar(_)
      | JValue::Null
      | JValue::Object(_)
      | JValue::ObjectVar(_)
      | JValue::StringVar(_) => self.err_compile(
        "The first element of an evaluation list requires a function name or a lambda object.",
        parsed,
      ),
    }
  }
  /// Evaluate arguments.
  fn eval_args(
    &mut self,
    args: &[Json],
    function: &mut String,
  ) -> Result<Vec<JValue>, Box<dyn Error>> {
    let mut result = vec![];
    for arg in args {
      result.push(self.eval(arg, function)?.value);
    }
    Ok(result)
  }
  /// Evaluates a lambda function definition.
  fn eval_lambda(&mut self, func: &Json, function: &mut String) -> JResult {
    self.assert(matches!(func.value, JValue::Array(_)), "Invalid function definition", func)?;
    let JValue::Array(ref func_list) = func.value else {
      return self.err_compile("Invalid function definition", func);
    };
    self.assert(func_list.len() >= 3, "Invalid function definition", func)?;
    let lambda = func_list.first().ok_or("Unreachable (eval_lambda)")?;
    self.assert(
      matches!(lambda.value, JValue::String(ref st) if st == "lambda"),
      "The first element of a lambda list requires \"lambda\".",
      lambda,
    )?;
    let JValue::Array(ref params) = func_list.get(1).ok_or("Unreachable (eval_lambda)")?.value
    else {
      return self.err_compile(
        "The second element of a lambda list requires an argument list.",
        func_list.get(1).ok_or("Unreachable (eval_lambda)")?,
      );
    };
    self.assert(
      params.is_empty(),
      "PARAMS IS TODO.",
      func_list.get(1).ok_or("Unreachable (eval_lambda)")?,
    )?;
    let n = self.get_name()?;
    writeln!(
      function,
      ".section .text${},\"x\"
.seh_proc {n}
{n}:
  push rbp
  .seh_pushreg rbp
  mov rbp, rsp
  .seh_setframe rbp, 0
  sub rsp, 32
  .seh_stackalloc 32
  .seh_endprologue
  .seh_handler .L_SEH_HANDLER, @except",
      &n[3..]
    )?;
    let result = self
      .eval_args(func_list.get(2..).ok_or("Unreachable (eval_lambda)")?, function)?
      .last()
      .ok_or_else(|| format_err("Empty lambda body", lambda.pos, lambda.line, &self.source))?
      .clone();
    writeln!(
      function,
      "  {}
  add rsp, 32
  leave
  ret
  .seh_endproc",
      match result {
        JValue::Int(int) => format!("mov rax, {int}"),
        JValue::IntVar(ref var) => format!("mov rax, qword ptr {var}[rip]"),
        JValue::Array(_)
        | JValue::ArrayVar(_)
        | JValue::Bool(_)
        | JValue::BoolVar(..)
        | JValue::Float(_)
        | JValue::FloatVar(_)
        | JValue::FuncVar { .. }
        | JValue::Null
        | JValue::Object(_)
        | JValue::ObjectVar(_)
        | JValue::String(_)
        | JValue::StringVar(_) => "xor eax, eax".into(),
      }
    )?;
    Ok(obj_json(JValue::FuncVar { name: n, params: params.clone(), ret: Box::new(result) }, lambda))
  }
  /// Evaluates a 'begin' block.
  fn f_begin(&mut self, args: &[Json], function: &mut String) -> JFuncResult {
    self.assert(
      args.len() >= 2,
      "'begin' requires at least one arguments",
      args.first().ok_or("Unreachable (begin)")?,
    )?;
    let begin = args.first().ok_or("Unreachable (f_begin)")?;
    Ok(
      self
        .eval_args(args.get(1..).ok_or("Unreachable (f_begin)")?, function)?
        .last()
        .ok_or_else(|| format_err("Empty lambda body", begin.pos, begin.line, &self.source))?
        .clone(),
    )
  }
  /// Gets the value of a local variable.
  fn f_get_local(&mut self, args: &[Json], _: &mut String) -> JFuncResult {
    self.assert(
      args.len() == 2,
      "'$' requires one argument",
      args.first().ok_or("Unreachable (f_get_local)")?,
    )?;
    let var = args.get(1).ok_or("Unreachable (f_get_local)")?;
    let JValue::String(ref var_name) = var.value else {
      return self.err_func("Variable name requires compile-time fixed string", var);
    };
    if let Some(value) = self.vars.get(var_name) {
      Ok(value.clone())
    } else {
      self.err_func(&format!("Undefined variables: '{var_name}'"), var)
    }
  }
  /// Displays a message box.
  fn f_message(&mut self, args: &[Json], function: &mut String) -> JFuncResult {
    self.assert(
      args.len() == 3,
      "'message' requires two arguments",
      args.first().ok_or("Unreachable (f_message)")?,
    )?;
    let title_json = args.get(1).ok_or("Unreachable (f_message)")?;
    let title = match self.eval(title_json, function)?.value {
      JValue::String(st) => {
        let name = self.get_name()?;
        writeln!(self.sect.data, "  {name}: .string \"{st}\"")?;
        name
      }
      JValue::StringVar(var) => var,
      JValue::Array(_)
      | JValue::ArrayVar(_)
      | JValue::Bool(_)
      | JValue::BoolVar(..)
      | JValue::Float(_)
      | JValue::FloatVar(_)
      | JValue::FuncVar { .. }
      | JValue::Int(_)
      | JValue::IntVar(_)
      | JValue::Null
      | JValue::Object(_)
      | JValue::ObjectVar(_) => {
        return self.err_func("The first argument of message must be a string", title_json);
      }
    };
    let msg_json = args.get(2).ok_or("Unreachable (f_message)")?;
    let msg = match self.eval(msg_json, function)?.value {
      JValue::String(st) => {
        let name = self.get_name()?;
        writeln!(self.sect.data, "  {name}: .string \"{st}\"")?;
        name
      }
      JValue::StringVar(var) => var,
      JValue::Array(_)
      | JValue::ArrayVar(_)
      | JValue::Bool(_)
      | JValue::BoolVar(..)
      | JValue::Float(_)
      | JValue::FloatVar(_)
      | JValue::FuncVar { .. }
      | JValue::Int(_)
      | JValue::IntVar(_)
      | JValue::Null
      | JValue::Object(_)
      | JValue::ObjectVar(_) => {
        return self.err_func("The second argument of message must be a string", msg_json);
      }
    };
    let wtitle = self.get_name()?;
    let wmsg = self.get_name()?;
    let ret = self.get_name()?;
    writeln!(
      self.sect.bss,
      "  .lcomm {wtitle}, 8
  .lcomm {wmsg}, 8
  .lcomm {ret}, 8"
    )?;
    write!(
      function,
      include_str!("message.s"),
      msg, msg, wmsg, title, title, wtitle, wmsg, wtitle, ret, wmsg, wtitle
    )?;
    Ok(JValue::IntVar(ret))
  }
  /// Performs subtraction.
  fn f_minus(&mut self, args: &[Json], function: &mut String) -> JFuncResult {
    self.binary_op(args, function, "sub", "-")
  }
  /// Performs addition.
  fn f_plus(&mut self, args: &[Json], function: &mut String) -> JFuncResult {
    self.binary_op(args, function, "add", "+")
  }
  /// Sets a local variable.
  fn f_set_local(&mut self, args: &[Json], function: &mut String) -> JFuncResult {
    self.assert(
      args.len() == 3,
      "'=' requires two arguments",
      args.first().ok_or("Unreachable (f_set_local)")?,
    )?;
    let JValue::String(ref var_name) = args.get(1).ok_or("Unreachable (f_set_local)")?.value else {
      return self.err_func(
        "Variable name requires compile-time fixed strings",
        args.get(1).ok_or("Unreachable (f_set_local)")?,
      );
    };
    let result = self.eval(args.get(2).ok_or("Unreachable (f_set_local)")?, function)?;
    let n = self.get_name()?;
    match result.value {
      JValue::String(ref st) => {
        writeln!(self.sect.data, "  {n}: .string \"{st}\"")?;
        self.vars.insert(var_name.clone(), JValue::StringVar(n.clone()));
        Ok(JValue::StringVar(n))
      }
      JValue::StringVar(ref sv) => {
        self.vars.insert(var_name.clone(), JValue::StringVar(sv.clone()));
        Ok(result.value)
      }
      JValue::Array(_)
      | JValue::ArrayVar(_)
      | JValue::Bool(_)
      | JValue::BoolVar(..)
      | JValue::Float(_)
      | JValue::FloatVar(_)
      | JValue::FuncVar { .. }
      | JValue::Int(_)
      | JValue::IntVar(_)
      | JValue::Null
      | JValue::Object(_)
      | JValue::ObjectVar(_) => Err("Assignment to an unimplemented type".into()),
    }
  }
  /// Generates a unique name for internal use.
  fn get_name(&mut self) -> Result<String, Box<dyn Error>> {
    let Some(res) = self.seed.checked_add(1) else { return Err("Seed Overflow".into()) };
    self.seed = res;
    Ok(format!(".LC{:x}", self.seed))
  }
  /// Registers a function in the function table.
  fn register(&mut self, name: &str, func: JFunc<Self>) {
    self.f_table.insert(name.into(), func);
  }
  /// Writes the compiled assembly code to a file.
  fn write_file(&self, main_func: &str, filename: &str, json_file: &str) -> io::Result<()> {
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(format!(".file \"{json_file}\"\n.intel_syntax noprefix\n").as_bytes())?;
    writer.write_all(include_bytes!("data.s"))?;
    writer.write_all(self.sect.data.as_bytes())?;
    writer.write_all(include_bytes!("bss.s"))?;
    writer.write_all(self.sect.bss.as_bytes())?;
    writer.write_all(include_bytes!("start.s"))?;
    writer.write_all(main_func.as_bytes())?;
    writer.write_all(include_bytes!("text.s"))?;
    writer.write_all(self.sect.text.as_bytes())?;
    writer.flush()?;
    Ok(())
  }
}
