use std::collections::{HashMap, HashSet};
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
    return "Error: Empty input".to_string();
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
  format!("{}\n{}^", &input_code[start..end], ws)
}
macro_rules! genErr {
  ($text:expr, $pos:expr,$ln: expr, $input_code:expr) => {
    Err(
      format!(
        "{}\nError occurred on line: {}\nError position:\n{}",
        $text,
        $ln + 1,
        get_error_line($input_code, $pos)
      )
      .into(),
    )
  };
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
  extern_set: HashSet<String>,
  data: String,
  bss: String,
  text: String,
  ftable: HashMap<String, F<Self>>,
  vars: HashMap<String, Json>,
  seed: usize,
  ln: usize,
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
  fn expect(&mut self, expected: char) -> Result<(), String> {
    if self.input_code[self.pos..].starts_with(expected) {
      self.next()?;
      Ok(())
    } else {
      genErr!(
        format!("Expected character '{}' not found.", expected),
        self.pos,
        self.ln,
        self.input_code
      )
    }
  }
  fn get_name(&mut self) -> String {
    self.seed += 1;
    format!(".{:x}", self.seed)
  }
  fn validate(
    &self,
    flag: bool,
    name: String,
    text: String,
    pos: usize,
    ln: usize,
  ) -> Result<(), Box<dyn Error>> {
    if flag {
      return genErr!(
        format!("\"{name}\" requires {text} argument"),
        pos,
        ln,
        self.input_code
      );
    };
    Ok(())
  }
  fn parse(&mut self, code: &'a str) -> JResult {
    self.input_code = code;
    let result = self.parse_value()?;
    self.skipws();
    if self.pos != self.input_code.len() {
      genErr!(
        "        Unexpected trailing characters",
        self.pos,
        self.ln,
        self.input_code
      )
    } else {
      Ok(result)
    }
  }
  fn skipws(&mut self) {
    while self.pos < self.input_code.len() {
      let c = self.input_code[self.pos..].chars().next().unwrap();
      if c.is_whitespace() {
        if c == '\n' {
          self.ln += 1;
        }
        self.pos += c.len_utf8();
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
      genErr!(
        format!("Faild to parse '{n}'"),
        self.pos,
        self.ln,
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
      self.next()?;
    }
    if self.input_code[self.pos..].starts_with('0') {
      num_str.push('0');
      self.next()?;
      if matches!(self.input_code[self.pos..].chars().next(), Some(c) if c.is_ascii_digit()) {
        return genErr!(
          "          Leading zeros are not allowed in numbers",
          self.pos,
          self.ln,
          self.input_code
        );
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
      return genErr!("Invalid number format", self.pos, self.ln, self.input_code);
    }
    if let Some(ch) = self.input_code[self.pos..].chars().next() {
      if ch == '.' {
        has_decimal = true;
        num_str.push(ch);
        self.next()?;
        if !matches!(self.input_code[self.pos..].chars().next(), Some(c) if c.is_ascii_digit()) {
          return genErr!(
            "A digit is required after the decimal point",
            self.pos,
            self.ln,
            self.input_code
          );
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
          return genErr!(
            "            A digit is required in the exponent part",
            self.pos,
            self.ln,
            self.input_code
          );
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
        |_| genErr!("Invalid integer value", self.pos, self.ln, self.input_code),
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
        |_| genErr!("Invalid numeric value", self.pos, self.ln, self.input_code),
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
      return genErr!(
        "        Missing opening quotation for string",
        self.pos,
        self.ln,
        self.input_code
      );
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
                    return genErr!("Invalid hex digit", self.pos, self.ln, self.input_code);
                  }
                } else {
                  return genErr!("Faild read hex", self.pos, self.ln, self.input_code);
                }
              }
              let cp =
                u32::from_str_radix(&hex, 16).map_err(|_| String::from("Invalid codepoint"))?;
              if (0xD800..=0xDFFF).contains(&cp) {
                return genErr!("Invalid unicode", self.pos, self.ln, self.input_code);
              }
              result.push(std::char::from_u32(cp).ok_or("Invalid unicode")?);
            }
            _ => {
              return genErr!(
                "                Invalid escape sequense",
                self.pos,
                self.ln,
                self.input_code
              );
            }
          }
        }
        c if c < '\u{20}' => {
          return genErr!(
            "            Invalid control character",
            self.pos,
            self.ln,
            self.input_code
          );
        }
        _ => result.push(c),
      }
    }
    genErr!(
      "String is not properly terminated",
      self.pos,
      self.ln,
      self.input_code
    )
  }
  fn parse_array(&mut self) -> JResult {
    let startpos = self.pos;
    let startln = self.ln;
    let mut array = Vec::new();
    self.expect('[')?;
    self.skipws();
    if self.input_code[self.pos..].starts_with(']') {
      self.pos += 1;
      return Ok(Json {
        pos: startpos,
        ln: startln,
        value: JValue::Array(VKind::Lit(array)),
      });
    }
    loop {
      array.push(self.parse_value()?);
      self.skipws();
      if self.input_code[self.pos..].starts_with(']') {
        self.pos += 1;
        return Ok(Json {
          pos: startpos,
          ln: startln,
          value: JValue::Array(VKind::Lit(array)),
        });
      } else if self.input_code[self.pos..].starts_with(',') {
        self.pos += 1;
        self.skipws();
      } else {
        return genErr!(
          "          Invalid array separator",
          self.pos,
          self.ln,
          self.input_code
        );
      }
    }
  }
  fn parse_object(&mut self) -> JResult {
    let startpos = self.pos;
    let startln = self.ln;
    let mut object = HashMap::new();
    self.expect('{')?;
    self.skipws();
    if self.input_code[self.pos..].starts_with('}') {
      self.pos += 1;
      return Ok(Json {
        pos: startpos,
        ln: startln,
        value: JValue::Object(VKind::Lit(object)),
      });
    }
    loop {
      let key = match self.parse_string()? {
        Json {
          pos: _,
          ln: _,
          value: JValue::String(VKind::Lit(s)),
        } => s,
        Json {
          pos: invalid_pos,
          ln: invalid_ln,
          value: _,
        } => {
          return genErr!(
            "            Keys must be strings",
            invalid_pos,
            invalid_ln,
            self.input_code
          );
        }
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
          pos: startpos,
          ln: startln,
          value: JValue::Object(VKind::Lit(object)),
        });
      }
      if self.input_code[self.pos..].starts_with(',') {
        self.pos += 1;
        self.skipws();
      } else {
        return genErr!(
          "          Invalid object separator",
          self.pos,
          self.ln,
          self.input_code
        );
      }
    }
  }
  fn parse_value(&mut self) -> JResult {
    self.skipws();
    if self.pos >= self.input_code.len() {
      return genErr!("Unexpected end of text", self.pos, self.ln, self.input_code);
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
  pub fn build(&mut self, parsed: Json, filename: &str) -> Result<(), Box<dyn Error>> {
    self.ftable.insert("=".into(), JParser::setvar as F<Self>);
    self.ftable.insert("$".into(), JParser::getvar as F<Self>);
    self.ftable.insert("+".into(), JParser::plus as F<Self>);
    self.ftable.insert("-".into(), JParser::minus as F<Self>);
    self
      .ftable
      .insert("message".into(), JParser::message as F<Self>);
    self
      .ftable
      .insert("begin".into(), JParser::begin as F<Self>);
    self.data.push_str(".data\n");
    self.bss.push_str(
      r#".bss
  .lcomm errorMessage, 512
  .lcomm errorCode, 4
  .lcomm STDOUT, 8
  .lcomm STDERR, 8
  .lcomm STDIN, 8
"#,
    );
    self.text.push_str(".text\n");
    self.extern_set.insert("ExitProcess".into());
    self.extern_set.insert("SetConsoleCP".into());
    self.extern_set.insert("GetLastError".into());
    self.extern_set.insert("MessageBoxW".into());
    self.extern_set.insert("FormatMessageW".into());
    self.extern_set.insert("GetStdHandle".into());
    let mut mainfunc = String::from(
      r#"_start:
  sub rsp, 40
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
    writeln!(
      file,
      r#"  xor ecx, ecx
  call ExitProcess
display_error:
  call GetLastError
  mov DWORD PTR [rip + errorCode], eax
  sub rsp, 32
  mov ecx, 0x1200
  xor edx, edx
  mov r8d, eax
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
  mov ecx, DWORD PTR [rip + errorCode]
  call ExitProcess"#
    )?;
    Ok(())
  }
  fn eval(&mut self, parsed: &Json, function: &mut String) -> JResult {
    let Json {
      pos: listpos,
      ln: listln,
      value: JValue::Array(VKind::Lit(list)),
    } = parsed
    else {
      return Ok(parsed.clone());
    };
    if list.is_empty() {
      return genErr!(
        "An procedure was expected, but an empty list was provided",
        *listpos,
        listln,
        self.input_code
      );
    };
    match &list[0] {
      Json {
        pos: cmdpos,
        ln: cmdln,
        value: JValue::String(VKind::Lit(cmd)),
      } => {
        if cmd == "lambda" {
          return Ok(parsed.clone());
        }
        if let Some(func) = self.ftable.get(cmd.as_str()) {
          return func(self, list, function);
        }
        genErr!(
          format!("Undefined function: {}", cmd),
          *cmdpos,
          cmdln,
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
      pos: _,
      ln: _,
      value: JValue::Array(VKind::Lit(func_list)),
    } = &parsed
    else {
      return genErr!(
        "Only a lambda list or a string is allowed as the first element of a list",
        parsed.pos,
        parsed.ln,
        self.input_code
      );
    };
    let Json {
      pos: _,
      ln: _,
      value: JValue::String(VKind::Lit(cmd)),
    } = &parsed
    else {
      return genErr!(
        "Only a lambda list or a string is allowed as the first element of a list",
        parsed.pos,
        parsed.ln,
        self.input_code
      );
    };
    if cmd != "lambda" {
      return genErr!(
        "Only a lambda list or a string is allowed as the first element of a list",
        parsed.pos,
        parsed.ln,
        self.input_code
      );
    }
    if func_list.len() < 3 {
      return genErr!(
        "        Invalid function defintion",
        parsed.pos,
        parsed.ln,
        self.input_code
      );
    };
    let Json {
      pos: _,
      ln: _,
      value: JValue::Array(VKind::Lit(params)),
    } = &func_list[1]
    else {
      return genErr!(
        "The second element of a lambda list must be an argument list",
        func_list[1].pos,
        func_list[1].ln,
        self.input_code
      );
    };
    for i in &func_list[2..] {
      self.eval(i, function)?;
    }
    Ok(Json {
      pos: 1,
      ln: 1,
      value: JValue::Function(VKind::Lit(params.clone())),
    })
  }
  fn begin(&mut self, args: &[Json], function: &mut String) -> JResult {
    self.validate(
      args.len() == 1,
      "begin".into(),
      "at least one".into(),
      args[0].pos,
      args[0].ln,
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
  fn setvar(&mut self, args: &[Json], function: &mut String) -> JResult {
    self.validate(
      args.len() != 3,
      "=".into(),
      "two".into(),
      args[0].pos,
      args[0].ln,
    )?;
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
                ln: args[0].ln,
                value: JValue::String(VKind::Var(n)),
              },
            );
          }
          _ => {
            return genErr!(
              "              Assignment to an unimplemented type",
              args[0].pos,
              args[0].ln,
              self.input_code
            );
          }
        }
      } else {
        self.vars.insert(var_name.clone(), value);
      }
      Ok(Json {
        pos: args[0].pos,
        ln: args[0].ln,
        value: JValue::Null,
      })
    } else {
      genErr!(
        "Variable names must be compile-time fixed strings",
        args[0].pos,
        args[0].ln,
        self.input_code
      )
    }
  }
  fn getvar(&mut self, args: &[Json], _: &mut String) -> JResult {
    self.validate(
      args.len() != 2,
      "$".into(),
      "one".into(),
      args[0].pos,
      args[0].ln,
    )?;
    if let JValue::String(VKind::Lit(var_name)) = &args[1].value {
      if let Some(value) = self.vars.get(var_name) {
        Ok(value.clone())
      } else {
        genErr!(
          &format!("Undefined variables: '{}'", var_name),
          args[0].pos,
          args[0].ln,
          self.input_code
        )
      }
    } else {
      genErr!(
        "Variable names must be compile-time fixed strings",
        args[0].pos,
        args[0].ln,
        self.input_code
      )
    }
  }
  fn plus(&mut self, args: &[Json], function: &mut String) -> JResult {
    self.validate(
      args.len() == 1,
      "+".into(),
      "at least one".into(),
      args[0].pos,
      args[0].ln,
    )?;
    let Ok(Json {
      pos: _,
      ln: _,
      value: JValue::Int(result),
    }) = self.eval(&args[1], function)
    else {
      return genErr!(
        "        '+' requires integer operands",
        args[0].pos,
        args[0].ln,
        self.input_code
      );
    };
    match result {
      VKind::Lit(l) => writeln!(function, "  mov rax, {}", l)?,
      VKind::Var(v) => writeln!(function, "  mov rax, QWORD PTR [rip + {}]", v)?,
    }
    for a in &args[2..args.len()] {
      let Ok(Json {
        pos: _,
        ln: _,
        value: JValue::Int(result),
      }) = self.eval(a, function)
      else {
        return genErr!(
          "          '+' requires integer operands",
          args[0].pos,
          args[0].ln,
          self.input_code
        );
      };
      match result {
        VKind::Lit(l) => writeln!(function, "  add rax, {}", l)?,
        VKind::Var(v) => writeln!(function, "  add rax, QWORD PTR [rip + {}]", v)?,
      }
    }
    let assign_name = self.get_name();
    writeln!(self.bss, "  .lcomm {}, 8", assign_name)?;
    writeln!(function, "  mov rax, QWORD PTR [rip + {}]", assign_name)?;
    Ok(Json {
      pos: args[0].pos,
      ln: args[0].ln,
      value: JValue::Int(VKind::Var(assign_name)),
    })
  }
  fn minus(&mut self, args: &[Json], function: &mut String) -> JResult {
    self.validate(
      args.len() == 1,
      "-".into(),
      "at least one".into(),
      args[0].pos,
      args[0].ln,
    )?;
    let Ok(Json {
      pos: _,
      ln: _,
      value: JValue::Int(result),
    }) = self.eval(&args[1], function)
    else {
      return genErr!(
        "        '-' requires integer operands",
        args[0].pos,
        args[0].ln,
        self.input_code
      );
    };
    match result {
      VKind::Lit(l) => writeln!(function, "  mov rax, {}", l)?,
      VKind::Var(v) => writeln!(function, "  mov rax, QWORD PTR [rip + {}]", v)?,
    }
    for a in &args[2..args.len()] {
      let Ok(Json {
        pos: _,
        ln: _,
        value: JValue::Int(result),
      }) = self.eval(a, function)
      else {
        return genErr!(
          "          '-' requires integer operands",
          args[0].pos,
          args[0].ln,
          self.input_code
        );
      };
      match result {
        VKind::Lit(l) => writeln!(function, "  sub rax, {}", l)?,
        VKind::Var(v) => writeln!(function, "  sub rax, QWORD PTR [rip + {}]", v)?,
      }
    }
    let assign_name = self.get_name();
    writeln!(self.bss, "  .lcomm {}, 8", assign_name)?;
    writeln!(function, "  mov QWORD PTR [rip + {}], rax", assign_name)?;
    Ok(Json {
      pos: args[0].pos,
      ln: args[0].ln,
      value: JValue::Int(VKind::Var(assign_name)),
    })
  }
  fn message(&mut self, args: &[Json], function: &mut String) -> JResult {
    self.validate(
      args.len() != 3,
      "message".into(),
      "two".into(),
      args[0].pos,
      args[0].ln,
    )?;
    let arg1 = self.eval(&args[1], function)?;
    self.extern_set.insert(String::from("MessageBoxA"));
    let title = match arg1 {
      Json {
        pos: _,
        ln: _,
        value: JValue::String(VKind::Lit(l)),
      } => {
        let mn = self.get_name();
        writeln!(self.data, "  {}: .string \"{}\"", mn, l)?;
        mn
      }
      Json {
        pos: _,
        ln: _,
        value: JValue::String(VKind::Var(v)),
      } => v,
      _ => {
        return genErr!(
          "The first argument of message must be a string",
          args[1].pos,
          args[1].ln,
          self.input_code
        );
      }
    };
    let arg2 = self.eval(&args[2], function)?;
    let msg = match arg2 {
      Json {
        pos: _,
        ln: _,
        value: JValue::String(VKind::Lit(l)),
      } => {
        let mn = self.get_name();
        writeln!(self.data, "  {}: .string \"{}\"", mn, l)?;
        mn
      }
      Json {
        pos: _,
        ln: _,
        value: JValue::String(VKind::Var(v)),
      } => v,
      _ => {
        return genErr!(
          "The second argument of message must be a string",
          args[2].pos,
          args[2].ln,
          self.input_code
        );
      }
    };
    let retcode = self.get_name();
    writeln!(self.bss, "  .lcomm {}, 8", retcode)?;
    writeln!(
      function,
      r#"  xor ecx, ecx
  lea rdx, QWORD PTR [rip + {}]
  lea r8, QWORD PTR [rip + {}]
  xor r9d, r9d
  call MessageBoxA
  test eax, eax
  jz display_error
  mov QWORD PTR [rip + {}], rax"#,
      msg, title, retcode
    )?;
    Ok(Json {
      pos: args[0].pos,
      ln: args[0].ln,
      value: JValue::Int(VKind::Var(retcode)),
    })
  }
}
fn error_exit(text: String) -> ! {
  let mut nu = String::new();
  eprint!("{text}\nPress Enter to exit:");
  let _ = io::stdin().read_line(&mut nu);
  std::process::exit(1)
}
#[allow(dead_code)]
impl Json {
  pub fn print_json(&self) -> fmt::Result {
    let mut output = String::new();
    if self.write_json(&mut output).is_ok() {
      println!("{}", output);
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
          out.write_str(": function)")
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
      .unwrap_or_else(|e| error_exit(format!("Couldn't print json: {}", e)));
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
    .args([&asm_file, "-o", &exe_file, "-nostartfiles"])
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
