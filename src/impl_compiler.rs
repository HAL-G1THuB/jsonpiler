use super::utility::{dummy, en64};
use super::{JFunc, JResult, JValue, Jsompiler, Json};
use std::fmt::Write as _;
use std::fs::File;
use std::io::{self, BufWriter, Write as _};
impl Jsompiler<'_> {
  fn get_name(&mut self) -> String {
    self.seed += 1;
    format!("_{:x}", self.seed)
  }
  fn assert(&self, flag: bool, text: &str, obj: &Json) -> JResult {
    if !flag {
      self.obj_err(text, obj)
    } else {
      dummy()
    }
  }
  fn entry(&mut self, name: &str, func: JFunc<Self>) {
    self.f_table.insert(name.into(), func);
  }
  fn write_file(&mut self, main_func: &str, filename: &str, json_file: &str) -> io::Result<()> {
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(b".file \"")?;
    writer.write_all(json_file.as_bytes())?;
    writer.write_all(
      br#""
.intel_syntax noprefix
.globl _start
.data
"#,
    )?;
    writer.write_all(self.data.as_bytes())?;
    writer.write_all(
      br#".bss
  .lcomm EMSG, 512
  .lcomm STDO, 8
  .lcomm STDE, 8
  .lcomm STDI, 8
"#,
    )?;
    writer.write_all(self.bss.as_bytes())?;
    writer.write_all(
      br#".text
_start:
  push rbp
  mov rbp, rsp
  sub rsp, 32
  mov ecx, 65001
  call SetConsoleCP
  test rax, rax
  jz display_error
  mov ecx, 65001
  call SetConsoleOutputCP
  test rax, rax
  jz display_error
  mov ecx, -10
  call GetStdHandle
  cmp rax, -1
  je display_error
  mov QWORD PTR STDI[rip], rax
  mov ecx, -11
  call GetStdHandle
  cmp rax, -1
  je display_error
  mov QWORD PTR STDO[rip], rax
  mov ecx, -12
  call GetStdHandle
  cmp rax, -1
  je display_error
  mov QWORD PTR STDE[rip], rax
"#,
    )?;
    writer.write_all(main_func.as_bytes())?;
    writer.write_all(
      br#"  xor ecx, ecx
  call ExitProcess
  display_error:
  call GetLastError
  mov rbx, rax
  sub rsp, 32
  mov ecx, 0x1200
  xor edx, edx
  mov r8, rbx
  xor r9d, r9d
  lea rax, QWORD PTR EMSG[rip]
  mov 32[rsp], rax
  mov QWORD PTR 40[rsp], 512
  mov QWORD PTR 48[rsp], 0
  call FormatMessageW
  add rsp, 32
  test rax, rax
  jz exit_program
  xor ecx, ecx
  lea rdx, QWORD PTR EMSG[rip]
  xor r8d, r8d
  mov r9, 0x10
  call MessageBoxW
  exit_program:
  mov rcx, rbx
  call ExitProcess
"#,
    )?;
    writer.write_all(self.text.as_bytes())?;
    writer.flush()?;
    Ok(())
  }
  pub fn build(&mut self, parsed: Json, json_file: &str, filename: &str) -> JResult {
    self.seed = 0;
    self.entry("=", Jsompiler::set_local);
    self.entry("$", Jsompiler::get_local);
    self.entry("+", Jsompiler::plus);
    self.entry("-", Jsompiler::minus);
    self.entry("message", Jsompiler::message);
    self.entry("begin", Jsompiler::begin);
    let mut main_func = String::new();
    let result = self.eval(&parsed, &mut main_func)?;
    self.write_file(&main_func, filename, json_file)?;
    Ok(result)
  }
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
          func(self, &list[1..], function)
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
        dummy()
      }
      _ => self.obj_err(
        "The first element of an evaluation list requires a function name.",
        parsed,
      ),
    }
  }
  fn eval_lambda(&mut self, func_list: &[Json], function: &mut String) -> JResult {
    if !matches!(func_list[0].value, JValue::String(ref s) if s == "lambda") {
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
      r#"{n}:
  push rbp
  mov rbp, rsp
  sub rsp, 32"#
    )?;
    for i in &func_list[2..] {
      self.eval(i, function)?;
    }
    writeln!(
      function,
      r#"  add rsp, 32
  mov rsp, rbp
  pop rbp
  ret"#,
    )?;
    Ok(self.obj_json(JValue::FuncVar(n, params.clone()), &func_list[0]))
  }
  fn begin(&mut self, args: &[Json], function: &mut String) -> JResult {
    self.assert(
      !args.is_empty(),
      "'begin' requires at least one arguments",
      &args[0],
    )?;
    let mut result = dummy()?;
    for a in args {
      result = self.eval(a, function)?
    }
    Ok(result)
  }
  fn set_local(&mut self, args: &[Json], function: &mut String) -> JResult {
    self.assert(args.len() == 2, "'=' requires at two arguments", &args[0])?;
    let JValue::String(var_name) = &args[0].value else {
      return self.obj_err(
        "Variable name requires compile-time fixed strings",
        &args[1],
      );
    };
    let result = self.eval(&args[1], function)?;
    let n = format!("\"{}\"", en64(var_name.as_bytes()));
    match &result.value {
      JValue::String(s) => {
        writeln!(self.data, "  {n}: .string \"{s}\"")?;
        self.vars.insert(var_name.clone(), JValue::StringVar(n));
        Ok(result)
      }
      JValue::StringVar(s) => {
        writeln!(self.data, "{n}: equ {s}")?;
        self.vars.insert(var_name.clone(), JValue::StringVar(n));
        Ok(result)
      }
      _ => self.obj_err("Assignment to an unimplemented type", &args[2]),
    }
  }
  fn get_local(&mut self, args: &[Json], _: &mut String) -> JResult {
    self.assert(args.len() == 1, "'$' requires one argument", &args[0])?;
    let JValue::String(var_name) = &args[0].value else {
      return self.obj_err("Variable name requires compile-time fixed string", &args[1]);
    };
    if let Some(value) = self.vars.get(var_name) {
      Ok(self.obj_json(value.clone(), &args[0]))
    } else {
      self.obj_err(&format!("Undefined variables: '{var_name}'"), &args[1])
    }
  }
  fn plus(&mut self, args: &[Json], function: &mut String) -> JResult {
    self.assert(
      !args.is_empty(),
      "'+' requires at least one arguments",
      &args[0],
    )?;
    match self.eval(&args[0], function)?.value {
      JValue::Int(l) => writeln!(function, "  mov rax, {l}")?,
      JValue::IntVar(v) => writeln!(function, "  mov rax, QWORD PTR {v}[rip]")?,
      _ => return self.obj_err("'+' requires integer operands", &args[0]),
    }
    for a in &args[1..args.len()] {
      match self.eval(a, function)?.value {
        JValue::Int(l) => writeln!(function, "  add rax, {l}")?,
        JValue::IntVar(v) => writeln!(function, "  add rax, QWORD PTR {v}[rip]")?,
        _ => {
          return self.obj_err("'+' requires integer operands", &args[0]);
        }
      };
    }
    let ret = self.get_name();
    writeln!(self.bss, "  .lcomm {ret}, 8")?;
    writeln!(function, "  mov QWORD PTR {ret}[rip], rax")?;
    Ok(Json {
      pos: args[0].pos,
      ln: args[0].ln,
      value: JValue::IntVar(ret),
    })
  }
  fn minus(&mut self, args: &[Json], function: &mut String) -> JResult {
    self.assert(
      !args.is_empty(),
      "'-' requires at least one arguments",
      &args[0],
    )?;
    match self.eval(&args[0], function)?.value {
      JValue::Int(l) => writeln!(function, "  mov rax, {l}")?,
      JValue::IntVar(v) => writeln!(function, "  mov rax, QWORD PTR {v}[rip]")?,
      _ => return self.obj_err("'-' requires integer operands", &args[0]),
    }
    for a in &args[2..args.len()] {
      match self.eval(a, function)?.value {
        JValue::Int(l) => writeln!(function, "  sub rax, {l}")?,
        JValue::IntVar(v) => writeln!(function, "  sub rax, QWORD PTR {v}[rip]")?,
        _ => {
          return self.obj_err("'+' requires integer operands", &args[0]);
        }
      };
    }
    let ret = self.get_name();
    writeln!(self.bss, "  .lcomm {ret}, 8")?;
    writeln!(function, "  mov QWORD PTR {ret}[rip], rax")?;
    Ok(Json {
      pos: args[0].pos,
      ln: args[0].ln,
      value: JValue::IntVar(ret),
    })
  }
  fn message(&mut self, args: &[Json], function: &mut String) -> JResult {
    self.assert(
      args.len() == 2,
      "'message' requires two arguments",
      &args[0],
    )?;
    let arg1 = self.eval(&args[0], function)?.value;
    let title = match arg1 {
      JValue::String(l) => {
        let name = self.get_name();
        writeln!(self.data, "  {name}: .string \"{l}\"")?;
        name
      }
      JValue::StringVar(v) => v,
      _ => {
        return self.obj_err("The first argument of message must be a string", &args[0]);
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
        return self.obj_err("The second argument of message must be a string", &args[1]);
      }
    };
    let ret = self.get_name();
    writeln!(self.bss, "  .lcomm {ret}, 8")?;
    writeln!(
      function,
      r#"  xor ecx, ecx
  lea rdx, QWORD PTR {msg}[rip]
  lea r8, QWORD PTR {title}[rip]
  xor r9d, r9d
  call MessageBoxA
  test eax, eax
  jz display_error
  mov QWORD PTR {ret}[rip], rax"#,
    )?;
    Ok(self.obj_json(JValue::IntVar(ret), &args[0]))
  }
}
