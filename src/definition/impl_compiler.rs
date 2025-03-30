use super::super::utility::dummy;
use super::{F, Jsompiler, JResult, JValue, Json, VKind};
use std::fmt::Write as _;
use std::fs::File;
use std::io::Write as _;
impl Jsompiler<'_> {
  fn get_name(&mut self) -> String {
    self.seed += 1;
    format!("_{:x}", self.seed)
  }
  fn validate(&self, flag: bool, name: &str, text: &str, obj: &Json) -> JResult {
    if flag {
      self.obj_err(&format!("\"{name}\" requires {text} argument"), obj)
    } else {
      dummy()
    }
  }
  pub fn build(&mut self, parsed: Json, filename: &str) -> JResult {
    self.seed = 0;
    self.f_table.insert("=".into(), Jsompiler::set_var as F<Self>);
    self.f_table.insert("$".into(), Jsompiler::get_var as F<Self>);
    self.f_table.insert("+".into(), Jsompiler::plus as F<Self>);
    self.f_table.insert("-".into(), Jsompiler::minus as F<Self>);
    self
      .f_table
      .insert("message".into(), Jsompiler::message as F<Self>);
    self
      .f_table
      .insert("begin".into(), Jsompiler::begin as F<Self>);
    self.data.push_str(".data\n");
    self.bss.push_str(
      r#".bss
  .lcomm errorMessage, 512
  .lcomm STDOUT, 8
  .lcomm STDERR, 8
  .lcomm STDIN, 8
"#,
    );
    self.text.push_str(".text\n");
    let mut main_func = String::from(
      r#"_start:
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
  mov QWORD PTR [rip + STDIN], rax
  mov ecx, -11
  call GetStdHandle
  cmp rax, -1
  je display_error
  mov QWORD PTR [rip + STDOUT], rax
  mov ecx, -12
  call GetStdHandle
  cmp rax, -1
  je display_error
  mov QWORD PTR [rip + STDERR], rax
"#,
    );
    let result = self.eval(&parsed, &mut main_func)?;
    let mut file = File::create(filename)?;
    writeln!(file, ".intel_syntax noprefix\n.globl _start")?;
    write!(file, "{}", self.data)?;
    write!(file, "{}", self.bss)?;
    write!(file, "{}", self.text)?;
    write!(file, "{main_func}")?;
    writeln!(
      file,
      r#"  xor ecx, ecx
  call ExitProcess
display_error:
  call GetLastError
  mov rbx, rax
  sub rsp, 32
  mov ecx, 0x1200
  xor edx, edx
  mov r8, rbx
  xor r9d, r9d
  lea rax, QWORD PTR [rip + errorMessage]
  mov [rsp + 32], rax
  mov qword ptr [rsp + 40], 512
  mov qword ptr [rsp + 48], 0
  call FormatMessageW
  add rsp, 32
  test rax, rax
  jz exit_program
  xor ecx, ecx
  lea rdx, QWORD PTR [rip + errorMessage]
  xor r8d, r8d
  mov r9, 0x10
  call MessageBoxW
exit_program:
  mov rcx, rbx
  call ExitProcess"#
    )?;
    Ok(result)
  }
  fn eval(&mut self, parsed: &Json, function: &mut String) -> JResult {
    let JValue::Array(VKind::Lit(list)) = &parsed.value else {
      return Ok(parsed.clone());
    };
    if list.is_empty() {
      return self.obj_err(
        "An procedure was expected, but an empty list was provided",
        parsed,
      );
    };
    match &list[0].value {
      JValue::String(VKind::Lit(cmd)) => {
        if cmd == "lambda" {
          return Ok(parsed.clone());
        }
        if let Some(func) = self.f_table.get(cmd.as_str()) {
          func(self, list.as_slice(), function)
        } else {
          self.obj_err(&format!("Undefined function: {cmd}"), &list[0])
        }
      }
      _ => {
        let mut func_buffer = String::new();
        let func_value = self.eval_lambda(parsed, &mut func_buffer)?;
        self.text.push_str(&func_buffer);
        Ok(func_value)
      }
    }
  }
  fn eval_lambda(&mut self, parsed: &Json, function: &mut String) -> JResult {
    let JValue::Array(VKind::Lit(func_list)) = &parsed.value else {
      return self.obj_err(
        "Only a lambda list or a string is allowed as the first element of a list",
        parsed,
      );
    };
    let JValue::String(VKind::Lit(cmd)) = &func_list[0].value else {
      return self.obj_err(
        "Only a lambda list or a string is allowed as the first element of a list",
        parsed,
      );
    };
    if cmd != "lambda" {
      return self.obj_err(
        "Only a lambda list or a string is allowed as the first element of a list",
        parsed,
      );
    }
    if func_list.len() < 3 {
      return self.obj_err("Invalid function definition", parsed);
    };
    let JValue::Array(VKind::Lit(params)) = &func_list[1].value else {
      return self.obj_err(
        "The second element of a lambda list must be an argument list",
        &func_list[1],
      );
    };
    for i in &func_list[2..] {
      self.eval(i, function)?;
    }
    Ok(self.obj_json(JValue::Function(VKind::Lit(params.clone())), &func_list[0]))
  }
  fn begin(&mut self, args: &[Json], function: &mut String) -> JResult {
    self.validate(args.len() == 1, "begin", "at least one", &args[0])?;
    let mut result = Json {
      pos: 0,
      ln: 0,
      value: JValue::Null,
    };
    for a in &args[1..] {
      a.print_json()?;
      result = self.eval(a, function)?
    }
    Ok(result)
  }
  fn set_var(&mut self, args: &[Json], function: &mut String) -> JResult {
    self.validate(args.len() != 3, "=", "two", &args[0])?;
    if let JValue::String(VKind::Lit(var_name)) = &args[1].value {
      let result = self.eval(&args[2], function)?;
      if !result.value.is_lit() {
        self.vars.insert(var_name.clone(), result.clone());
        return Ok(result);
      }
      match result.value {
        JValue::String(VKind::Lit(s)) => {
          let n = self.get_name();
          writeln!(self.data, "  {n}: .string \"{s}\"")?;
          self.vars.insert(
            var_name.clone(),
            self.obj_json(JValue::String(VKind::Var(n.clone())), &args[0]),
          );
          Ok(self.obj_json(JValue::String(VKind::Var(n)), &args[0]))
        }
        _ => self.obj_err("Assignment to an unimplemented type", &args[2]),
      }
    } else {
      self.obj_err(
        "Variable names must be compile-time fixed strings",
        &args[0],
      )
    }
  }
  fn get_var(&mut self, args: &[Json], _: &mut String) -> JResult {
    self.validate(args.len() != 2, "$", "one", &args[0])?;
    if let JValue::String(VKind::Lit(var_name)) = &args[1].value {
      if let Some(value) = self.vars.get(var_name) {
        Ok(value.clone())
      } else {
        self.obj_err(&format!("Undefined variables: '{}'", var_name), &args[1])
      }
    } else {
      self.obj_err(
        "Variable names must be compile-time fixed strings",
        &args[0],
      )
    }
  }
  fn plus(&mut self, args: &[Json], function: &mut String) -> JResult {
    self.validate(args.len() == 1, "+", "at least one", &args[0])?;
    let Ok(Json {
      pos: _,
      ln: _,
      value: JValue::Int(result),
    }) = self.eval(&args[1], function)
    else {
      return self.obj_err("'+' requires integer operands", &args[0]);
    };
    match result {
      VKind::Lit(l) => writeln!(function, "  mov rax, {l}")?,
      VKind::Var(v) => writeln!(function, "  mov rax, QWORD PTR [rip + {v}]")?,
    }
    for a in &args[2..args.len()] {
      match self.eval(a, function)?.value {
        JValue::Int(VKind::Lit(l)) => writeln!(function, "  add rax, {l}")?,
        JValue::Int(VKind::Var(v)) => writeln!(function, "  add rax, QWORD PTR [rip + {v}]")?,
        _ => {
          return self.obj_err("'+' requires integer operands", &args[0]);
        }
      };
    }
    let ret = self.get_name();
    writeln!(self.bss, "  .lcomm {ret}, 8")?;
    writeln!(function, "  mov QWORD PTR [rip + {ret}], rax")?;
    Ok(Json {
      pos: args[0].pos,
      ln: args[0].ln,
      value: JValue::Int(VKind::Var(ret)),
    })
  }
  fn minus(&mut self, args: &[Json], function: &mut String) -> JResult {
    self.validate(args.len() == 1, "-", "at least one", &args[0])?;
    let JValue::Int(result) = self.eval(&args[1], function)?.value else {
      return self.obj_err("'-' requires integer operands", &args[0]);
    };
    match result {
      VKind::Lit(l) => writeln!(function, "  mov rax, {l}")?,
      VKind::Var(v) => writeln!(function, "  mov rax, QWORD PTR [rip + {v}]")?,
    }
    for a in &args[2..args.len()] {
      let JValue::Int(result) = self.eval(a, function)?.value else {
        return self.obj_err("'-' requires integer operands", &args[0]);
      };
      match result {
        VKind::Lit(l) => writeln!(function, "  sub rax, {l}")?,
        VKind::Var(v) => writeln!(function, "  sub rax, QWORD PTR [rip + {v}]")?,
      }
    }
    let ret = self.get_name();
    writeln!(self.bss, "  .lcomm {ret}, 8")?;
    writeln!(function, "  mov QWORD PTR [rip + {ret}], rax")?;
    Ok(Json {
      pos: args[0].pos,
      ln: args[0].ln,
      value: JValue::Int(VKind::Var(ret)),
    })
  }
  fn message(&mut self, args: &[Json], function: &mut String) -> JResult {
    self.validate(args.len() != 3, "message", "two", &args[0])?;
    let arg1 = self.eval(&args[1], function)?.value;
    let title = match arg1 {
      JValue::String(VKind::Lit(l)) => {
        let name = self.get_name();
        writeln!(self.data, "  {name}: .string \"{l}\"")?;
        name
      }
      JValue::String(VKind::Var(v)) => v,
      _ => {
        return self.obj_err("The first argument of message must be a string", &args[1]);
      }
    };
    let msg = match self.eval(&args[2], function)?.value {
      JValue::String(VKind::Lit(l)) => {
        let name = self.get_name();
        writeln!(self.data, "  {name}: .string \"{l}\"")?;
        name
      }
      JValue::String(VKind::Var(v)) => v,
      _ => {
        return self.obj_err("The second argument of message must be a string", &args[2]);
      }
    };
    let ret = self.get_name();
    writeln!(self.bss, "  .lcomm {ret}, 8")?;
    writeln!(
      function,
      r#"  xor ecx, ecx
  lea rdx, QWORD PTR [rip + {msg}]
  lea r8, QWORD PTR [rip + {title}]
  xor r9d, r9d
  call MessageBoxA
  test eax, eax
  jz display_error
  mov QWORD PTR [rip + {ret}], rax"#,
    )?;
    Ok(self.obj_json(JValue::Int(VKind::Var(ret)), &args[0]))
  }
}
