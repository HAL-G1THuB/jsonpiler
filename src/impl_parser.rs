//! Implementation of the parser inside the `Jsompiler`.
use super::{JResult, JValue, Jsompiler, Json, ParseInfo, utility::format_err};
use core::{char::from_u32, error::Error};
use std::collections::HashMap;
impl Jsompiler {
  /// Create parse error.
  fn err_parse(&self, text: &str) -> JResult {
    Err(format_err(text, self.info.pos, self.info.line, &self.source).into())
  }
  /// Checks if the next character in the input code matches the expected character.
  fn expect(&mut self, expected: char) -> Result<(), Box<dyn Error>> {
    if self.source[self.info.pos..].starts_with(expected) {
      self.next()?;
      Ok(())
    } else {
      Err(
        format_err(
          &format!("Expected character '{expected}' not found."),
          self.info.pos,
          self.info.line,
          &self.source,
        )
        .into(),
      )
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
    self.info = ParseInfo { pos: 0, line: 1 };
    let result = self.parse_value()?;
    self.skip_ws()?;
    if self.info.pos == self.source.len() {
      Ok(result)
    } else {
      self.err_parse("Unexpected trailing characters")
    }
  }
  /// Parses an array from the input code.
  fn parse_array(&mut self) -> JResult {
    let start_pos = self.info.pos;
    let start_ln = self.info.line;
    let mut array = vec![];
    self.expect('[')?;
    self.skip_ws()?;
    if self.source[self.info.pos..].starts_with(']') {
      self.info.pos = self.step(1)?;
      return Ok(Json { pos: start_pos, line: start_ln, value: JValue::Array(array) });
    }
    loop {
      array.push(self.parse_value()?);
      if self.source[self.info.pos..].starts_with(']') {
        self.info.pos = self.step(1)?;
        return Ok(Json { pos: start_pos, line: start_ln, value: JValue::Array(array) });
      } else if self.source[self.info.pos..].starts_with(',') {
        self.info.pos = self.step(1)?;
      } else {
        return self.err_parse("Invalid array separator");
      }
    }
  }
  /// Parses a specific name and returns a `Json` object with the associated value.
  fn parse_name(&mut self, name: &str, val: JValue) -> JResult {
    if self.source[self.info.pos..].starts_with(name) {
      let start = self.info.pos;
      self.info.pos = self.step(name.len())?;
      Ok(Json { pos: start, line: self.info.line, value: val })
    } else {
      self.err_parse(&format!("Failed to parse '{name}'"))
    }
  }
  /// Parses a number (integer or float) from the input code.
  fn parse_number(&mut self) -> JResult {
    let start = self.info.pos;
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
          return self.err_parse("Leading zeros are not allowed in numbers");
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
      _ => return self.err_parse("Invalid number format."),
    }
    if matches!(self.peek()?, '.') {
      has_decimal = true;
      num_str.push('.');
      self.next()?;
      if !matches!(self.peek()?, ch if ch.is_ascii_digit()) {
        return self.err_parse("A digit is required after the decimal point.");
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
        return self.err_parse("A digit is required in the exponent part.");
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
        |_| self.err_parse("Invalid numeric value."),
        |float_val| Ok(Json { pos: start, line: self.info.line, value: JValue::Float(float_val) }),
      )
    } else {
      num_str.parse::<i64>().map_or_else(
        |_| self.err_parse("Invalid numeric value."),
        |int_val| Ok(Json { pos: start, line: self.info.line, value: JValue::Int(int_val) }),
      )
    }
  }
  /// Parses an object from the input code.
  fn parse_object(&mut self) -> JResult {
    let start_pos = self.info.pos;
    let start_ln = self.info.line;
    let mut object = HashMap::new();
    self.expect('{')?;
    self.skip_ws()?;
    if self.source[self.info.pos..].starts_with('}') {
      self.info.pos = self.step(1)?;
      return Ok(Json { pos: start_pos, line: start_ln, value: JValue::Object(object) });
    }
    loop {
      let key = self.parse_value()?;
      let JValue::String(string) = key.value else {
        return Err(format_err("Keys must be strings.", key.pos, key.line, &self.source).into());
      };
      self.expect(':')?;
      let value = self.parse_value()?;
      object.insert(string, value);
      if self.source[self.info.pos..].starts_with('}') {
        self.info.pos = self.step(1)?;
        return Ok(Json { pos: start_pos, line: start_ln, value: JValue::Object(object) });
      }
      if self.source[self.info.pos..].starts_with(',') {
        self.info.pos = self.step(1)?;
      } else {
        return self.err_parse("Invalid object separator.");
      }
    }
  }
  /// Parses a string from the input code.
  fn parse_string(&mut self) -> JResult {
    if !self.source[self.info.pos..].starts_with('\"') {
      return self.err_parse("Missing opening quotation for string.");
    }
    let start = self.info.pos;
    self.info.pos = self.step(1)?;
    let mut result = String::new();
    while self.info.pos < self.source.len() {
      let ch = self.next()?;
      match ch {
        '\"' => {
          return Ok(Json { pos: start, line: self.info.line, value: JValue::String(result) });
        }
        '\n' => return self.err_parse("Invalid line breaks in strings."),
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
                  return self.err_parse("Invalid hex digit.");
                }
                hex.push(cha);
              }
              let cp = u32::from_str_radix(&hex, 16)
                .map_err(|err_msg| format!("Invalid code point: {err_msg}"))?;
              if (0xD800..=0xDFFF).contains(&cp) {
                return self.err_parse("Invalid unicode.");
              }
              result.push(from_u32(cp).ok_or("Invalid unicode.")?);
            }
            esc_ch @ ('\\' | '"' | '/') => result.push(esc_ch),
            _ => return self.err_parse("Invalid escape sequence."),
          }
        }
        cha if cha < '\u{20}' => return self.err_parse("Invalid control character."),
        cha => result.push(cha),
      }
    }
    self.err_parse("String is not properly terminated.")
  }
  /// Parses a value from the input code.
  fn parse_value(&mut self) -> JResult {
    self.skip_ws()?;
    if self.info.pos >= self.source.len() {
      return self.err_parse("Unexpected end of text.");
    }
    let result = match self.peek()? {
      '"' => self.parse_string(),
      '{' => self.parse_object(),
      '[' => self.parse_array(),
      't' => self.parse_name("true", JValue::Bool(true)),
      'f' => self.parse_name("false", JValue::Bool(false)),
      'n' => self.parse_name("null", JValue::Null),
      '0'..='9' | '-' => self.parse_number(),
      _ => self.err_parse("This is not a json value."),
    };
    self.skip_ws()?;
    result
  }
  /// Peek next character.
  fn peek(&mut self) -> Result<char, Box<dyn Error>> {
    self.source[self.info.pos..].chars().next().ok_or("Reached end of text.".into())
  }
  /// Skips whitespace characters in the input code.
  fn skip_ws(&mut self) -> Result<(), Box<dyn Error>> {
    loop {
      let Ok(ch) = self.peek() else { break Ok(()) };
      if ch.is_whitespace() {
        if ch == '\n' {
          self.info.line = self.info.line.checked_add(1).ok_or("LineOverflowError")?;
        }
        self.info.pos = self.step(ch.len_utf8())?;
      } else {
        break Ok(());
      }
    }
  }
  /// Advance pos.
  fn step(&mut self, num: usize) -> Result<usize, &str> {
    self.info.pos.checked_add(num).ok_or("PosOverflowError")
  }
}
