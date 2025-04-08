use super::utility::dummy;
use super::{JFunc, JFuncResult, JResult, JValue, Jsompiler, Json, utility::obj_json};
use std::fmt::Write as _;
use std::fs::File;
use std::io::{self, BufWriter, Write as _};
impl Jsompiler<'_> {
  /// Generates a unique name for internal use.
  fn get_name(&mut self) -> String {
    self.seed += 1;
    format!(".L{:x}", self.seed)
  }
  /// Asserts a condition and returns an error if the condition is false.
  fn assert(&self, flag: bool, text: &str, obj: &Json) -> JResult {
    if flag {
      Ok(dummy())
    } else {
      self.obj_err(text, obj)
    }
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
      return self.obj_err(
        "An function call was expected, but an empty list was provided",
        parsed,
      );
    };
    match &list[0].value {
      JValue::String(cmd) => {
        if cmd == "lambda" {
          let mut func_buffer = String::new();
          let result = Ok(self.eval_lambda(list, &mut func_buffer)?);
          self.text.push_str(&func_buffer);
          return result;
        }
        if let Some(func) = self.f_table.get(cmd.as_str()) {
          Ok(obj_json(func(self, &list[1..], function)?, &list[0]))
        } else {
          self.obj_err(&format!("Function {cmd} is undefined"), &list[0])
        }
      }
      JValue::Array(func_list) => {
        let mut func_buffer = String::new();
        let tmp = self.vars.clone();
        let JValue::FuncVar(name, _params) = self.eval_lambda(func_list, &mut func_buffer)?.value
        else {
          unreachable!()
        };
        self.text.push_str(&func_buffer);
        writeln!(function, "  call {name}")?;
        self.vars = tmp;
        Ok(dummy())
      }
      _ => self.obj_err(
        "The first element of an evaluation list requires a function name.",
        parsed,
      ),
    }
  }
  /// Evaluates a lambda function definition.
  fn eval_lambda(&mut self, func_list: &[Json], function: &mut String) -> JResult {
    if !matches!(&func_list[0].value, JValue::String(s) if s == "lambda") {
      return self.obj_err(
        "The first element of a lambda list requires \"lambda\".",
        &func_list[0],
      );
    }
    if func_list.len() < 3 {
      return self.obj_err("Invalid function definition", &func_list[0]);
    };
    let JValue::Array(params) = &func_list[1].value else {
      return self.obj_err(
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
      ".seh_proc	{n}
{n}:
  push rbp
  .seh_pushreg rbp
  mov rbp, rsp
	.seh_setframe	rbp, 0
  sub rsp, 32
	.seh_stackalloc	32
	.seh_endprologue
  .seh_handler exception_handler, @except"
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
    self.assert(
      !args.is_empty(),
      "'begin' requires at least one arguments",
      &args[0],
    )?;
    let mut result = dummy();
    for a in args {
      result = self.eval(a, function)?;
    }
    Ok(result.value)
  }
  /// Sets a local variable.
  fn set_local(&mut self, args: &[Json], function: &mut String) -> JFuncResult {
    self.assert(args.len() == 2, "'=' requires two arguments", &args[0])?;
    let JValue::String(var_name) = &args[0].value else {
      return Err("Variable name requires compile-time fixed strings".into());
    };
    let result = self.eval(&args[1], function)?;
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
    self.assert(args.len() == 1, "'$' requires one argument", &args[0])?;
    let JValue::String(var_name) = &args[0].value else {
      return Err("Variable name requires compile-time fixed string".into());
    };
    self.vars.get(var_name).map_or_else(
      || Err(format!("Undefined variables: '{var_name}'").into()),
      |value| Ok(value.clone()),
    )
  }
  /// Performs addition.
  fn plus(&mut self, args: &[Json], function: &mut String) -> JFuncResult {
    self.assert(
      !args.is_empty(),
      "'+' requires at least one arguments",
      &args[0],
    )?;
    match self.eval(&args[0], function)?.value {
      JValue::Int(l) => writeln!(function, "  mov rax, {l}")?,
      JValue::IntVar(v) => writeln!(function, "  mov rax, QWORD PTR {v}[rip]")?,
      _ => return Err("'+' requires integer operands".into()),
    }
    for a in &args[1..args.len()] {
      match self.eval(a, function)?.value {
        JValue::Int(l) => writeln!(function, "  add rax, {l}")?,
        JValue::IntVar(v) => writeln!(function, "  add rax, QWORD PTR {v}[rip]")?,
        _ => return Err("'+' requires integer operands".into()),
      };
    }
    let ret = self.get_name();
    writeln!(self.bss, "  .lcomm {ret}, 8")?;
    writeln!(function, "  mov QWORD PTR {ret}[rip], rax")?;
    Ok(JValue::IntVar(ret))
  }
  /// Performs subtraction.
  fn minus(&mut self, args: &[Json], function: &mut String) -> JFuncResult {
    self.assert(
      !args.is_empty(),
      "'-' requires at least one arguments",
      &args[0],
    )?;
    match self.eval(&args[0], function)?.value {
      JValue::Int(l) => writeln!(function, "  mov rax, {l}")?,
      JValue::IntVar(v) => writeln!(function, "  mov rax, QWORD PTR {v}[rip]")?,
      _ => return Err("'-' requires integer operands".into()),
    }
    for a in &args[1..args.len()] {
      match self.eval(a, function)?.value {
        JValue::Int(l) => writeln!(function, "  sub rax, {l}")?,
        JValue::IntVar(v) => writeln!(function, "  sub rax, QWORD PTR {v}[rip]")?,
        _ => {
          return Err("'+' requires integer operands".into());
        }
      };
    }
    let ret = self.get_name();
    writeln!(self.bss, "  .lcomm {ret}, 8")?;
    writeln!(function, "  mov QWORD PTR {ret}[rip], rax")?;
    Ok(JValue::IntVar(ret))
  }
  /// Displays a message box.
  fn message(&mut self, args: &[Json], function: &mut String) -> JFuncResult {
    self.assert(
      args.len() == 2,
      "'message' requires two arguments",
      &args[0],
    )?;
    let title = match self.eval(&args[0], function)?.value {
      JValue::String(l) => {
        let name = self.get_name();
        writeln!(self.data, "  {name}: .string \"{l}\"")?;
        name
      }
      JValue::StringVar(v) => v,
      _ => {
        return Err("The first argument of message must be a string".into());
      }
    };
    let msg = match self.eval(&args[1], function)?.value {
      JValue::String(l) => {
        let name = self.get_name();
        writeln!(self.data, "  {name}: .string \"{l}\"")?;
        name
      }
      JValue::StringVar(v) => v,
      _ => {
        return Err("The second argument of message must be a string".into());
      }
    };
    writeln!(function, "  sub rsp, 16")?;
    let wtitle = self.get_name();
    let wmsg = self.get_name();
    for (c, w) in [(&msg, &wmsg), (&title, &wtitle)] {
      writeln!(self.bss, "  .lcomm {w}, 8")?;
      writeln!(
        function,
        "  mov ecx, 65001
  xor edx, edx
  lea r8, QWORD PTR {c}[rip]
  mov r9d, -1
  mov QWORD PTR 0x20[rsp], 0
  mov QWORD PTR 0x28[rsp], 0
  call [QWORD PTR __imp_MultiByteToWideChar[rip]]
  test rax, rax
  jz display_error
  shl eax, 1
  mov edi, eax
  mov ecx, eax
  call malloc
  mov r12, rax
  mov ecx, 65001
  xor edx, edx
  lea r8, QWORD PTR {c}[rip]
  mov r9d, -1
  mov QWORD PTR 0x20[rsp], r12
  mov QWORD PTR 0x28[rsp], rdi
  call [QWORD PTR __imp_MultiByteToWideChar[rip]]
  test rax, rax
  jz display_error
  mov QWORD PTR {w}[rip], r12"
      )?;
    }
    let ret = self.get_name();
    writeln!(self.bss, "  .lcomm {ret}, 8")?;
    writeln!(
      function,
      "  xor ecx, ecx
  mov rdx, QWORD PTR {wmsg}[rip]
  mov r8, QWORD PTR {wtitle}[rip]
  xor r9d, r9d
  call [QWORD PTR __imp_MessageBoxW[rip]]
  test rax, rax
  jz display_error
  mov QWORD PTR {ret}[rip], rax
  mov rcx, QWORD PTR {wmsg}[rip]
  call free
  mov rcx, QWORD PTR {wtitle}[rip]
  call free
  add rsp, 16",
    )?;
    Ok(JValue::IntVar(ret))
  }
}
