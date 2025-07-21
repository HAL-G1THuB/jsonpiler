//! Implementation of the parser inside the `Jsonpiler`.
use crate::{Bind::Lit, ErrOR, JObject, Json, JsonWithPos, Jsonpiler, Position, add, err};
use core::str;
/// Macro to return if the next character matches the expected one.
macro_rules! return_if {
  ($self: ident, $ch: expr, $pos: expr, $value: expr) => {
    if $self.peek()? == $ch {
      $self.inc()?;
      $pos.size = $self.pos.offset.saturating_sub($pos.offset);
      return Ok(JsonWithPos { pos: $pos, value: $value });
    }
  };
}
impl Jsonpiler {
  /// Advances the position by `num` characters.
  fn advance(&mut self, num: usize) -> ErrOR<()> {
    self.pos.offset = add(self.pos.offset, num)?;
    Ok(())
  }
  /// Checks if the next character in the input code matches the expected character.
  fn expect(&mut self, expected: u8) -> ErrOR<()> {
    let byte = self.peek()?;
    if byte == expected {
      self.inc()?;
      Ok(())
    } else {
      err!(self, self.pos, "Expected character '{}' not found.", char::from(expected))
    }
  }
  /// Advances the position by `n` characters.
  fn inc(&mut self) -> ErrOR<()> {
    self.advance(1)
  }
  /// Advances the current position in the input code and returns the next character.
  fn next(&mut self) -> ErrOR<u8> {
    let byte = self.peek()?;
    self.inc()?;
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
  pub(crate) fn parse(&mut self, code: String) -> ErrOR<JsonWithPos> {
    self.source = code.into_bytes();
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
    self.skip_ws()?;
    return_if!(self, b']', start, Json::Array(Lit(array)));
    loop {
      array.push(self.parse_value()?);
      return_if!(self, b']', start, Json::Array(Lit(array)));
      self.expect(b',')?;
    }
  }
  /// Parses a specific name and returns a `Json` object with the associated value.
  fn parse_keyword(&mut self, name: &[u8], value: Json) -> ErrOR<JsonWithPos> {
    if self
      .source
      .get(self.pos.offset..)
      .ok_or(self.fmt_err("Unexpected EOF.", &self.pos))?
      .starts_with(name)
    {
      let pos = Position { size: name.len(), ..self.pos };
      self.advance(name.len())?;
      Ok(JsonWithPos { pos, value })
    } else {
      err!(self, self.pos, "Failed to parse '{}'", String::from_utf8_lossy(name))
    }
  }
  /// Parses a number (integer or float) from the input code.
  fn parse_number(&mut self) -> ErrOR<JsonWithPos> {
    fn push_number(parser: &mut Jsonpiler, num_str: &mut Vec<u8>) -> ErrOR<()> {
      loop {
        let ch = parser.peek()?;
        if !ch.is_ascii_digit() {
          break Ok(());
        }
        num_str.push(ch);
        parser.inc()?;
      }
    }
    let mut pos = self.pos.clone();
    let mut num_str = vec![];
    let mut is_float = false;
    if self.peek()? == b'-' {
      self.inc()?;
      num_str.push(b'-');
    }
    if self.peek()? == b'0' {
      self.inc()?;
      num_str.push(b'0');
      if self.peek()?.is_ascii_digit() {
        return err!(self, "Leading zeros are not allowed in numbers");
      }
    } else {
      push_number(self, &mut num_str)?;
    }
    if self.peek()? == b'.' {
      is_float = true;
      self.inc()?;
      num_str.push(b'.');
      push_number(self, &mut num_str)?;
    }
    if matches!(self.peek()?, b'e' | b'E') {
      is_float = true;
      self.inc()?;
      num_str.push(b'e');
      let maybe_sign = self.peek()?;
      if matches!(maybe_sign, b'-' | b'+') {
        self.inc()?;
        num_str.push(maybe_sign);
      }
      push_number(self, &mut num_str)?;
    }
    pos.size = num_str.len();
    if is_float {
      str::from_utf8(&num_str)?.parse::<f64>().map_or_else(
        |_| err!(self, "Invalid numeric value."),
        |float| Ok(JsonWithPos { pos, value: Json::Float(Lit(float)) }),
      )
    } else {
      str::from_utf8(&num_str)?.parse::<i64>().map_or_else(
        |_| err!(self, "Invalid numeric value."),
        |int| Ok(JsonWithPos { pos, value: Json::Int(Lit(int)) }),
      )
    }
  }
  /// Parses an object from the input code.
  fn parse_object(&mut self) -> ErrOR<JsonWithPos> {
    let mut pos = self.pos.clone();
    let mut object = JObject::default();
    self.expect(b'{')?;
    self.skip_ws()?;
    return_if!(self, b'}', pos, Json::Object(Lit(object)));
    loop {
      let key = self.parse_string()?;
      let Json::String(Lit(string)) = key.value else {
        return err!(self, &key.pos, "Keys must be strings.");
      };
      self.skip_ws()?;
      self.expect(b':')?;
      object.insert(string, self.parse_value()?);
      return_if!(self, b'}', pos, Json::Object(Lit(object)));
      self.expect(b',')?;
      self.skip_ws()?;
    }
  }
  /// Parses a string from the input code.
  fn parse_string(&mut self) -> ErrOR<JsonWithPos> {
    let mut pos = self.pos.clone();
    self.expect(b'"')?;
    let mut result = String::new();
    while let Ok(byte) = self.next() {
      match byte {
        b'"' => {
          pos.size = self.pos.offset.saturating_sub(pos.offset);
          return Ok(JsonWithPos { pos, value: Json::String(Lit(result)) });
        }
        b'\\' => {
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
              let Ok(cp) = u32::from_str_radix(&hex, 16) else {
                return err!(self, "Invalid code point");
              };
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
        0x00..=0x1F => return err!(self, "Invalid control character."),
        0x20..=0x7F => result.push(char::from(byte)),
        _ => {
          let dec_len = match byte {
            0xC2..=0xDF => 1,
            0xE0..=0xEF => 2,
            0xF0..=0xF4 => 3,
            _ => return err!(self, "Invalid UTF-8 start byte in string."),
          };
          let Some(slice) =
            self.source.get(self.pos.offset.saturating_sub(1)..add(self.pos.offset, dec_len)?)
          else {
            break;
          };
          match str::from_utf8(slice) {
            Ok(string) => {
              result.push_str(string);
              self.advance(dec_len)?;
            }
            Err(_) => return err!(self, "Invalid UTF-8 continuation bytes in string."),
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
      b't' => self.parse_keyword(b"true", Json::LBool(true)),
      b'f' => self.parse_keyword(b"false", Json::LBool(false)),
      b'n' => self.parse_keyword(b"null", Json::Null),
      b'0'..=b'9' | b'-' => self.parse_number(),
      _ => err!(self, "Expected a json value, but an unknown value was passed."),
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
