//! Implementation of the parser inside the `Jsonpiler`.
use super::{ErrOR, JObject, JResult, Json, JsonWithPos, Jsonpiler, Position, err};
/// Macro to return if the next character matches the expected one.
macro_rules! return_if {
  ($self: ident, $ch: expr, $start: expr, $val: expr) => {
    $self.skip_ws()?;
    if $self.advance_if($ch)? {
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
    self.pos.offset =
      self.pos.offset.checked_add(num).ok_or(self.fmt_err("Pos Overflow", &self.pos))?;
    Ok(())
  }
  /// Returns true if the next character matches the expected one.
  fn advance_if(&mut self, ch: char) -> ErrOR<bool> {
    let flag = self.peek()? == ch;
    if flag {
      self.advance(ch.len_utf8())?;
    }
    Ok(flag)
  }
  /// Checks if the next character in the input code matches the expected character.
  fn expect(&mut self, expected: char) -> ErrOR<()> {
    let ch = self.peek()?;
    if ch == expected {
      self.advance(ch.len_utf8())?;
      Ok(())
    } else {
      err!(self, "Expected character '{expected}' not found.")
    }
  }
  /// Advances the position by `n` characters.
  fn inc(&mut self) -> ErrOR<()> {
    self.advance(1)
  }
  /// Advances the current position in the input code and returns the next character.
  fn next(&mut self) -> ErrOR<char> {
    let ch = self.peek()?;
    self.advance(ch.len_utf8())?;
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
  pub(crate) fn parse(&mut self, code: String) -> JResult {
    self.source = code;
    self.pos = Position { offset: 0, line: 1 };
    let result = self.parse_value()?;
    if self.pos.offset == self.source.len() {
      Ok(result)
    } else {
      err!(self, "Unexpected trailing characters")
    }
  }
  /// Parses an array from the input code.
  fn parse_array(&mut self) -> JResult {
    let start = self.pos.clone();
    let mut array = vec![];
    self.expect('[')?;
    return_if!(self, ']', start, Json::LArray(array));
    loop {
      array.push(self.parse_value()?);
      return_if!(self, ']', start, Json::LArray(array));
      self.expect(',')?;
    }
  }
  /// Parses a specific name and returns a `Json` object with the associated value.
  fn parse_name(&mut self, name: &str, val: Json) -> JResult {
    if source_slice!(self).starts_with(name) {
      let start = self.pos.clone();
      self.advance(name.len())?;
      Ok(JsonWithPos { pos: start, value: val })
    } else {
      err!(self, "Failed to parse '{name}'")
    }
  }
  /// Parses a number (integer or float) from the input code.
  fn parse_number(&mut self) -> JResult {
    fn push_number(parser: &mut Jsonpiler, num_str: &mut String, err: &str) -> ErrOR<()> {
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
    let start = self.pos.clone();
    let mut num_str = String::new();
    let mut has_decimal = false;
    let mut has_exponent = false;
    if self.advance_if('-')? {
      num_str.push('-');
    }
    if self.advance_if('0')? {
      num_str.push('0');
      if self.peek()?.is_ascii_digit() {
        return err!(self, "Leading zeros are not allowed in numbers");
      }
    } else {
      push_number(self, &mut num_str, "Invalid number format.")?;
    }
    if matches!(self.peek()?, '.') {
      has_decimal = true;
      num_str.push(self.next()?);
      push_number(self, &mut num_str, "A digit is required after the decimal point.")?;
    }
    if matches!(self.peek()?, 'e' | 'E') {
      has_exponent = true;
      num_str.push(self.next()?);
      if matches!(self.peek()?, '+' | '-') {
        num_str.push(self.next()?);
      }
      push_number(self, &mut num_str, "A digit is required after the exponent notation.")?;
    }
    if has_decimal || has_exponent {
      num_str.parse::<f64>().map_or_else(
        |_| err!(self, "Invalid numeric value."),
        |float_val| Ok(JsonWithPos { pos: start, value: Json::LFloat(float_val) }),
      )
    } else {
      num_str.parse::<i64>().map_or_else(
        |_| err!(self, "Invalid numeric value."),
        |int_val| Ok(JsonWithPos { pos: start, value: Json::LInt(int_val) }),
      )
    }
  }
  /// Parses an object from the input code.
  fn parse_object(&mut self) -> JResult {
    let start = self.pos.clone();
    let mut object = JObject::default();
    self.expect('{')?;
    return_if!(self, '}', start, Json::LObject(object));
    loop {
      let key = self.parse_value()?;
      let Json::LString(string) = key.value else {
        return err!(self, &key.pos, "Keys must be strings.");
      };
      self.expect(':')?;
      let value = self.parse_value()?;
      object.insert(string, value);
      return_if!(self, '}', start, Json::LObject(object));
      self.expect(',')?;
    }
  }
  /// Parses a string from the input code.
  fn parse_string(&mut self) -> JResult {
    let start = self.pos.clone();
    self.expect('"')?;
    let mut result = String::new();
    let mut ch;
    loop {
      ch = self.next()?;
      match ch {
        '"' => return Ok(JsonWithPos { pos: start, value: Json::LString(result) }),
        '\n' => return err!(self, "Invalid line breaks in strings."),
        '\\' => match self.next()? {
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
                return err!(self, "Invalid hex digit.");
              }
              hex.push(cha);
            }
            let Ok(cp) = u32::from_str_radix(&hex, 16) else {
              return err!(self, "Invalid code point.");
            };
            if (0xD800..=0xDFFF).contains(&cp) {
              return err!(self, "Invalid surrogate pair in unicode.");
            }
            let Some(u32_cp) = char::from_u32(cp) else {
              return err!(self, "Invalid unicode.");
            };
            result.push(u32_cp);
          }
          esc_ch @ ('\\' | '"' | '/') => result.push(esc_ch),
          _ => return err!(self, "Invalid escape sequence."),
        },
        cha if cha < '\u{20}' => {
          return err!(self, "Invalid control character.");
        }
        cha => result.push(cha),
      }
    }
  }
  /// Parses a value from the input code.
  fn parse_value(&mut self) -> JResult {
    self.skip_ws()?;
    let result = match self.peek()? {
      '"' => self.parse_string(),
      '{' => self.parse_object(),
      '[' => self.parse_array(),
      't' => self.parse_name("true", Json::LBool(true)),
      'f' => self.parse_name("false", Json::LBool(false)),
      'n' => self.parse_name("null", Json::Null),
      '0'..='9' | '-' => self.parse_number(),
      _ => err!(self, "This is not a json value."),
    };
    self.skip_ws()?;
    result
  }
  /// Peek next character.
  fn peek(&self) -> ErrOR<char> {
    source_slice!(self).chars().next().ok_or(self.fmt_err("Unexpected EOF.", &self.pos).into())
  }
  /// Skips whitespace characters in the input code.
  fn skip_ws(&mut self) -> ErrOR<()> {
    while let Ok(ch) = self.peek() {
      if !ch.is_ascii_whitespace() {
        break;
      }
      if ch == '\n' {
        self.pos.line =
          self.pos.line.checked_add(1).ok_or(self.fmt_err("Line Overflow", &self.pos))?;
      }
      self.inc()?;
    }
    Ok(())
  }
}
