//! Implementation of the parser inside the `Jsonpiler`.
use crate::{Bind, ErrOR, JObject, Json, JsonWithPos, Jsonpiler, Position, add, err};
use core::str;
/// Macro to return if the next character matches the expected one.
macro_rules! return_if {
  ($self: ident, $ch: expr, $start: expr, $val: expr) => {
    $self.skip_ws()?;
    if $self.advance_if($ch)? {
      $start.size = $self.pos.offset.saturating_sub($start.offset);
      return Ok(JsonWithPos { pos: $start, value: $val });
    }
  };
}
/// Gets slice of source code.
macro_rules! source_slice {
  ($self: ident) => {
    $self.source.get($self.pos.offset..).ok_or($self.fmt_err("Unexpected EOF.", &$self.pos))?
  };
}
impl Jsonpiler {
  /// Advances the position by `num` characters.
  fn advance(&mut self, num: usize) -> ErrOR<()> {
    self.pos.offset = add(self.pos.offset, num)?;
    Ok(())
  }
  /// Returns true if the next character matches the expected one.
  fn advance_if(&mut self, ch: u8) -> ErrOR<bool> {
    let flag = self.peek()? == ch;
    if flag {
      self.advance(1)?;
    }
    Ok(flag)
  }
  /// Checks if the next character in the input code matches the expected character.
  fn expect(&mut self, expected: u8) -> ErrOR<()> {
    let byte = self.peek()?;
    if byte == expected {
      self.advance(1)?;
      Ok(())
    } else {
      err!(self, "Expected byte '{expected}' not found.")
    }
  }
  /// Advances the position by `n` characters.
  fn inc(&mut self) -> ErrOR<()> {
    self.advance(1)
  }
  /// Advances the current position in the input code and returns the next character.
  fn next(&mut self) -> ErrOR<u8> {
    let byte = self.peek()?;
    self.advance(1)?;
    Ok(byte)
  }
  /// Parses the entire input code and returns the resulting `Json` representation.
  /// # Arguments
  /// * `code` - The input code to parse.
  /// # Returns
  /// * `Ok(Json)` - The parsed `Json` representation.
  /// * `Err(Box<dyn Error>)` - An error if the input code is invalid.
  /// # Errors
  /// * `Box<dyn Error>` - An error if the input code is invalid.
  pub(crate) fn parse(&mut self, code: &str) -> ErrOR<JsonWithPos> {
    self.source = code.as_bytes().to_vec();
    self.pos = Position { offset: 0, line: 1, size: 1 };
    let result = self.parse_value()?;
    if self.pos.offset == self.source.len() {
      Ok(result)
    } else {
      err!(self, "Unexpected trailing characters")
    }
  }
  /// Parses an array from the input code.
  fn parse_array(&mut self) -> ErrOR<JsonWithPos> {
    let mut start = self.pos.clone();
    let mut array = vec![];
    self.expect(b'[')?;
    return_if!(self, b']', start, Json::Array(Bind::Lit(array)));
    loop {
      array.push(self.parse_value()?);
      return_if!(self, b']', start, Json::Array(Bind::Lit(array)));
      self.expect(b',')?;
    }
  }
  /// Parses a specific name and returns a `Json` object with the associated value.
  fn parse_keyword(&mut self, name: &str, val: Json) -> ErrOR<JsonWithPos> {
    if source_slice!(self).starts_with(name.as_bytes()) {
      let mut start = self.pos.clone();
      self.advance(name.len())?;
      start.size = name.len();
      Ok(JsonWithPos { pos: start, value: val })
    } else {
      err!(self, "Failed to parse '{name}'")
    }
  }
  /// Parses a number (integer or float) from the input code.
  fn parse_number(&mut self) -> ErrOR<JsonWithPos> {
    fn push_number(parser: &mut Jsonpiler, num_str: &mut Vec<u8>, err: &str) -> ErrOR<()> {
      if !parser.peek()?.is_ascii_digit() {
        return err!(parser, &parser.pos, "{err}");
      }
      loop {
        let ch = parser.peek()?;
        if !ch.is_ascii_digit() {
          break Ok(());
        }
        num_str.push(ch);
        parser.inc()?;
      }
    }
    let mut start = self.pos.clone();
    let mut num_str = vec![];
    let mut has_decimal = false;
    let mut has_exponent = false;
    if self.advance_if(b'-')? {
      num_str.push(b'-');
    }
    if self.advance_if(b'0')? {
      num_str.push(b'0');
      if self.peek()?.is_ascii_digit() {
        return err!(self, "Leading zeros are not allowed in numbers");
      }
    } else {
      push_number(self, &mut num_str, "Invalid number format.")?;
    }
    if matches!(self.peek()?, b'.') {
      has_decimal = true;
      num_str.push(self.next()?);
      push_number(self, &mut num_str, "A digit is required after the decimal point.")?;
    }
    if matches!(self.peek()?, b'e' | b'E') {
      has_exponent = true;
      num_str.push(self.next()?);
      if matches!(self.peek()?, b'+' | b'-') {
        num_str.push(self.next()?);
      }
      push_number(self, &mut num_str, "A digit is required after the exponent notation.")?;
    }
    start.size = num_str.len();
    if has_decimal || has_exponent {
      str::from_utf8(&num_str)?.parse::<f64>().map_or_else(
        |_| err!(self, "Invalid numeric value."),
        |float| Ok(JsonWithPos { pos: start, value: Json::Float(Bind::Lit(float)) }),
      )
    } else {
      str::from_utf8(&num_str)?.parse::<i64>().map_or_else(
        |_| err!(self, "Invalid numeric value."),
        |int| Ok(JsonWithPos { pos: start, value: Json::Int(Bind::Lit(int)) }),
      )
    }
  }
  /// Parses an object from the input code.
  fn parse_object(&mut self) -> ErrOR<JsonWithPos> {
    let mut start = self.pos.clone();
    let mut object = JObject::default();
    self.expect(b'{')?;
    return_if!(self, b'}', start, Json::Object(Bind::Lit(object)));
    loop {
      let key = self.parse_value()?;
      let Json::String(Bind::Lit(string)) = key.value else {
        return err!(self, &key.pos, "Keys must be strings.");
      };
      self.expect(b':')?;
      let value = self.parse_value()?;
      object.insert(string, value);
      return_if!(self, b'}', start, Json::Object(Bind::Lit(object)));
      self.expect(b',')?;
    }
  }
  /// Parses a string from the input code.
  fn parse_string(&mut self) -> ErrOR<JsonWithPos> {
    let start = self.pos.clone();
    self.expect(b'"')?;
    let mut result = String::new();
    while let Some(&byte) = self.source.get(self.pos.offset) {
      match byte {
        b'"' => {
          self.advance(1)?;
          let size = self.pos.offset.saturating_sub(start.offset);
          return Ok(JsonWithPos {
            pos: Position { size, ..start },
            value: Json::String(Bind::Lit(result)),
          });
        }
        b'\\' => {
          self.advance(1)?;
          let esc = self.next()?;
          match esc {
            b'n' => result.push('\n'),
            b't' => result.push('\t'),
            b'r' => result.push('\r'),
            b'b' => result.push('\x08'),
            b'f' => result.push('\x0C'),
            b'"' => result.push('"'),
            b'\\' => result.push('\\'),
            b'/' => result.push('/'),
            b'u' => {
              let mut hex = String::new();
              for _ in 0u8..4u8 {
                let ch = self.next()?;
                if !ch.is_ascii_hexdigit() {
                  return err!(self, "Invalid hex digit.");
                }
                hex.push(char::from(ch));
              }
              let cp = u32::from_str_radix(&hex, 16)
                .map_err(|err| self.fmt_err(&format!("Invalid code point: {err}"), &self.pos))?;
              if (0xD800..=0xDFFF).contains(&cp) {
                return err!(self, "Invalid surrogate pair in unicode.");
              }
              let Some(ch) = char::from_u32(cp) else {
                return err!(self, "Invalid unicode.");
              };
              result.push(ch);
            }
            _ => return err!(self, "Invalid escape sequence."),
          }
        }
        ctrl if ctrl < 0x20 => return err!(self, "Invalid control character."),
        _ => {
          let string = str::from_utf8(
            self.source.get(self.pos.offset..).ok_or(self.fmt_err("Unexpected EOF.", &self.pos))?,
          );
          match string {
            Ok(st) => {
              let ch = st.chars().next().ok_or(self.fmt_err("Unexpected EOF.", &self.pos))?;
              result.push(ch);
              self.advance(ch.len_utf8())?;
            }
            Err(_) => return err!(self, "Invalid UTF-8 in string."),
          }
        }
      }
    }
    err!(self, "Unterminated string.")
  }
  /// Parses a value from the input code.
  fn parse_value(&mut self) -> ErrOR<JsonWithPos> {
    self.skip_ws()?;
    let result = match self.peek()? {
      b'"' => self.parse_string(),
      b'{' => self.parse_object(),
      b'[' => self.parse_array(),
      b't' => self.parse_keyword("true", Json::LBool(true)),
      b'f' => self.parse_keyword("false", Json::LBool(false)),
      b'n' => self.parse_keyword("null", Json::Null),
      b'0'..=b'9' | b'-' => self.parse_number(),
      _ => err!(self, "This is not a json value."),
    };
    self.skip_ws()?;
    result
  }
  /// Peek next character.
  fn peek(&self) -> ErrOR<u8> {
    self
      .source
      .get(self.pos.offset)
      .copied()
      .ok_or(self.fmt_err("Unexpected EOF.", &self.pos).into())
  }
  /// Skips whitespace characters in the input code.
  fn skip_ws(&mut self) -> ErrOR<()> {
    while let Ok(ch) = self.peek() {
      if !ch.is_ascii_whitespace() {
        break;
      }
      if ch == b'\n' {
        self.pos.line = add(self.pos.line, 1)?;
      }
      self.inc()?;
    }
    Ok(())
  }
}
