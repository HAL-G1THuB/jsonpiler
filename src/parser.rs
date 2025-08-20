mod err_msg;
use crate::{Bind::Lit, ErrOR, Json, Position, WithPos, parse_err, return_if};
#[derive(Clone)]
pub(crate) struct Parser {
  pos: Position,
  source: Vec<u8>,
}
impl Position {
  fn extend_to(&mut self, end: usize) {
    self.size = end - self.offset;
  }
}
impl Parser {
  fn advance(&mut self, num: usize) -> ErrOR<()> {
    self.pos.offset += num;
    self.check_eof()
  }
  fn check_eof(&mut self) -> ErrOR<()> {
    if self.source.len() <= self.pos.offset {
      parse_err!(self, Position { offset: self.source.len() - 1, ..self.pos }, "Unexpected EOF.")
    } else {
      Ok(())
    }
  }
  fn expect(&mut self, expected: u8) -> ErrOR<()> {
    if self.peek() == expected {
      self.advance(1)
    } else {
      parse_err!(self, self.pos, "Expected character '{}' not found.", char::from(expected))
    }
  }
  pub(crate) fn from(source: Vec<u8>) -> Self {
    Self { pos: Position { line: 1, offset: 0, size: 0 }, source }
  }
  fn next(&mut self) -> ErrOR<u8> {
    let byte = self.peek();
    self.advance(1)?;
    Ok(byte)
  }
  pub(crate) fn parse(&mut self) -> ErrOR<WithPos<Json>> {
    let result = self.parse_value()?;
    let _: ErrOR<()> = self.skip_ws();
    if self.pos.offset == self.source.len() {
      Ok(result)
    } else {
      parse_err!(self, "Unexpected trailing characters")
    }
  }
  fn parse_array(&mut self) -> ErrOR<WithPos<Json>> {
    let mut pos = self.pos;
    let mut array = vec![];
    self.expect(b'[')?;
    self.skip_ws()?;
    return_if!(self, b']', pos, Json::Array(Lit(array)));
    loop {
      array.push(self.parse_value()?);
      self.skip_ws()?;
      return_if!(self, b']', pos, Json::Array(Lit(array)));
      self.expect(b',')?;
      self.skip_ws()?;
    }
  }
  fn parse_keyword(&mut self, keyword: &str, value: Json) -> ErrOR<WithPos<Json>> {
    let mut pos = self.pos;
    self.advance(keyword.len())?;
    let slice = &self.source[pos.offset..self.pos.offset];
    if slice == keyword.as_bytes() {
      pos.extend_to(self.pos.offset);
      Ok(WithPos { pos, value })
    } else {
      parse_err!(self, pos, "Failed to parse '{keyword}'")
    }
  }
  fn parse_number(&mut self) -> ErrOR<WithPos<Json>> {
    let mut pos = self.pos;
    let mut is_float = false;
    let mut ch = self.next()?;
    if ch == b'-' {
      ch = self.next()?;
    }
    if ch == b'0' {
      ch = self.peek();
      if ch.is_ascii_digit() {
        return parse_err!(self, "Leading zeros are not allowed");
      }
    } else {
      self.skip_digits();
    }
    if ch == b'.' {
      self.check_eof()?;
      is_float = true;
      ch = self.peek();
      if !ch.is_ascii_digit() {
        return parse_err!(self, "Expected digit after '.'");
      }
      self.skip_digits();
    }
    if matches!(ch, b'e' | b'E') {
      self.check_eof()?;
      is_float = true;
      ch = self.peek();
      if matches!(ch, b'+' | b'-') {
        self.advance(1)?;
        ch = self.peek();
      }
      if !ch.is_ascii_digit() {
        return parse_err!(self, "Expected digit after exponent");
      }
      self.skip_digits();
    }
    let end = self.pos.offset;
    pos.extend_to(end);
    let slice = &self.source[pos.offset..end];
    let num_str = unsafe { str::from_utf8_unchecked(slice) };
    if is_float {
      num_str.parse::<f64>().map_or_else(
        |_| parse_err!(self, pos, "Invalid float value: {num_str}"),
        |float| Ok(WithPos { pos, value: Json::Float(Lit(float)) }),
      )
    } else {
      num_str.parse::<i64>().map_or_else(
        |_| parse_err!(self, pos, "Invalid integer value: {num_str}"),
        |int| Ok(WithPos { pos, value: Json::Int(Lit(int)) }),
      )
    }
  }
  fn parse_object(&mut self) -> ErrOR<WithPos<Json>> {
    let mut pos = self.pos;
    let mut object = vec![];
    self.expect(b'{')?;
    self.skip_ws()?;
    return_if!(self, b'}', pos, Json::Object(Lit(object)));
    loop {
      let key = self.parse_value()?;
      let Json::String(Lit(string)) = key.value else {
        return parse_err!(self, key.pos, "Keys must be strings.");
      };
      self.skip_ws()?;
      self.expect(b':')?;
      self.skip_ws()?;
      object.push((WithPos { value: string, pos: key.pos }, self.parse_value()?));
      self.skip_ws()?;
      return_if!(self, b'}', pos, Json::Object(Lit(object)));
      self.expect(b',')?;
      self.skip_ws()?;
    }
  }
  fn parse_string(&mut self) -> ErrOR<WithPos<Json>> {
    let mut pos = self.pos;
    self.expect(b'"')?;
    let mut bytes = Vec::new();
    loop {
      match self.peek() {
        b'"' => {
          self.pos.offset += 1;
          pos.extend_to(self.pos.offset);
          let string = String::from_utf8(bytes).map_err(|_| {
            format!("Invalid UTF-8 in string.\nError occurred on line: {}", self.pos.line)
          })?;
          return Ok(WithPos { pos, value: Json::String(Lit(string)) });
        }
        b'\r' | b'\n' => return parse_err!(self, "Unescaped newline in string."),
        b'\\' => {
          self.advance(1)?;
          let esc = self.peek();
          match esc {
            b'u' => {
              let mut hex = String::new();
              for _ in 0..4 {
                self.advance(1)?;
                hex.push(char::from(self.peek()));
              }
              let code_point =
                u32::from_str_radix(&hex, 16).map_err(|_| "Invalid hex digits in \\u escape.")?;
              let ch = char::from_u32(code_point).ok_or("Invalid Unicode codepoint")?;
              let mut utf8_buf = [0u8; 4];
              for byte in ch.encode_utf8(&mut utf8_buf).as_bytes() {
                bytes.append(&mut format!("\\{byte:03o}").into_bytes());
              }
            }
            b'/' => bytes.push(b'/'),
            b'"' => bytes.push(b'"'),
            b'\\' => bytes.push(b'\\'),
            b'b' => bytes.push(b'\x08'),
            b'f' => bytes.push(b'\x0C'),
            b'r' => bytes.push(b'\r'),
            b'n' => bytes.push(b'\n'),
            b't' => bytes.push(b'\t'),
            _ => return parse_err!(self, "Invalid escape sequence."),
          }
        }
        0x00..=0x1F => return parse_err!(self, "Invalid control character in string."),
        byte => bytes.push(byte),
      }
      self.advance(1)?;
    }
  }
  fn parse_value(&mut self) -> ErrOR<WithPos<Json>> {
    self.skip_ws()?;
    match self.peek() {
      b'"' => self.parse_string(),
      b'{' => self.parse_object(),
      b'[' => self.parse_array(),
      b't' => self.parse_keyword("true", Json::Bool(Lit(true))),
      b'f' => self.parse_keyword("false", Json::Bool(Lit(false))),
      b'n' => self.parse_keyword("null", Json::Null),
      b'0'..=b'9' | b'-' => self.parse_number(),
      _ => parse_err!(self, "Expected a json value, but an unknown value was passed."),
    }
  }
  fn peek(&self) -> u8 {
    self.source[self.pos.offset]
  }
  fn skip_digits(&mut self) {
    while let Some(&ch) = self.source.get(self.pos.offset) {
      if !ch.is_ascii_digit() {
        break;
      }
      self.pos.offset += 1;
    }
  }
  fn skip_ws(&mut self) -> ErrOR<()> {
    while self.pos.offset < self.source.len() {
      let ch = self.source[self.pos.offset];
      if !ch.is_ascii_whitespace() {
        return Ok(());
      }
      if ch == b'\n' {
        self.pos.line += 1;
      }
      self.advance(1)?;
    }
    self.check_eof()
  }
}
