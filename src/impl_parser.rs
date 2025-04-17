//! Implementation of the parser inside the `Jsonpiler`.
use super::{ErrorInfo, JObject, JResult, JValue, Json, Jsonpiler};
use core::{char::from_u32, error::Error};
impl Jsonpiler {
  /// Checks if the next character in the input code matches the expected character.
  fn expect(&mut self, expected: char) -> Result<(), Box<dyn Error>> {
    let ch = self.peek()?;
    if ch == expected {
      self.info.pos = self.step(ch.len_utf8())?;
      Ok(())
    } else {
      Err(self.fmt_err(&format!("Expected character '{expected}' not found."), &self.info).into())
    }
  }
  /// Advances the current position in the input code and returns the next character.
  fn next(&mut self) -> Result<char, Box<dyn Error>> {
    let ch = self.peek()?;
    self.info.pos = self.step(ch.len_utf8())?;
    Ok(ch)
  }
  /// Parses the entire input code and returns the resulting `Json` object.
  /// # Arguments
  /// * `code` - The input code to parse.
  /// # Returns
  /// * `Ok(Json)` - The parsed `Json` object.
  /// * `Err(Box<dyn Error>)` - An error if the input code is invalid.
  /// # Errors
  /// * `Box<dyn Error>` - An error if the input code is invalid.
  #[inline]
  pub(crate) fn parse(&mut self, code: String) -> JResult {
    self.source = code;
    self.info = ErrorInfo { pos: 0, line: 1 };
    let result = self.parse_value()?;
    if self.info.pos == self.source.len() {
      Ok(result)
    } else {
      Err(self.fmt_err("Unexpected trailing characters", &self.info).into())
    }
  }
  /// Parses an array from the input code.
  fn parse_array(&mut self) -> JResult {
    let start = self.info.clone();
    let mut array = vec![];
    self.expect('[')?;
    self.skip_ws()?;
    if self.source[self.info.pos..].starts_with(']') {
      self.info.pos = self.step(1)?;
      return Ok(Json { info: start, value: JValue::Array(array) });
    }
    loop {
      array.push(self.parse_value()?);
      if self.source[self.info.pos..].starts_with(']') {
        self.info.pos = self.step(1)?;
        return Ok(Json { info: start, value: JValue::Array(array) });
      }
      self.expect(',')?;
    }
  }
  /// Parses a specific name and returns a `Json` object with the associated value.
  fn parse_name(&mut self, name: &str, val: JValue) -> JResult {
    if self.source[self.info.pos..].starts_with(name) {
      let start = self.info.clone();
      self.info.pos = self.step(name.len())?;
      Ok(Json { info: start, value: val })
    } else {
      Err(self.fmt_err(&format!("Failed to parse '{name}'"), &self.info).into())
    }
  }
  /// Parses a number (integer or float) from the input code.
  fn parse_number(&mut self) -> JResult {
    let start = self.info.clone();
    let mut num_str = String::new();
    let mut has_decimal = false;
    let mut has_exponent = false;
    if self.source[self.info.pos..].starts_with('-') {
      num_str.push('-');
      self.next()?;
    }
    let num_char = self.peek()?;
    match num_char {
      '0' => {
        num_str.push('0');
        self.next()?;
        if matches!(self.peek()?, ch if ch.is_ascii_digit()) {
          return Err(self.fmt_err("Leading zeros are not allowed in numbers", &self.info).into());
        }
      }
      '1'..='9' => loop {
        let ch = self.peek()?;
        if ch.is_ascii_digit() {
          num_str.push(ch);
          self.next()?;
        } else {
          break;
        }
      },
      _ => return Err(self.fmt_err("Invalid number format.", &self.info).into()),
    }
    if matches!(self.peek()?, '.') {
      has_decimal = true;
      num_str.push('.');
      self.next()?;
      if !matches!(self.peek()?, ch if ch.is_ascii_digit()) {
        return Err(
          self.fmt_err("A digit is required after the decimal point.", &self.info).into(),
        );
      }
      loop {
        let ch = self.peek()?;
        if ch.is_ascii_digit() {
          num_str.push(ch);
          self.next()?;
        } else {
          break;
        }
      }
    }
    if matches!(self.peek()?, 'e' | 'E') {
      has_exponent = true;
      num_str.push('e');
      self.next()?;
      if matches!(self.peek()?, '+' | '-') {
        num_str.push(self.next()?);
      }
      if !matches!(self.peek()?, ch if ch.is_ascii_digit()) {
        return Err(self.fmt_err("A digit is required in the exponent part.", &self.info).into());
      }
      loop {
        let ch = self.peek()?;
        if ch.is_ascii_digit() {
          num_str.push(ch);
          self.next()?;
        } else {
          break;
        }
      }
    }
    if has_decimal || has_exponent {
      num_str.parse::<f64>().map_or_else(
        |_| Err(self.fmt_err("Invalid numeric value.", &self.info).into()),
        |float_val| Ok(Json { info: start, value: JValue::Float(float_val) }),
      )
    } else {
      num_str.parse::<i64>().map_or_else(
        |_| Err(self.fmt_err("Invalid numeric value.", &self.info).into()),
        |int_val| Ok(Json { info: start, value: JValue::Int(int_val) }),
      )
    }
  }
  /// Parses an object from the input code.
  fn parse_object(&mut self) -> JResult {
    let start = self.info.clone();
    let mut object = JObject::default();
    self.expect('{')?;
    self.skip_ws()?;
    if self.source[self.info.pos..].starts_with('}') {
      self.info.pos = self.step(1)?;
      return Ok(Json { info: start, value: JValue::Object(object) });
    }
    loop {
      let key = self.parse_value()?;
      let JValue::String(string) = key.value else {
        return Err(self.fmt_err("Keys must be strings.", &key.info).into());
      };
      self.expect(':')?;
      let value = self.parse_value()?;
      object.insert(string, value);
      if self.source[self.info.pos..].starts_with('}') {
        self.info.pos = self.step(1)?;
        return Ok(Json { info: start, value: JValue::Object(object) });
      }
      self.expect(',')?;
      self.info.pos = self.step(1)?;
    }
  }
  /// Parses a string from the input code.
  fn parse_string(&mut self) -> JResult {
    self.expect('\"')?;
    let start_info = self.info.clone();
    let mut result = String::new();
    while let Ok(ch) = self.next() {
      match ch {
        '\"' => return Ok(Json { info: start_info, value: JValue::String(result) }),
        '\n' => return Err(self.fmt_err("Invalid line breaks in strings.", &self.info).into()),
        '\\' => {
          let escaped = self.next()?;
          match escaped {
            'n' => result.push('\n'),
            't' => result.push('\t'),
            'r' => result.push('\r'),
            'b' => result.push('\x08'),
            'f' => result.push('\x0C'),
            'u' => {
              let mut hex = String::new();
              for _ in 0u32..4u32 {
                let cha = self.next()?;
                if !cha.is_ascii_hexdigit() {
                  return Err(self.fmt_err("Invalid hex digit.", &self.info).into());
                }
                hex.push(cha);
              }
              let cp = u32::from_str_radix(&hex, 16)
                .map_err(|err_msg| format!("Invalid code point: {err_msg}"))?;
              if (0xD800..=0xDFFF).contains(&cp) {
                return Err(self.fmt_err("Invalid unicode.", &self.info).into());
              }
              let Some(u32_cp) = from_u32(cp) else {
                return Err(self.fmt_err("Invalid unicode.", &self.info).into());
              };
              result.push(u32_cp);
            }
            esc_ch @ ('\\' | '"' | '/') => result.push(esc_ch),
            _ => return Err(self.fmt_err("Invalid escape sequence.", &self.info).into()),
          }
        }
        cha if cha < '\u{20}' => {
          return Err(self.fmt_err("Invalid control character.", &self.info).into());
        }
        cha => result.push(cha),
      }
    }
    Err(self.fmt_err("String is not properly terminated.", &self.info).into())
  }
  /// Parses a value from the input code.
  fn parse_value(&mut self) -> JResult {
    self.skip_ws()?;
    let result = match self.peek()? {
      '"' => self.parse_string(),
      '{' => self.parse_object(),
      '[' => self.parse_array(),
      't' => self.parse_name("true", JValue::Bool(true)),
      'f' => self.parse_name("false", JValue::Bool(false)),
      'n' => self.parse_name("null", JValue::Null),
      '0'..='9' | '-' => self.parse_number(),
      _ => Err(self.fmt_err("This is not a json value.", &self.info).into()),
    };
    self.skip_ws()?;
    result
  }
  /// Peek next character.
  fn peek(&mut self) -> Result<char, String> {
    self.source[self.info.pos..]
      .chars()
      .next()
      .ok_or(self.fmt_err("Unexpected end of text.", &self.info))
  }
  /// Skips whitespace characters in the input code.
  fn skip_ws(&mut self) -> Result<(), Box<dyn Error>> {
    while let Ok(ch) = self.peek() {
      match ch {
        ws if ws.is_whitespace() => {
          if ws == '\n' {
            self.info.line =
              self.info.line.checked_add(1).ok_or(self.fmt_err("LineOverflowError", &self.info))?;
          }
          self.info.pos = self.step(ws.len_utf8())?;
        }
        _ => break,
      }
    }
    Ok(())
  }
  /// Advance pos.
  fn step(&self, num: usize) -> Result<usize, String> {
    self.info.pos.checked_add(num).ok_or(self.fmt_err("PosOverflowError", &self.info))
  }
}
