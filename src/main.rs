use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt::{self, Write as _};
use std::fs::{self, File};
use std::io::{self, Write as _};
use std::path::Path;
use std::process::Command;
type JResult = Result<Json, Box<dyn Error>>;
type F<T> = fn(&mut T, &[Json], &mut String) -> JResult;
fn get_error_line(input_code: &str, index: usize) -> String {
  if input_code.is_empty() {
    return "Error: Empty input".into();
  }
  let len = input_code.len();
  let idx = index.min(len.saturating_sub(1));
  let start = if idx > 0 {
    input_code[..idx].rfind('\n').map_or(0, |pos| pos + 1)
  } else {
    0
  };
  let end = input_code[idx..].find('\n').map_or(len, |pos| idx + pos);
  let ws = " ".repeat(idx.saturating_sub(start));
  format!("Error position:\n{}\n{}^", &input_code[start..end], ws)
}
fn format_err (text:&str, pos:usize,ln: usize, input_code: &str) -> JResult {
    Err(
      format!(
        "{text}\nError occurred on line: {}\nError position:{}",
        ln + 1,
        get_error_line(input_code, pos)
      )
      .into(),
    )
  }
#[derive(Debug, Clone)]
struct Json {
  pub pos: usize,
  pub ln: usize,
  pub value: JValue,
}
#[derive(Debug, Clone)]
enum VKind<T> {
  Var(String),
  Lit(T),
}
#[derive(Debug, Clone)]
enum JValue {
  Null,
  Bool(VKind<bool>),
  Int(VKind<i64>),
  Float(VKind<f64>),
  String(VKind<String>),
  Array(VKind<Vec<Json>>),
  Object(VKind<HashMap<String, Json>>),
  Function(VKind<Vec<Json>>),
}
impl JValue {
  fn is_lit(&self) -> bool {
    match self {
      JValue::Null => true,
      JValue::Bool(v) => matches!(v, VKind::Lit(_)),
      JValue::Int(v) => matches!(v, VKind::Lit(_)),
      JValue::Float(v) => matches!(v, VKind::Lit(_)),
      JValue::String(v) => matches!(v, VKind::Lit(_)),
      JValue::Array(v) => matches!(v, VKind::Lit(_)),
      JValue::Object(v) => matches!(v, VKind::Lit(_)),
      JValue::Function(v) => matches!(v, VKind::Lit(_)),
    }
  }
}
#[derive(Default)]
struct JParser<'a> {
  input_code: &'a str,
  pos: usize,
  seed: usize,
  ln: usize,
  data: String,
  bss: String,
  text: String,
  f_table: HashMap<String, F<Self>>,
  vars: HashMap<String, Json>,
}
impl<'a> JParser<'a> {
  fn next(&mut self) -> Result<char, String> {
    let ch = self.input_code[self.pos..]
      .chars()
      .next()
      .ok_or("Reached end of text")?;
    self.pos += ch.len_utf8();
    Ok(ch)
  }
  fn expect(&mut self, expected: char) -> JResult {
    if self.input_code[self.pos..].starts_with(expected) {
      self.next()?;
      self.dummy()
    } else {
      self.parse_err(&format!("Expected character '{expected}' not found."))
    }
  }
  fn get_name(&mut self) -> String {
    self.seed += 1;
    format!("_{:x}", self.seed)
  }
  fn dummy(&self) -> JResult {
    Ok(Json {
      pos: 0,
      ln: 0,
      value: JValue::Null,
    })
  }
  fn validate(&self, flag: bool, name: &str, text: &str, obj: &Json) -> JResult {
    if flag {
      self.obj_err(&format!("\"{name}\" requires {text} argument"), obj)
    } else {
      self.dummy()
    }
  }
  fn parse_err(&self, text: &str) -> JResult {
    format_err(text, self.pos, self.ln, self.input_code)
  }
  fn obj_err(&self, text: &str, obj: &Json) -> JResult {
    format_err(text, obj.pos, obj.ln, self.input_code)
  }
  fn obj_json(&self, val: JValue, obj: &Json) -> Json {
    Json {
      pos: obj.pos,
      ln: obj.ln,
      value: val,
    }
  }
  fn parse(&mut self, code: &'a str) -> JResult {
    self.input_code = code;
    let result = self.parse_value()?;
    self.skip_ws();
    if self.pos != self.input_code.len() {
      self.parse_err("Unexpected trailing characters")
    } else {
      Ok(result)
    }
  }
  fn skip_ws(&mut self) {
    while self.pos < self.input_code.len() {
      let Some(c) = self.input_code[self.pos..].chars().next() else {
        break;
      };
      if c.is_whitespace() {
        if c == '\n' {
          self.ln += 1;
        }
        self.pos += c.len_utf8()
      } else {
        break;
      }
    }
  }
  fn parse_name(&mut self, n: &str, v: JValue) -> JResult {
    if self.input_code[self.pos..].starts_with(n) {
      let start = self.pos;
      self.pos += n.len();
      Ok(Json {
        pos: start,
        ln: self.ln,
        value: v,
      })
    } else {
      self.parse_err(&format!("Failed to parse '{n}'"))
    }
  }
  fn parse_number(&mut self) -> JResult {
    let start = self.pos;
    let mut num_str = String::new();
    let mut has_decimal = false;
    let mut has_exponent = false;
    if self.input_code[self.pos..].starts_with('-') {
      num_str.push('-');
      self.next()?;
    }
    if self.input_code[self.pos..].starts_with('0') {
      num_str.push('0');
      self.next()?;
      if matches!(self.input_code[self.pos..].chars().next(), Some(c) if c.is_ascii_digit()) {
        return self.parse_err("Leading zeros are not allowed in numbers");
      }
    } else if matches!(self.input_code[self.pos..].chars().next(), Some('1'..='9')) {
      while let Some(ch) = self.input_code[self.pos..].chars().next() {
        if ch.is_ascii_digit() {
          num_str.push(ch);
          self.next()?;
        } else {
          break;
        }
      }
    } else {
      return self.parse_err("Invalid number format");
    }
    if let Some(ch) = self.input_code[self.pos..].chars().next() {
      if ch == '.' {
        has_decimal = true;
        num_str.push(ch);
        self.next()?;
        if !matches!(self.input_code[self.pos..].chars().next(), Some(c) if c.is_ascii_digit()) {
          return self.parse_err("A digit is required after the decimal point");
        }
        while let Some(ch) = self.input_code[self.pos..].chars().next() {
          if ch.is_ascii_digit() {
            num_str.push(ch);
            self.next()?;
          } else {
            break;
          }
        }
      }
    }
    if let Some(ch) = self.input_code[self.pos..].chars().next() {
      if ch == 'e' || ch == 'E' {
        has_exponent = true;
        num_str.push(ch);
        self.next()?;
        if matches!(self.input_code[self.pos..].chars().next(), Some('+' | '-')) {
          num_str.push(self.next()?);
        }
        if !matches!(self.input_code[self.pos..].chars().next(), Some(c) if c.is_ascii_digit()) {
          return self.parse_err("A digit is required in the exponent part");
        }
        while let Some(ch) = self.input_code[self.pos..].chars().next() {
          if ch.is_ascii_digit() {
            num_str.push(ch);
            self.next()?;
          } else {
            break;
          }
        }
      }
    }
    if !has_decimal && !has_exponent {
      num_str.parse::<i64>().map_or_else(
        |_| self.parse_err("Invalid integer value"),
        |int_val| {
          Ok(Json {
            pos: start,
            ln: self.ln,
            value: JValue::Int(VKind::Lit(int_val)),
          })
        },
      )
    } else {
      num_str.parse::<f64>().map_or_else(
        |_| self.parse_err("Invalid numeric value"),
        |float_val| {
          Ok(Json {
            pos: start,
            ln: self.ln,
            value: JValue::Float(VKind::Lit(float_val)),
          })
        },
      )
    }
  }
  fn parse_string(&mut self) -> JResult {
    if !self.input_code[self.pos..].starts_with('\"') {
      return self.parse_err("Missing opening quotation for string");
    }
    let start = self.pos;
    self.pos += 1;
    let mut result = String::new();
    while self.pos < self.input_code.len() {
      let c = self.next()?;
      match c {
        '\"' => {
          return Ok(Json {
            pos: start,
            ln: self.ln,
            value: JValue::String(VKind::Lit(result)),
          });
        }
        '\\' => {
          let escaped = self.next()?;
          match escaped {
            'n' => result.push('\n'),
            't' => result.push('\t'),
            'r' => result.push('\r'),
            'b' => result.push('\x08'),
            'f' => result.push('\x0C'),
            '\\' => result.push('\\'),
            '/' => result.push('/'),
            '"' => result.push('"'),
            'u' => {
              let mut hex = String::new();
              for _ in 0..4 {
                if let Ok(c) = self.next() {
                  if c.is_ascii_hexdigit() {
                    hex.push(c);
                  } else {
                    return self.parse_err("Invalid hex digit");
                  }
                } else {
                  return self.parse_err("Failed read hex");
                }
              }
              let cp =
                u32::from_str_radix(&hex, 16).map_err(|_| String::from("Invalid code point"))?;
              if (0xD800..=0xDFFF).contains(&cp) {
                return self.parse_err("Invalid unicode");
              }
              result.push(std::char::from_u32(cp).ok_or("Invalid unicode")?);
            }
            _ => {
              return self.parse_err("Invalid escape sequence");
            }
          }
        }
        c if c < '\u{20}' => {
          return self.parse_err("Invalid control character");
        }
        _ => result.push(c),
      }
    }
    self.parse_err("String is not properly terminated")
  }
  fn parse_array(&mut self) -> JResult {
    let start_pos = self.pos;
    let start_ln = self.ln;
    let mut array = Vec::new();
    self.expect('[')?;
    self.skip_ws();
    if self.input_code[self.pos..].starts_with(']') {
      self.pos += 1;
      return Ok(Json {
        pos: start_pos,
        ln: start_ln,
        value: JValue::Array(VKind::Lit(array)),
      });
    }
    loop {
      array.push(self.parse_value()?);
      if self.input_code[self.pos..].starts_with(']') {
        self.pos += 1;
        return Ok(Json {
          pos: start_pos,
          ln: start_ln,
          value: JValue::Array(VKind::Lit(array)),
        });
      } else if self.input_code[self.pos..].starts_with(',') {
        self.pos += 1;
      } else {
        return self.parse_err("Invalid array separator");
      }
    }
  }
  fn parse_object(&mut self) -> JResult {
    let start_pos = self.pos;
    let start_ln = self.ln;
    let mut object = HashMap::new();
    self.expect('{')?;
    self.skip_ws();
    if self.input_code[self.pos..].starts_with('}') {
      self.pos += 1;
      return Ok(Json {
        pos: start_pos,
        ln: start_ln,
        value: JValue::Object(VKind::Lit(object)),
      });
    }
    loop {
      let key = self.parse_value()?;
      let JValue::String(VKind::Lit(s)) = key.value else {
        return self.obj_err("Keys must be strings", &key);
      };
      self.expect(':')?;
      let value = self.parse_value()?;
      object.insert(s, value);
      if self.input_code[self.pos..].starts_with('}') {
        self.pos += 1;
        return Ok(Json {
          pos: start_pos,
          ln: start_ln,
          value: JValue::Object(VKind::Lit(object)),
        });
      }
      if self.input_code[self.pos..].starts_with(',') {
        self.pos += 1
      } else {
        return self.parse_err("Invalid object separator");
      }
    }
  }
  fn parse_value(&mut self) -> JResult {
    self.skip_ws();
    if self.pos >= self.input_code.len() {
      return self.parse_err("Unexpected end of text");
    }
    let result = match self.input_code[self.pos..].chars().next() {
      Some('"') => self.parse_string(),
      Some('{') => self.parse_object(),
      Some('[') => self.parse_array(),
      Some('t') => self.parse_name("true", JValue::Bool(VKind::Lit(true))),
      Some('f') => self.parse_name("false", JValue::Bool(VKind::Lit(false))),
      Some('n') => self.parse_name("null", JValue::Null),
      _ => self.parse_number(),
    };
    self.skip_ws();
    result
  }
  pub fn build(&mut self, parsed: Json, filename: &str) -> JResult {
    self.f_table.insert("=".into(), JParser::set_var as F<Self>);
    self.f_table.insert("$".into(), JParser::get_var as F<Self>);
    self.f_table.insert("+".into(), JParser::plus as F<Self>);
    self.f_table.insert("-".into(), JParser::minus as F<Self>);
    self
      .f_table
      .insert("message".into(), JParser::message as F<Self>);
    self
      .f_table
      .insert("begin".into(), JParser::begin as F<Self>);
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
    self.validate(
      args.len() == 1,
      "begin",
      "at least one",
      &args[0],
    )?;
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
#[allow(dead_code)]
impl Json {
  pub fn print_json(&self) -> fmt::Result {
    let mut output = String::new();
    if self.write_json(&mut output).is_ok() {
      println!("{output}");
    }
    Ok(())
  }
  fn write_json(&self, out: &mut String) -> fmt::Result {
    match &self.value {
      JValue::Null => out.write_str("null"),
      JValue::Bool(maybe_b) => match maybe_b {
        VKind::Lit(b) => match b {
          true => write!(out, "true"),
          false => write!(out, "false"),
        },
        VKind::Var(v) => write!(out, "({v}: bool)"),
      },
      JValue::Int(maybe_i) => match maybe_i {
        VKind::Lit(i) => write!(out, "{i}"),
        VKind::Var(v) => write!(out, "({v}: int)"),
      },
      JValue::Float(maybe_f) => match maybe_f {
        VKind::Lit(f) => write!(out, "{f}"),
        VKind::Var(v) => write!(out, "({v}: float)"),
      },
      JValue::String(maybe_s) => match maybe_s {
        VKind::Lit(s) => write!(out, "\"{}\"", self.escape_string(s)),
        VKind::Var(v) => write!(out, "({v}: string)"),
      },
      JValue::Array(maybe_a) => match maybe_a {
        VKind::Var(v) => {
          write!(out, "({v}: array)")
        }
        VKind::Lit(a) => {
          out.write_str("[")?;
          for (i, item) in a.iter().enumerate() {
            if i > 0 {
              out.write_str(", ")?;
            }
            item.write_json(out)?;
          }
          out.write_str("]")
        }
      },
      JValue::Function(maybe_fn) => match maybe_fn {
        VKind::Var(v) => {
          write!(out, "({v}: function)")
        }
        VKind::Lit(f) => {
          out.write_str("(")?;
          for (i, item) in f.iter().enumerate() {
            if i > 0 {
              out.write_str(", ")?;
            }
            item.write_json(out)?;
          }
          out.write_str(": function)")
        }
      },
      JValue::Object(maybe_o) => match maybe_o {
        VKind::Var(v) => {
          write!(out, "({v}: array)")
        }
        VKind::Lit(o) => {
          out.write_str("{")?;
          for (i, (k, v)) in o.iter().enumerate() {
            if i > 0 {
              out.write_str(", ")?;
            }
            write!(out, "\"{}\": ", self.escape_string(k))?;
            v.write_json(out)?;
          }
          out.write_str("}")
        }
      },
    }
  }
  fn escape_string(&self, s: &str) -> String {
    let mut escaped = String::new();
    for c in s.chars() {
      match c {
        '\"' => escaped.push_str("\\\""),
        '\\' => escaped.push_str("\\\\"),
        '\n' => escaped.push_str("\\n"),
        '\t' => escaped.push_str("\\t"),
        '\r' => escaped.push_str("\\r"),
        '\u{08}' => escaped.push_str("\\b"),
        '\u{0C}' => escaped.push_str("\\f"),
        c if c < '\u{20}' => escaped.push_str(&format!("\\u{:04x}", c as u32)),
        _ => escaped.push(c),
      }
    }
    escaped
  }
}
fn error_exit(text: String) -> ! {
  let mut nu = String::new();
  eprint!("{text}\nPress Enter to exit:");
  let _ = io::stdin().read_line(&mut nu);
  std::process::exit(1)
}
fn main() -> ! {
  let args: Vec<String> = env::args().collect();
  if args.len() != 2 {
    eprintln!("Usage: {} <input json file>", args[0]);
    std::process::exit(0)
  }
  let input_code = fs::read_to_string(&args[1])
    .unwrap_or_else(|e| error_exit(format!("Failed to read file: {e}")));
  let mut parser = JParser::default();
  let parsed = parser
    .parse(&input_code)
    .unwrap_or_else(|e| error_exit(format!("ParseError: {e}")));
  #[cfg(debug_assertions)]
  {
    parsed
      .print_json()
      .unwrap_or_else(|e| error_exit(format!("Couldn't print json: {e}")));
  }
  let json_file = Path::new(&args[1])
    .file_stem()
    .unwrap_or_else(|| error_exit(format!("Invalid filename: {}", args[1])))
    .to_string_lossy();
  let asm_file = format!("{json_file}.s");
  let exe_file = format!("{json_file}.exe");
  parser
    .build(parsed, &asm_file)
    .unwrap_or_else(|e| error_exit(format!("CompileError: {e}")));
  if !Command::new("gcc")
    .args([&asm_file, "-o", &exe_file, "-nostartfiles", "-luser32", "-lkernel32"])
    .status()
    .unwrap_or_else(|e| error_exit(format!("Failed to assemble or link: {e}")))
    .success()
  {
    error_exit(String::from("Failed to assemble or link"))
  };
  let mut path = env::current_dir()
    .unwrap_or_else(|e| error_exit(format!("Failed to get current directory: {e}")));
  path.push(&exe_file);
  let exit_code = Command::new(path)
    .spawn()
    .unwrap_or_else(|e| error_exit(format!("Failed to spawn child process: {e}")))
    .wait()
    .unwrap_or_else(|e| error_exit(format!("Failed to wait for child process: {e}")))
    .code()
    .unwrap_or_else(|| error_exit(String::from("Failed to retrieve the exit code")));
  std::process::exit(exit_code)
}
