//! Parser implementation.
use super::{
  JFunc, JFuncResult, JResult, JValue, Jsompiler, Json,
  utility::{format_err, obj_json},
};
use core::{error::Error, fmt::Write as _};
use std::{
  fs::File,
  io::{self, BufWriter, Write as _},
};
impl Jsompiler<'_> {
  /// create compile error.
  fn compile_err(&self, text: &str, obj: &Json) -> JResult {
    Err(format_err(text, obj.pos, obj.ln, self.input_code).into())
  }
  /// create function error.
  fn func_err(&self, text: &str, obj: &Json) -> JFuncResult {
    Err(format_err(text, obj.pos, obj.ln, self.input_code).into())
  }
  /// Generates a unique name for internal use.
  fn get_name(&mut self) -> String {
    self.seed += 1;
    format!(".LC{:x}", self.seed)
  }
  /// Registers a function in the function table.
  fn register(&mut self, name: &str, func: JFunc<Self>) {
    self.f_table.insert(name.into(), func);
  }
  /// Writes the compiled assembly code to a file.
  fn write_file(&self, main_func: &str, filename: &str, json_file: &str) -> io::Result<()> {
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(format!(".file \"{json_file}\"\n").as_bytes())?;
    writer.write_all(include_bytes!("data.s"))?;
    writer.write_all(self.data.as_bytes())?;
    writer.write_all(include_bytes!("bss.s"))?;
    writer.write_all(self.bss.as_bytes())?;
    writer.write_all(include_bytes!("start.s"))?;
    writer.write_all(main_func.as_bytes())?;
    writer.write_all(include_bytes!("text.s"))?;
    writer.write_all(self.text.as_bytes())?;
    writer.flush()?;
    Ok(())
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
  /// * `Err(JError)` - If an error occurred during the compilation process.
  ///
  /// # Errors
  ///
  /// * `JError` - If an error occurred during the compilation process.
  pub fn build(&mut self, parsed: &Json, json_file: &str, filename: &str) -> JResult {
    self.seed = 0;
    self.register("=", Jsompiler::set_local);
    self.register("$", Jsompiler::get_local);
    self.register("+", Jsompiler::plus);
    self.register("-", Jsompiler::minus);
    self.register("message", Jsompiler::message);
    self.register("begin", Jsompiler::begin);
    let mut main_func = String::new();
    let result = self.eval(parsed, &mut main_func)?;
    self.write_file(&main_func, filename, json_file)?;
    Ok(result)
  }
  /// Evaluates a JSON object.
  fn eval(&mut self, parsed: &Json, function: &mut String) -> JResult {
    let JValue::Array(list) = &parsed.value else {
      return Ok(parsed.clone());
    };
    if list.is_empty() {
      return self
        .compile_err("An function call was expected, but an empty list was provided.", parsed);
    }
    match &list[0].value {
      JValue::String(cmd) => {
        if cmd == "lambda" {
          let mut func_buffer = String::new();
          let result = Ok(self.eval_lambda(list, &mut func_buffer)?);
          self.text.push_str(&func_buffer);
          return result;
        }
        if let Some(func) = self.f_table.get(cmd.as_str()) {
          Ok(obj_json(func(self, list, function)?, &list[0]))
        } else {
          self.compile_err(&format!("Function {cmd} is undefined."), &list[0])
        }
      }
      JValue::Array(func_list) => {
        let mut func_buffer = String::new();
        let tmp = self.vars.clone();
        let lambda = self.eval_lambda(func_list, &mut func_buffer)?;
        let JValue::FuncVar(name, _params) = lambda.value else {
          return self.compile_err("InternalError: 'lambda' don't return lambda object.", &lambda);
        };
        self.text.push_str(&func_buffer);
        writeln!(function, "  call {name}")?;
        self.vars = tmp;
        Ok(Json::default())
      }
      _ => self.compile_err(
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
  fn eval_lambda(&mut self, func_list: &[Json], function: &mut String) -> JResult {
    if !matches!(&func_list[0].value, JValue::String(s) if s == "lambda") {
      self.compile_err("The first element of a lambda list requires \"lambda\".", &func_list[0])?;
    }
    if func_list.len() < 3 {
      self.compile_err("Invalid function definition", &func_list[0])?;
    }
    let JValue::Array(params) = &func_list[1].value else {
      return self.compile_err(
        "The second element of a lambda list requires an argument list.",
        &func_list[1],
      );
    };
    if !params.is_empty() {
      todo!("TODO!")
    }
    let n = self.get_name();
    writeln!(
      function,
      ".section .text${},\"x\"
.seh_proc	{n}
{n}:
  push rbp
  .seh_pushreg rbp
  mov rbp, rsp
	.seh_setframe	rbp, 0
  sub rsp, 32
	.seh_stackalloc	32
	.seh_endprologue
  .seh_handler .L_SEH_HANDLER, @except",
      &n[3..]
    )?;
    for i in &func_list[2..] {
      self.eval(i, function)?;
    }
    writeln!(
      function,
      "  add rsp, 32
  leave
  ret
  .seh_endproc",
    )?;
    Ok(obj_json(JValue::FuncVar(n, params.clone()), &func_list[0]))
  }
  /// Evaluates a 'begin' block.
  fn begin(&mut self, args: &[Json], function: &mut String) -> JFuncResult {
    if args.len() <= 1 {
      return self.func_err("'begin' requires at least one arguments", &args[0]);
    }
    Ok(self.eval_args(&args[1..], function)?[0].clone())
  }
  /// Sets a local variable.
  fn set_local(&mut self, args: &[Json], function: &mut String) -> JFuncResult {
    if args.len() != 3 {
      return self.func_err("'=' requires two arguments", &args[0]);
    }
    let JValue::String(var_name) = &args[1].value else {
      return self.func_err("Variable name requires compile-time fixed strings", &args[1]);
    };
    let result = self.eval(&args[2], function)?;
    let n = self.get_name();
    match &result.value {
      JValue::String(s) => {
        writeln!(self.data, "  {n}: .string \"{s}\"")?;
        self.vars.insert(var_name.clone(), JValue::StringVar(n.clone()));
        Ok(JValue::StringVar(n))
      }
      JValue::StringVar(s) => {
        self.vars.insert(var_name.clone(), JValue::StringVar(s.clone()));
        Ok(result.value)
      }
      _ => Err("Assignment to an unimplemented type".into()),
    }
  }
  /// Gets the value of a local variable.
  fn get_local(&mut self, args: &[Json], _: &mut String) -> JFuncResult {
    if args.len() != 2 {
      return self.func_err("'$' requires one argument", &args[0]);
    }
    let JValue::String(var_name) = &args[1].value else {
      return self.func_err("Variable name requires compile-time fixed string", &args[1]);
    };
    if let Some(value) = self.vars.get(var_name) {
      Ok(value.clone())
    } else {
      self.func_err(&format!("Undefined variables: '{var_name}'"), &args[1])
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
    if args.len() <= 2 {
      return self
        .func_err(&format!("'{error_context}' requires at least two arguments"), &args[0]);
    }
    let result_vec = self.eval_args(&args[1..], function)?;
    match &result_vec[0] {
      JValue::Int(l) => writeln!(function, "  mov rax, {l}")?,
      JValue::IntVar(v) => writeln!(function, "  mov rax, qword ptr {v}[rip]")?,
      _ => return Err(format!("'{error_context}' requires integer operands").into()),
    }
    for a in &result_vec[1..] {
      match a {
        JValue::Int(l) => writeln!(function, "  {mnemonic} rax, {l}")?,
        JValue::IntVar(v) => writeln!(function, "  {mnemonic} rax, qword ptr {v}[rip]")?,
        _ => return Err(format!("'{error_context}' requires integer operands").into()),
      }
    }
    let ret = self.get_name();
    writeln!(self.bss, "  .lcomm {ret}, 8")?;
    writeln!(function, "  mov qword ptr {ret}[rip], rax")?;
    Ok(JValue::IntVar(ret))
  }
  /// Performs addition.
  fn plus(&mut self, args: &[Json], function: &mut String) -> JFuncResult {
    self.binary_op(args, function, "add", "+")
  }
  /// Performs subtraction.
  fn minus(&mut self, args: &[Json], function: &mut String) -> JFuncResult {
    self.binary_op(args, function, "sub", "-")
  }
  /// Displays a message box.
  fn message(&mut self, args: &[Json], function: &mut String) -> JFuncResult {
    if args.len() != 3 {
      return self.func_err("'message' requires two arguments", &args[0]);
    }
    let title = match self.eval(&args[1], function)?.value {
      JValue::String(l) => {
        let name = self.get_name();
        writeln!(self.data, "  {name}: .string \"{l}\"")?;
        name
      }
      JValue::StringVar(var) => var,
      _ => return self.func_err("The first argument of message must be a string", &args[1]),
    };
    let msg = match self.eval(&args[2], function)?.value {
      JValue::String(l) => {
        let name = self.get_name();
        writeln!(self.data, "  {name}: .string \"{l}\"")?;
        name
      }
      JValue::StringVar(var) => var,
      _ => return self.func_err("The second argument of message must be a string", &args[2]),
    };
    let wtitle = self.get_name();
    let wmsg = self.get_name();
    let ret = self.get_name();
    writeln!(
      self.bss,
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
}
