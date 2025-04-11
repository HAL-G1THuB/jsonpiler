//! Parser implementation.
use super::{JResult, JValue, Jsompiler, Json, utility::format_err};
use std::{char::from_u32, collections::HashMap, error::Error};
impl<'a> Jsompiler<'a> {
  /// Checks if the next character in the input code matches the expected character.
  fn expect(&mut self, expected: char) -> Result<(), Box<dyn Error>> {
    if self.input_code[self.pos..].starts_with(expected) {
      self.next()?;
      Ok(())
    } else {
      Err(
        format_err(
          &format!("Expected character '{expected}' not found."),
          self.pos,
          self.ln,
          self.input_code,
        )
        .into(),
      )
    }
  }
  /// Advances the current position in the input code and returns the next character.
  fn next(&mut self) -> Result<char, String> {
    let ch = self.input_code[self.pos..].chars().next().ok_or("Reached end of text")?;
    self.pos += ch.len_utf8();
    Ok(ch)
  }
  fn parse_err(&self, text: &str) -> JResult {
    Err(format_err(text, self.pos, self.ln, self.input_code).into())
  }
  /// Parses the entire input code and returns the resulting `Json` object.
  ///
  /// # Arguments
  ///
  /// * `code` - The input code to parse.
  ///
  /// # Returns
  ///
  /// * `Ok(Json)` - The parsed `Json` object.
  /// * `Err(Box<dyn Error>)` - An error if the input code is invalid.
  ///
  /// # Errors
  ///
  /// `JError` - Returns a `JError` structure containing an error message if an invalid syntax is passed.
  pub fn parse(&mut self, code: &'a str) -> JResult {
    self.input_code = code;
    self.pos = 0;
    self.ln = 1;
    let result = self.parse_value()?;
    self.skip_ws();
    if self.pos == self.input_code.len() {
      Ok(result)
    } else {
      self.parse_err("Unexpected trailing characters")
    }
  }
  /// Skips whitespace characters in the input code.
  fn skip_ws(&mut self) {
    while let Some(c) = self.input_code[self.pos..].chars().next() {
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
  /// Parses a specific name and returns a `Json` object with the associated value.
  fn parse_name(&mut self, n: &str, v: JValue) -> JResult {
    if self.input_code[self.pos..].starts_with(n) {
      let start = self.pos;
      self.pos += n.len();
      Ok(Json { pos: start, ln: self.ln, value: v })
    } else {
      self.parse_err(&format!("Failed to parse '{n}'"))
    }
  }
  /// Parses a number (integer or float) from the input code.
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
        while let Some(ch2) = self.input_code[self.pos..].chars().next() {
          if ch2.is_ascii_digit() {
            num_str.push(ch2);
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
        |int_val| Ok(Json { pos: start, ln: self.ln, value: JValue::Int(int_val) }),
      )
    } else {
      num_str.parse::<f64>().map_or_else(
        |_| self.parse_err("Invalid numeric value"),
        |float_val| Ok(Json { pos: start, ln: self.ln, value: JValue::Float(float_val) }),
      )
    }
  }
  /// Parses a string from the input code.
  fn parse_string(&mut self) -> JResult {
    if !self.input_code[self.pos..].starts_with('\"') {
      return self.parse_err("Missing opening quotation for string");
    }
    let start = self.pos;
    self.pos += 1;
    let mut result = String::new();
    while self.pos < self.input_code.len() {
      let ch = self.next()?;
      match ch {
        '\"' => {
          return Ok(Json { pos: start, ln: self.ln, value: JValue::String(result) });
        }
        '\n' => {
          self.parse_err("Invalid line breaks in strings")?;
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
              let cp = u32::from_str_radix(&hex, 16)
                .map_err(|err_msg| format!("Invalid code point: {err_msg}"))?;
              if (0xD800..=0xDFFF).contains(&cp) {
                return self.parse_err("Invalid unicode");
              }
              result.push(from_u32(cp).ok_or("Invalid unicode")?);
            }
            _ => {
              return self.parse_err("Invalid escape sequence");
            }
          }
        }
        cha if cha < '\u{20}' => {
          return self.parse_err("Invalid control character");
        }
        cha => result.push(cha),
      }
    }
    self.parse_err("String is not properly terminated")
  }
  /// Parses an array from the input code.
  fn parse_array(&mut self) -> JResult {
    let start_pos = self.pos;
    let start_ln = self.ln;
    let mut array = Vec::new();
    self.expect('[')?;
    self.skip_ws();
    if self.input_code[self.pos..].starts_with(']') {
      self.pos += 1;
      return Ok(Json { pos: start_pos, ln: start_ln, value: JValue::Array(array) });
    }
    loop {
      array.push(self.parse_value()?);
      if self.input_code[self.pos..].starts_with(']') {
        self.pos += 1;
        return Ok(Json { pos: start_pos, ln: start_ln, value: JValue::Array(array) });
      } else if self.input_code[self.pos..].starts_with(',') {
        self.pos += 1;
      } else {
        return self.parse_err("Invalid array separator");
      }
    }
  }
  /// Parses an object from the input code.
  fn parse_object(&mut self) -> JResult {
    let start_pos = self.pos;
    let start_ln = self.ln;
    let mut object = HashMap::new();
    self.expect('{')?;
    self.skip_ws();
    if self.input_code[self.pos..].starts_with('}') {
      self.pos += 1;
      return Ok(Json { pos: start_pos, ln: start_ln, value: JValue::Object(object) });
    }
    loop {
      let key = self.parse_value()?;
      let JValue::String(string) = key.value else {
        return Err(format_err("Keys must be strings", key.pos, key.ln, self.input_code).into());
      };
      self.expect(':')?;
      let value = self.parse_value()?;
      object.insert(string, value);
      if self.input_code[self.pos..].starts_with('}') {
        self.pos += 1;
        return Ok(Json { pos: start_pos, ln: start_ln, value: JValue::Object(object) });
      }
      if self.input_code[self.pos..].starts_with(',') {
        self.pos += 1;
      } else {
        return self.parse_err("Invalid object separator");
      }
    }
  }
  /// Parses a value from the input code.
  fn parse_value(&mut self) -> JResult {
    self.skip_ws();
    if self.pos >= self.input_code.len() {
      return self.parse_err("Unexpected end of text");
    }
    let result = match self.input_code[self.pos..].chars().next() {
      Some('"') => self.parse_string(),
      Some('{') => self.parse_object(),
      Some('[') => self.parse_array(),
      Some('t') => self.parse_name("true", JValue::Bool(true)),
      Some('f') => self.parse_name("false", JValue::Bool(false)),
      Some('n') => self.parse_name("null", JValue::Null),
      _ => self.parse_number(),
    };
    self.skip_ws();
    result
  }
}
