//! Implementation of the parser inside the `Jsonpiler`.
use super::{ErrOR, ErrorInfo, JObject, JResult, JValue, Json, Jsonpiler};
/// Generate an error.
macro_rules! err {
  ($self:ident, $($arg: tt)*) => {
    Err($self.fmt_err(&format!($($arg)*), &$self.info).into())
  };
}
impl Jsonpiler {
  /// Advances the position by `num` characters.
  fn advance(&mut self, n: usize) -> ErrOR<()> {
    self.index = self.index.checked_add(n).ok_or(self.fmt_err("IndexOverflowError", &self.info))?;
    self.info.pos = self.indices.get(self.index).map_or(self.source.len(), |&(i, _)| i);
    Ok(())
  }
  /// Checks if the next character in the input code matches the expected character.
  fn expect(&mut self, expected: char) -> ErrOR<()> {
    let ch = self.peek()?;
    if ch == expected {
      self.inc()?;
      Ok(())
    } else {
      err!(self, "Expected character '{expected}' not found.")
    }
  }
  /// Advances the position by `n` characters.
  fn inc(&mut self) -> ErrOR<()> {
    self.advance(1)
  }
  /// Returns true if the next character matches the expected one.
  fn inc_if(&mut self, ch: char) -> ErrOR<bool> {
    let flag = self.peek()? == ch;
    if flag {
      self.inc()?;
    }
    Ok(flag)
  }
  /// Advances the current position in the input code and returns the next character.
  fn next(&mut self) -> ErrOR<char> {
    let ch = self.peek()?;
    self.inc()?;
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
    self.indices = self.source.char_indices().collect();
    self.index = 0;
    self.info = ErrorInfo { pos: 0, line: 1 };
    let result = self.parse_value()?;
    if self.info.pos == self.source.len() {
      self.indices.clear();
      Ok(result)
    } else {
      err!(self, "Unexpected trailing characters")
    }
  }
  /// Parses an array from the input code.
  fn parse_array(&mut self) -> JResult {
    let start = self.info.clone();
    let mut array = vec![];
    self.expect('[')?;
    self.skip_ws()?;
    if self.inc_if(']')? {
      return Ok(Json { info: start, value: JValue::LArray(array) });
    }
    loop {
      array.push(self.parse_value()?);
      if self.inc_if(']')? {
        return Ok(Json { info: start, value: JValue::LArray(array) });
      }
      self.expect(',')?;
    }
  }
  /// Parses a specific name and returns a `Json` object with the associated value.
  fn parse_name(&mut self, name: &str, val: JValue) -> JResult {
    if self
      .source
      .get(self.info.pos..)
      .ok_or(self.fmt_err("Unexpected end of text.", &self.info))?
      .starts_with(name)
    {
      let start = self.info.clone();
      self.advance(name.len())?;
      Ok(Json { info: start, value: val })
    } else {
      err!(self, "Failed to parse `{name}`")
    }
  }
  /// Parses a number (integer or float) from the input code.
  fn parse_number(&mut self) -> JResult {
    fn push_number(parser: &mut Jsonpiler, num_str: &mut String, error: &str) -> ErrOR<()> {
      if !matches!(parser.peek()?, ch if ch.is_ascii_digit()) {
        return Err(parser.fmt_err(error, &parser.info).into());
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
    let start = self.info.clone();
    let mut num_str = String::new();
    let mut has_decimal = false;
    let mut has_exponent = false;
    if self.inc_if('-')? {
      num_str.push('-');
    }
    match self.peek()? {
      '0' => {
        num_str.push('0');
        self.inc()?;
        if matches!(self.peek()?, ch if ch.is_ascii_digit()) {
          return err!(self, "Leading zeros are not allowed in numbers");
        }
      }
      _ => push_number(self, &mut num_str, "Invalid number format.")?,
    }
    if matches!(self.peek()?, '.') {
      has_decimal = true;
      num_str.push('.');
      self.inc()?;
      push_number(self, &mut num_str, "A digit is required after the decimal point.")?;
    }
    if matches!(self.peek()?, 'e' | 'E') {
      has_exponent = true;
      num_str.push(self.peek()?);
      self.inc()?;
      if matches!(self.peek()?, '+' | '-') {
        num_str.push(self.peek()?);
        self.inc()?;
      }
      push_number(self, &mut num_str, "A digit is required after the exponent notation.")?;
    }
    if has_decimal || has_exponent {
      num_str.parse::<f64>().map_or_else(
        |_| err!(self, "Invalid numeric value."),
        |float_val| Ok(Json { info: start, value: JValue::LFloat(float_val) }),
      )
    } else {
      num_str.parse::<i64>().map_or_else(
        |_| err!(self, "Invalid numeric value."),
        |int_val| Ok(Json { info: start, value: JValue::LInt(int_val) }),
      )
    }
  }
  /// Parses an object from the input code.
  fn parse_object(&mut self) -> JResult {
    let start = self.info.clone();
    let mut object = JObject::default();
    self.expect('{')?;
    self.skip_ws()?;
    if self.inc_if('}')? {
      return Ok(Json { info: start, value: JValue::LObject(object) });
    }
    loop {
      let key = self.parse_value()?;
      let JValue::LString(string) = key.value else {
        return err!(self, "Keys must be strings.");
      };
      self.expect(':')?;
      let value = self.parse_value()?;
      object.insert(string, value);
      if self.inc_if('}')? {
        return Ok(Json { info: start, value: JValue::LObject(object) });
      }
      self.expect(',')?;
    }
  }
  /// Parses a string from the input code.
  fn parse_string(&mut self) -> JResult {
    let start = self.info.clone();
    self.expect('"')?;
    let mut result = String::new();
    while let Ok(ch) = self.next() {
      match ch {
        '"' => return Ok(Json { info: start, value: JValue::LString(result) }),
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
            let maybe_cp = u32::from_str_radix(&hex, 16);
            let cp = match maybe_cp {
              Ok(cp) => cp,
              Err(err_msg) => return err!(self, "Invalid code point: {err_msg}"),
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
        ctrl if ctrl < '\u{20}' => return err!(self, "Invalid control character."),
        cha => result.push(cha),
      }
    }
    err!(self, "String is not properly terminated.")
  }
  /// Parses a value from the input code.
  fn parse_value(&mut self) -> JResult {
    self.skip_ws()?;
    let result = match self.peek()? {
      '"' => self.parse_string(),
      '{' => self.parse_object(),
      '[' => self.parse_array(),
      't' => self.parse_name("true", JValue::LBool(true)),
      'f' => self.parse_name("false", JValue::LBool(false)),
      'n' => self.parse_name("null", JValue::Null),
      '0'..='9' | '-' => self.parse_number(),
      _ => err!(self, "This is not a json value."),
    };
    self.skip_ws()?;
    result
  }
  /// Peek next character.
  fn peek(&self) -> ErrOR<char> {
    self
      .indices
      .get(self.index)
      .map(|&(_, ch)| ch)
      .ok_or(self.fmt_err("Unexpected end of text.", &self.info).into())
  }
  /// Skips whitespace characters in the input code.
  fn skip_ws(&mut self) -> ErrOR<()> {
    while let Ok(ch) = self.peek() {
      if !ch.is_ascii_whitespace() {
        break;
      }
      if ch == '\n' {
        self.info.line =
          self.info.line.checked_add(1).ok_or(self.fmt_err("LineOverflowError", &self.info))?;
      }
      self.inc()?;
    }
    Ok(())
  }
}
