//! Implementation of the compiler inside the `Jsompiler`.
extern crate alloc;
use super::{
  BuiltinFunc, JFunc, JFuncResult, JResult, JValue, Jsompiler, Json, Section,
  utility::{format_err, obj_json},
};
use alloc::borrow::Cow;
use core::{error::Error, fmt::Write as _};
use std::fs::File;
use std::io::{self, BufWriter, Write as _};
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
  /// Builds the assembly code from the parsed JSON.
  /// This function is the main entry point for the compilation process. It takes the parsed JSON,
  /// sets up the initial function table, evaluates the JSON, and writes the resulting assembly
  /// code to a file.
  /// # Arguments
  /// * `parsed` - The parsed JSON object.
  /// * `json_file` - The name of the original JSON file.
  /// * `filename` - The name of the file to write the assembly code to.
  /// # Returns
  /// * `Ok(Json)` - The result of the evaluation.
  /// * `Err(Box<dyn Error>)` - If an error occurred during the compilation process.
  /// # Errors
  /// * `Box<dyn Error>` - If an error occurred during the compilation process.
  #[inline]
  pub fn build(
    &mut self, parsed: String, json_file: &str, filename: &str,
  ) -> Result<(), Box<dyn Error>> {
    let json = self.parse(parsed)?;
    self.seed = 0;
    self.register("=", true, Jsompiler::f_local_set);
    self.register("$", true, Jsompiler::f_local_get);
    self.register("+", true, Jsompiler::f_plus);
    self.register("-", true, Jsompiler::f_minus);
    self.register("message", true, Jsompiler::f_message);
    self.register("begin", true, Jsompiler::f_begin);
    let mut start = String::new();
    self.sect = Section::default();
    let result = self.eval(&json, &mut start)?;
    writeln!(
      start,
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
        | JValue::StringVar(_) => "xor ecx, ecx".into(),
      }
    )?;
    self.write_file(&start, filename, json_file)?;
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
    let first = list.first().ok_or("Unreachable (eval)")?;
    match first.value {
      JValue::String(ref cmd) => {
        if cmd == "lambda" {
          let mut func_buffer = String::new();
          let result = Ok(self.eval_lambda(parsed, &mut func_buffer)?);
          self.sect.text.push_str(&func_buffer);
          result
        } else if self.f_table.contains_key(cmd.as_str()) {
          let evaluated;
          {
            let func = self.f_table.get_mut(cmd.as_str()).ok_or("Unreachable (eval)")?;
            evaluated = func.evaluated;
          };
          let args = if evaluated {
            Cow::Owned(self.eval_args(list.get(1..).unwrap_or(&[]), function)?)
          } else {
            Cow::Borrowed(list.get(1..).unwrap_or(&[]))
          };
          let func = self.f_table.get_mut(cmd.as_str()).ok_or("Unreachable (eval)")?;
          Ok(obj_json((func.func)(self, first, &args, function)?, first))
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
          return self.err_compile("Unreachable (eval)", &lambda);
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
    &mut self, args: &[Json], function: &mut String,
  ) -> Result<Vec<Json>, Box<dyn Error>> {
    let mut result = vec![];
    for arg in args {
      result.push(self.eval(arg, function)?);
    }
    Ok(result)
  }
  /// Evaluates a lambda function definition.
  fn eval_lambda(&mut self, func: &Json, function: &mut String) -> JResult {
    let JValue::Array(ref func_list) = func.value else {
      return self.err_compile("Invalid function definition.", func);
    };
    self.assert(func_list.len() >= 3, "Invalid function definition.", func)?;
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
    let mut ret = JValue::Null;
    for arg in func_list.get(2..).ok_or("Empty lambda body.")? {
      ret = self.eval(arg, function)?.value;
    }
    writeln!(
      function,
      "  {}
  add rsp, 32
  leave
  ret
  .seh_endproc",
      match ret {
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
    Ok(obj_json(JValue::FuncVar { name: n, params: params.clone(), ret: Box::new(ret) }, lambda))
  }
  /// Evaluates a 'begin' block.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_begin(&mut self, first: &Json, args: &[Json], _: &mut String) -> JFuncResult {
    let Some(last) = args.last() else {
      return self.err_func("'begin' requires at least one argument", first);
    };
    Ok(last.value.clone())
  }
  /// Utility functions for binary operations
  fn f_binary_op(
    &mut self, first: &Json, args: &[Json], function: &mut String, mnemonic: &str, op: &str,
  ) -> JFuncResult {
    self.assert(args.len() >= 2, &format!("'{op}' requires at least two arguments"), first)?;
    let augend = args.first().ok_or("Unreachable (binary_op)")?;
    match augend.value {
      JValue::Int(int) => writeln!(function, "  mov rax, {int}")?,
      JValue::IntVar(ref iv) => writeln!(function, "  mov rax, qword ptr {iv}[rip]")?,
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
        return self.err_func(&format!("'{op}' requires integer operands"), augend);
      }
    }
    for addend in args.get(1..).ok_or("Unreachable (binary_op)")? {
      match addend.value {
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
          return self.err_func(&format!("'{op}' requires integer operands"), addend);
        }
      }
    }
    let ret = self.get_name()?;
    writeln!(self.sect.bss, "  .lcomm {ret}, 8")?;
    writeln!(function, "  mov qword ptr {ret}[rip], rax")?;
    Ok(JValue::IntVar(ret))
  }
  /// Gets the value of a local variable.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_local_get(&mut self, first: &Json, args: &[Json], _: &mut String) -> JFuncResult {
    let Some(var) = args.first() else {
      return self.err_func("'$' requires one argument.", first);
    };
    let JValue::String(ref var_name) = var.value else {
      return self.err_func("Variable name requires compile-time fixed string.", var);
    };
    match self.vars.get(var_name) {
      Some(value) => Ok(value.clone()),
      None => self.err_func(&format!("Undefined variables: '{var_name}'"), var),
    }
  }
  /// Sets a local variable.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_local_set(&mut self, first: &Json, args: &[Json], function: &mut String) -> JFuncResult {
    self.assert(args.len() == 2, "'=' requires two arguments.", first)?;
    let JValue::String(ref var_name) = args.first().ok_or("Unreachable (f_set_local)")?.value
    else {
      return self.err_func(
        "Variable name requires compile-time fixed strings.",
        args.first().ok_or("Unreachable (f_set_local)")?,
      );
    };
    let result = self.eval(args.get(1).ok_or("Unreachable (f_set_local)")?, function)?;
    match result.value {
      JValue::String(ref st) => {
        let n = self.get_name()?;
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
      | JValue::ObjectVar(_) => self.err_func(
        "Assignment to an unimplemented type.",
        args.get(1).ok_or("Unreachable (f_set_local)")?,
      ),
    }
  }
  /// Displays a message box.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_message(&mut self, first: &Json, args: &[Json], function: &mut String) -> JFuncResult {
    self.assert(args.len() == 2, "'message' requires two arguments.", first)?;
    let title = match args.first().ok_or("Unreachable (f_message)")?.value {
      JValue::String(ref st) => {
        let name = self.get_name()?;
        writeln!(self.sect.data, "  {name}: .string \"{st}\"")?;
        name
      }
      JValue::StringVar(ref var) => var.clone(),
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
        return self.err_func(
          "The first argument of message must be a string.",
          args.first().ok_or("Unreachable (f_message)")?,
        );
      }
    };
    let msg = match args.get(1).ok_or("Unreachable (f_message)")?.value {
      JValue::String(ref st) => {
        let name = self.get_name()?;
        writeln!(self.sect.data, "  {name}: .string \"{st}\"")?;
        name
      }
      JValue::StringVar(ref var) => var.clone(),
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
        return self.err_func(
          "The second argument of message require a string.",
          args.get(1).ok_or("Unreachable (f_message)")?,
        );
      }
    };
    let wtitle = self.get_name()?;
    let wmsg = self.get_name()?;
    let ret = self.get_name()?;
    for data in [&wtitle, &wmsg, &ret] {
      writeln!(self.sect.bss, "  .lcomm {data}, 8")?;
    }
    write!(
      function,
      include_str!("asm/message.s"),
      msg, wmsg, title, wtitle, wmsg, wtitle, ret, wmsg, wtitle
    )?;
    Ok(JValue::IntVar(ret))
  }
  /// Performs subtraction.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_minus(&mut self, first: &Json, args: &[Json], function: &mut String) -> JFuncResult {
    self.f_binary_op(first, args, function, "sub", "-")
  }
  /// Performs addition.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_plus(&mut self, first: &Json, args: &[Json], function: &mut String) -> JFuncResult {
    self.f_binary_op(first, args, function, "add", "+")
  }
  /// Generates a unique name for internal use.
  fn get_name(&mut self) -> Result<String, &str> {
    let Some(res) = self.seed.checked_add(1) else { return Err("SeedOverflowError") };
    self.seed = res;
    Ok(format!(".LC{:x}", self.seed))
  }
  /// Registers a function in the function table.
  fn register(&mut self, name: &str, ev: bool, fu: JFunc<Self>) {
    self.f_table.insert(name.into(), BuiltinFunc { evaluated: ev, func: fu });
  }
  /// Writes the compiled assembly code to a file.
  fn write_file(&self, start: &str, filename: &str, json_file: &str) -> io::Result<()> {
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(format!(".file \"{json_file}\"\n.intel_syntax noprefix\n").as_bytes())?;
    writer.write_all(include_bytes!("asm/data.s"))?;
    writer.write_all(self.sect.data.as_bytes())?;
    writer.write_all(include_bytes!("asm/bss.s"))?;
    writer.write_all(self.sect.bss.as_bytes())?;
    writer.write_all(include_bytes!("asm/start.s"))?;
    writer.write_all(start.as_bytes())?;
    writer.write_all(include_bytes!("asm/text.s"))?;
    writer.write_all(self.sect.text.as_bytes())?;
    writer.flush()?;
    Ok(())
  }
}
