use std::collections::{HashMap, HashSet};
use std::env;
use std::error::Error;
use std::fmt::{self, Write as _};
use std::fs::{self, File};
use std::io::Write as _;
use std::path::Path;
use std::process::Command;
type JResult = Result<Json, Box<dyn Error>>;
type FType<T> = fn(&mut T, &[Json], &mut String) -> JResult;
fn get_error_line(input_code: &str, index: &usize) -> Option<String> {
  if *index >= input_code.len() {
    return None;
  }
  let start = input_code[..*index].rfind('\n').map_or(0, |pos| pos + 1);
  let end = input_code[*index..]
    .find('\n')
    .map_or(input_code.len(), |pos| index + pos);
  let error_line = &input_code[start..end];
  let marker = " ".repeat(index - start) + "^";
  Some(format!("{}\n{}", error_line, marker))
}
macro_rules! genErr {
  ($text:expr, $pos:expr, $input_code:expr) => {
    Err(
      format!(
        "{}\nError occurred at byte: {}\nError position:\n{}",
        $text,
        &(*$pos + 1),
        get_error_line($input_code, $pos).unwrap_or(String::from("End of File"))
      )
      .into(),
    )
  };
}
#[derive(Debug, Clone)]
struct Json {
  pub pos: usize,
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
struct JParser<'a> {
  input_code: &'a str,
  pos: usize,
  extern_set: HashSet<String>,
  data: String,
  bss: String,
  text: String,
  func_table: HashMap<String, FType<Self>>,
  vars: HashMap<String, Json>,
  seed: usize,
}
impl<'a> JParser<'a> {
  pub fn new(code: &'a str) -> Self {
    let mut table = HashMap::new();
    table.insert(String::from("="), JParser::f_setvar as FType<Self>);
    table.insert(String::from("$"), JParser::f_getvar as FType<Self>);
    table.insert(String::from("+"), JParser::f_plus as FType<Self>);
    table.insert(String::from("-"), JParser::f_minus as FType<Self>);
    table.insert(String::from("message"), JParser::f_message as FType<Self>);
    table.insert(String::from("begin"), JParser::f_begin as FType<Self>);
    Self {
      input_code: code,
      extern_set: HashSet::new(),
      data: String::from(".section .data\n"),
      bss: String::from(".section .bss\n"),
      text: String::from(".section .text\n"),
      func_table: table,
      vars: HashMap::new(),
      pos: 0,
      seed: 0,
    }
  }
  fn next_char(&mut self) -> Result<char, String> {
    let ch = self.input_code[self.pos..]
      .chars()
      .next()
      .ok_or("Reached end of text")?;
    self.pos += ch.len_utf8();
    Ok(ch)
  }
  fn expect(&mut self, expected: char) -> Result<(), String> {
    if self.input_code[self.pos..].starts_with(expected) {
      self.next_char()?;
      Ok(())
    } else {
      genErr!(
        format!("Expected character '{}' not found.", expected),
        &self.pos,
        self.input_code
      )
    }
  }
  fn get_name(&mut self) -> String {
    self.seed += 1;
    format!("_{:x}", self.seed)
  }
  fn parse(&mut self) -> JResult {
    let result = self.parse_value()?;
    self.skipws();
    if self.pos != self.input_code.len() {
      genErr!("Unexpected trailing characters", &self.pos, self.input_code)
    } else {
      Ok(result)
    }
  }
  fn skipws(&mut self) {
    if let Some(non_ws_pos) = self.input_code[self.pos..].find(|c: char| !c.is_whitespace()) {
      self.pos += non_ws_pos;
    } else {
      self.pos = self.input_code.len();
    }
  }
  fn parse_name(&mut self, n: &str, v: JValue) -> JResult {
    if self.input_code[self.pos..].starts_with(n) {
      let start = self.pos;
      self.pos += n.len();
      Ok(Json {
        pos: start,
        value: v,
      })
    } else {
      genErr!(
        format!("Faild to parse '{}'", n),
        &self.pos,
        self.input_code
      )
    }
  }
  fn parse_number(&mut self) -> JResult {
    let start = self.pos;
    let mut num_str = String::new();
    let mut has_decimal = false;
    let mut has_exponent = false;
    if self.input_code[self.pos..].starts_with('-') {
      num_str.push('-');
      self.next_char()?;
    }
    while let Some(ch) = self.input_code[self.pos..].chars().next() {
      match ch {
        '0'..='9' => {
          num_str.push(ch);
          self.next_char()?;
        }
        '.' if !has_decimal && !has_exponent => {
          has_decimal = true;
          num_str.push(ch);
          self.next_char()?;
          if !matches!(self.input_code[self.pos..].chars().next()
, Some(c) if c.is_ascii_digit())
          {
            return genErr!(
              "There are no digits after the decimal point",
              &self.pos,
              self.input_code
            );
          }
        }
        'e' | 'E' if !has_exponent => {
          has_exponent = true;
          num_str.push(ch);
          self.next_char()?;
          if matches!(self.input_code[self.pos..].chars().next(), Some('+' | '-')) {
            num_str.push(self.next_char()?);
          }
          if !matches!(self.input_code[self.pos..].chars().next()
, Some(c) if c.is_ascii_digit())
          {
            return genErr!(
              "Missing digits in the exponent part",
              &self.pos,
              self.input_code
            );
          }
        }
        _ => break,
      }
    }
    match num_str.parse::<i64>() {
      Ok(int_val) if !has_decimal && !has_exponent => Ok(Json {
        pos: start,
        value: JValue::Int(VKind::Lit(int_val)),
      }),
      _ => num_str.parse::<f64>().map_or_else(
        |_| genErr!("Invalid value", &self.pos, self.input_code),
        |float_val| {
          Ok(Json {
            pos: start,
            value: JValue::Float(VKind::Lit(float_val)),
          })
        },
      ),
    }
  }
  fn parse_string(&mut self) -> JResult {
    if !self.input_code[self.pos..].starts_with('\"') {
      return genErr!(
        "Missing opening quotation for string",
        &self.pos,
        self.input_code
      );
    }
    let start = self.pos;
    self.pos += 1;
    let mut result = String::new();
    while self.pos < self.input_code.len() {
      let c = self.next_char()?;
      match c {
        '\"' => {
          return Ok(Json {
            pos: start,
            value: JValue::String(VKind::Lit(result)),
          });
        }
        '\\' => {
          let escaped = self.next_char()?;
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
                if let Ok(c) = self.next_char() {
                  hex.push(c);
                } else {
                  return genErr!("Faild read hex", &self.pos, self.input_code);
                }
              }
              let cp =
                u32::from_str_radix(&hex, 16).map_err(|_| String::from("Invalid codepoint"))?;
              result.push(std::char::from_u32(cp).ok_or("Invalid unicode")?);
            }
            _ => return genErr!("Invalid escape sequense", &self.pos, self.input_code),
          }
        }
        _ => result.push(c),
      }
    }
    genErr!(
      "String is not properly terminated",
      &self.pos,
      self.input_code
    )
  }
  fn parse_array(&mut self) -> JResult {
    let start = self.pos;
    let mut array = Vec::new();
    self.expect('[')?;
    self.skipws();
    if self.input_code[self.pos..].starts_with(']') {
      self.pos += 1;
      return Ok(Json {
        pos: start,
        value: JValue::Array(VKind::Lit(array)),
      });
    }
    loop {
      array.push(self.parse_value()?);
      self.skipws();
      if self.input_code[self.pos..].starts_with(']') {
        self.pos += 1;
        return Ok(Json {
          pos: start,
          value: JValue::Array(VKind::Lit(array)),
        });
      } else if self.input_code[self.pos..].starts_with(',') {
        self.pos += 1;
        self.skipws();
      } else {
        return genErr!("Invalid array separator", &self.pos, self.input_code);
      }
    }
  }
  fn parse_object(&mut self) -> JResult {
    let start = self.pos;
    let mut object = HashMap::new();
    self.expect('{')?;
    self.skipws();
    if self.input_code[self.pos..].starts_with('}') {
      self.pos += 1;
      return Ok(Json {
        pos: start,
        value: JValue::Object(VKind::Lit(object)),
      });
    }
    loop {
      let key = match self.parse_string()? {
        Json {
          pos: _,
          value: JValue::String(VKind::Lit(s)),
        } => s,
        Json {
          pos: invalid_pos,
          value: _,
        } => return genErr!("Keys must be strings", &invalid_pos, self.input_code),
      };
      self.skipws();
      self.expect(':')?;
      self.skipws();
      let value = self.parse_value()?;
      object.insert(key, value);
      self.skipws();
      if self.input_code[self.pos..].starts_with('}') {
        self.pos += 1;
        return Ok(Json {
          pos: start,
          value: JValue::Object(VKind::Lit(object)),
        });
      }
      if self.input_code[self.pos..].starts_with(',') {
        self.pos += 1;
        self.skipws();
      } else {
        return genErr!("Invalid object separator", &self.pos, self.input_code);
      }
    }
  }
  fn parse_value(&mut self) -> JResult {
    self.skipws();
    if self.pos >= self.input_code.len() {
      return genErr!("Unexpected end of text", &self.pos, self.input_code);
    }
    match self.input_code[self.pos..].chars().next() {
      Some('"') => self.parse_string(),
      Some('{') => self.parse_object(),
      Some('[') => self.parse_array(),
      Some('t') => self.parse_name("true", JValue::Bool(VKind::Lit(true))),
      Some('f') => self.parse_name("false", JValue::Bool(VKind::Lit(false))),
      Some('n') => self.parse_name("null", JValue::Null),
      _ => self.parse_number(),
    }
  }
  pub fn build(&mut self, parsed: Json, filename: &String) -> Result<(), Box<dyn Error>> {
    self.extern_set.insert(String::from("ExitProcess"));
    self
      .extern_set
      .insert(String::from("SetConsoleCP, SetConsoleOutputCP"));
    self.extern_set.insert(String::from("GetLastError"));
    self.extern_set.insert(String::from("WriteConsoleW"));
    self.extern_set.insert(String::from("FormatMessageW"));
    self.extern_set.insert(String::from("GetStdHandle"));
    self.bss.push_str(
      r#"  .lcomm errorMessage, 512
  .lcomm lastError, 4
  .lcomm STDOUT, 8
  .lcomm STDERR, 8
  .lcomm STDIN, 8
"#,
    );
    let mut mainfunc = String::from(
      r#"_start:
  sub rsp, 40
  mov ecx, 65001
  call SetConsoleCP
  mov ecx, 65001
  call SetConsoleOutputCP
  mov ecx, -10
  call GetStdHandle
  mov [rip + STDIN], rax
  mov ecx, -11
  call GetStdHandle
  mov [rip + STDOUT], rax
  mov ecx, -12
  call GetStdHandle
  mov [rip + STDERR], rax
"#,
    );
    self.eval(&parsed, &mut mainfunc)?;
    let mut file = File::create(filename)?;
    writeln!(file, ".intel_syntax noprefix")?;
    writeln!(file, ".global start")?;
    for inc in &self.extern_set {
      writeln!(file, ".extern {}", inc)?;
    }
    write!(file, "{}", self.data)?;
    write!(file, "{}", self.bss)?;
    write!(file, "{}", self.text)?;
    write!(file, "{}", mainfunc)?;
    write!(
      file,
      r#"  xor ecx, ecx
  call ExitProcess
display_error:
  call GetLastError
  mov [rip + errorMessage], eax
  sub rsp, 32
  mov ecx, 0x1200
  xor edx, edx
  mov r8d, eax
  xor r9d, r9d
  lea rax, [rip + errorMessage]
  mov [rsp + 32], rax
  mov qword ptr [rsp + 40], 512
  mov qword ptr [rsp + 48], 0
  call FormatMessageW
  add rsp, 16
  test eax, eax
  jz exit_program
  mov rcx, [rip + STDERR]
  lea rdx, [rip + errorMessage]
  mov r8, 256
  lea r9, [rsp + 32]
  mov qword ptr [rsp + 40], 0
  add rsp, 16
  call WriteConsoleW
exit_program:
  mov ecx, [rip + lastError]
  call ExitProcess
"#
    )?;
    Ok(())
  }
  fn eval(&mut self, parsed: &Json, function: &mut String) -> JResult {
    let Json {
      pos: listpos,
      value: JValue::Array(VKind::Lit(list)),
    } = parsed
    else {
      return Ok(parsed.clone());
    };
    if list.is_empty() {
      return genErr!(
        "An procedure was expected, but an empty list was provided",
        listpos,
        self.input_code
      );
    };
    match &list[0] {
      Json {
        pos: cmdpos,
        value: JValue::String(VKind::Lit(cmd)),
      } => {
        if cmd == "lambda" {
          return Ok(parsed.clone());
        }
        if let Some(func) = self.func_table.get(cmd.as_str()) {
          return func(self, list, function);
        }
        genErr!(
          format!("Undefined function: {}", cmd),
          cmdpos,
          self.input_code
        )
      }
      _ => {
        let mut func_buffer = String::new();
        let funcvalue = self.eval_lambda(parsed, &mut func_buffer)?;
        self.text.push_str(&func_buffer);
        Ok(funcvalue)
      }
    }
  }
  fn eval_lambda(&mut self, parsed: &Json, function: &mut String) -> JResult {
    let Json {
      pos: func_list_pos,
      value: JValue::Array(VKind::Lit(func_list)),
    } = &parsed
    else {
      return genErr!(
        "Only a lambda list or a string is allowed as the first element of a list",
        &parsed.pos,
        self.input_code
      );
    };
    let Json {
      pos: cmdpos,
      value: JValue::String(VKind::Lit(cmd)),
    } = &parsed
    else {
      return genErr!(
        "Only a lambda list or a string is allowed as the first element of a list",
        &func_list_pos,
        self.input_code
      );
    };
    if cmd != "lambda" {
      return genErr!(
        "Only a lambda list or a string is allowed as the first element of a list",
        cmdpos,
        self.input_code
      );
    }
    if func_list.len() < 3 {
      return genErr!("Invalid function defintion", func_list_pos, self.input_code);
    };
    let Json {
      pos: _,
      value: JValue::Array(VKind::Lit(params)),
    } = &func_list[1]
    else {
      return genErr!(
        "The second element of a lambda list must be an argument list",
        func_list_pos,
        self.input_code
      );
    };
    for i in func_list.iter().skip(3) {
      self.eval(i, function)?;
    }
    Ok(Json {
      pos: 1,
      value: JValue::Function(VKind::Lit(params.clone())),
    })
  }
  fn f_setvar(&mut self, args: &[Json], function: &mut String) -> JResult {
    if args.len() != 3 {
      return genErr!(
        "'=' is exactly two arguments",
        &args[0].pos,
        self.input_code
      );
    }
    if let JValue::String(VKind::Lit(var_name)) = &args[1].value {
      let value = self.eval(&args[2], function)?;
      if value.value.is_lit() {
        match value.value {
          JValue::String(VKind::Lit(s)) => {
            let n = self.get_name();
            writeln!(self.data, "  {}: .string \"{}\"", n, s)?;
            self.vars.insert(
              var_name.clone(),
              Json {
                pos: args[0].pos,
                value: JValue::String(VKind::Var(n)),
              },
            );
          }
          _ => {
            return genErr!(
              "Assignment to an unimplemented type",
              &args[0].pos,
              self.input_code
            );
          }
        }
      } else {
        self.vars.insert(var_name.clone(), value);
      }
      Ok(Json {
        pos: args[0].pos,
        value: JValue::Null,
      })
    } else {
      genErr!(
        "Variable names must be compile-time fixed strings",
        &args[0].pos,
        ""
      )
    }
  }
  fn f_getvar(&mut self, args: &[Json], _: &mut String) -> JResult {
    if args.len() != 2 {
      return genErr!(
        "'=' is exactly one arguments",
        &args[0].pos,
        self.input_code
      );
    }
    if let JValue::String(VKind::Lit(var_name)) = &args[1].value {
      if let Some(value) = self.vars.get(var_name) {
        Ok(value.clone())
      } else {
        genErr!(
          &format!("Undefined variables: '{}'", var_name),
          &args[0].pos,
          self.input_code
        )
      }
    } else {
      genErr!(
        "Variable names must be compile-time fixed strings",
        &args[0].pos,
        self.input_code
      )
    }
  }
  fn f_plus(&mut self, args: &[Json], function: &mut String) -> JResult {
    if args.len() <= 1 {
      return genErr!(
        "'+' requires at least one arguments",
        &args[0].pos,
        self.input_code
      );
    };
    let Ok(Json {
      pos: _,
      value: JValue::Int(result),
    }) = self.eval(&args[1], function)
    else {
      return genErr!(
        "'+' requires integer operands",
        &args[0].pos,
        self.input_code
      );
    };
    match result {
      VKind::Lit(l) => writeln!(function, "  mov rax, {}", l)?,
      VKind::Var(v) => writeln!(function, "  mov rax, [rip + {}]", v)?,
    }
    for a in &args[2..args.len()] {
      let Ok(Json {
        pos: _,
        value: JValue::Int(result),
      }) = self.eval(a, function)
      else {
        return genErr!(
          "'+' requires integer operands",
          &args[0].pos,
          self.input_code
        );
      };
      match result {
        VKind::Lit(l) => writeln!(function, "  add rax, {}", l)?,
        VKind::Var(v) => writeln!(function, "  add rax, [rip + {}]", v)?,
      }
    }
    let assign_name = self.get_name();
    writeln!(self.bss, "  .lcomm {}, 8", assign_name)?;
    writeln!(function, "  mov rax, [rip + {}]", assign_name)?;
    Ok(Json {
      pos: args[0].pos,
      value: JValue::Int(VKind::Var(assign_name)),
    })
  }
  fn f_begin(&mut self, args: &[Json], function: &mut String) -> JResult {
    if args.len() <= 1 {
      return genErr!(
        "begin requires at least one arguments",
        &args[0].pos,
        self.input_code
      );
    };
    let mut result: JResult = Err("Unreachable".into());
    for a in &args[1..args.len()] {
      result = self.eval(a, function)
    }
    result
  }
  fn f_minus(&mut self, args: &[Json], function: &mut String) -> JResult {
    if args.len() <= 1 {
      return genErr!(
        "'-' requires at least one operand",
        &args[0].pos,
        self.input_code
      );
    };
    let Ok(Json {
      pos: _,
      value: JValue::Int(result),
    }) = self.eval(&args[1], function)
    else {
      return genErr!(
        "'-' requires integer operands",
        &args[0].pos,
        self.input_code
      );
    };
    match result {
      VKind::Lit(l) => writeln!(function, "  mov rax, {}", l)?,
      VKind::Var(v) => writeln!(function, "  mov rax, [rip + {}]", v)?,
    }
    for a in &args[2..args.len()] {
      let Ok(Json {
        pos: _,
        value: JValue::Int(result),
      }) = self.eval(a, function)
      else {
        return genErr!(
          "'-' requires integer operands",
          &args[0].pos,
          self.input_code
        );
      };
      match result {
        VKind::Lit(l) => writeln!(function, "  sub rax, {}", l)?,
        VKind::Var(v) => writeln!(function, "  sub rax, [rip + {}]", v)?,
      }
    }
    let assign_name = self.get_name();
    writeln!(self.bss, "  .lcomm {}, 8", assign_name)?;
    writeln!(function, "  movq [rip + {}], rax", assign_name)?;
    Ok(Json {
      pos: args[0].pos,
      value: JValue::Int(VKind::Var(assign_name)),
    })
  }
  fn f_message(&mut self, args: &[Json], function: &mut String) -> JResult {
    if args.len() != 3 {
      return genErr!(
        "message requires three operands",
        &args[0].pos,
        self.input_code
      );
    };
    let parsed2 = self.eval(&args[2], function)?;
    let msg = match parsed2 {
      Json {
        pos: _,
        value: JValue::String(VKind::Lit(l)),
      } => {
        let mn = self.get_name();
        writeln!(self.data, "  {}: .string \"{}\"", mn, l)?;
        mn
      }
      Json {
        pos: _,
        value: JValue::String(VKind::Var(v)),
      } => v,
      _ => {
        return genErr!(
          "The second argument of message must be a string",
          &args[2].pos,
          self.input_code
        );
      }
    };
    let parsed1 = self.eval(&args[1], function)?;
    self.extern_set.insert(String::from("MessageBoxA"));
    let title = match parsed1 {
      Json {
        pos: _,
        value: JValue::String(VKind::Lit(l)),
      } => {
        let mn = self.get_name();
        writeln!(self.data, "  {}: .string \"{}\"", mn, l)?;
        mn
      }
      Json {
        pos: _,
        value: JValue::String(VKind::Var(v)),
      } => v,
      _ => {
        return genErr!(
          "The first argument of message must be a string",
          &args[1].pos,
          self.input_code
        );
      }
    };
    let retcode = self.get_name();
    writeln!(self.bss, "  .lcomm {}, 8", retcode)?;
    writeln!(
      function,
      r#"  xor ecx, ecx
  lea rdx, [rip + {}]
  lea r8, [rip + {}]
  xor r9d, r9d
  call MessageBoxA
  test eax, eax
  jz display_error
  mov [rip + {}], rax
"#,
      msg, title, retcode
    )?;
    Ok(Json {
      pos: args[0].pos,
      value: JValue::Null,
    })
  }
}
impl Json {
  pub fn print_json(&self) -> fmt::Result {
    let mut output = String::new();
    if self.write_json(&mut output).is_ok() {
      writeln!(output)?;
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
        VKind::Var(v) => write!(out, "({}: bool)", v),
      },
      JValue::Int(maybe_i) => match maybe_i {
        VKind::Lit(i) => write!(out, "{}", i),
        VKind::Var(v) => write!(out, "({}: int)", v),
      },
      JValue::Float(maybe_f) => match maybe_f {
        VKind::Lit(f) => write!(out, "{}", f),
        VKind::Var(v) => write!(out, "({}: float)", v),
      },
      JValue::String(maybe_s) => match maybe_s {
        VKind::Lit(s) => write!(out, "\"{}\"", self.escape_string(s)),
        VKind::Var(v) => write!(out, "({}: string)", v),
      },
      JValue::Array(maybe_a) => match maybe_a {
        VKind::Var(v) => {
          write!(out, "({}: array)", v)
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
          write!(out, "({}: function)", v)
        }
        VKind::Lit(f) => {
          out.write_str("(")?;
          for (i, item) in f.iter().enumerate() {
            if i > 0 {
              out.write_str(", ")?;
            }
            item.write_json(out)?;
          }
          out.write_str(": function")
        }
      },
      JValue::Object(maybe_o) => match maybe_o {
        VKind::Var(v) => {
          write!(out, "({}: array)", v)
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
fn main() -> Result<(), Box<dyn Error>> {
  let args: Vec<String> = env::args().collect();
  if args.len() != 2 {
    println!("Usage: {} <input json file>", args[0]);
    return Ok(());
  }
  let input_code = fs::read_to_string(&args[1])?;
  let mut parser = JParser::new(&input_code);
  let parsed = parser
    .parse()
    .map_err(|errmsg| panic!("\nParseError: {}", errmsg))?;
  if false {
    parsed.print_json()?;
  }
  let filename = Path::new(&args[1])
    .file_stem()
    .ok_or(format!("Invalid file name: {}", args[1]))?
    .to_string_lossy();
  let asm_file = format!("{}.s", filename);
  let exe_file = format!("{}.exe", filename);
  parser
    .build(parsed, &asm_file)
    .map_err(|errmsg| panic!("\nCompileError: {}", errmsg))?;
  Command::new("gcc")
    .args([&asm_file, "-o", &exe_file, "-nostartfiles"])
    .status()
    .map_err(|_| "Failed assembling or linking process")?
    .success()
    .then_some(())
    .ok_or("Failed assembling or linking process")?;
  let mut path = env::current_dir()?;
  path.push(&exe_file);
  let exit_code = Command::new(path)
    .spawn()?
    .wait()?
    .code()
    .ok_or("Failed to retrieve the exit code")?;
  std::process::exit(exit_code);
}
