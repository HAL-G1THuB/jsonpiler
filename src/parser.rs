mod err_msg;
mod jspl;
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
  pub(crate) fn consume_if(&mut self, expected: u8) -> ErrOR<bool> {
    if self.peek() == expected {
      self.advance(1)?;
      Ok(true)
    } else {
      Ok(false)
    }
  }
  fn expect(&mut self, expected: u8) -> ErrOR<()> {
    if self.consume_if(expected)? {
      Ok(())
    } else {
      parse_err!(self, self.pos, "Expected character '{}' not found.", char::from(expected))
    }
  }
  pub(crate) fn from(source: Vec<u8>) -> Self {
    Self { pos: Position { line: 1, offset: 0, size: 0 }, source }
  }
  pub(crate) fn parse(&mut self, is_jspl: bool) -> ErrOR<WithPos<Json>> {
    let value = if is_jspl { self.parse_block(true) } else { self.parse_json() }?;
    Ok(value)
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
  pub(crate) fn parse_json(&mut self) -> ErrOR<WithPos<Json>> {
    let result = self.parse_value()?;
    let _: ErrOR<()> = self.skip_ws();
    if self.pos.offset == self.source.len() {
      Ok(result)
    } else {
      parse_err!(self, "Unexpected trailing characters")
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
    let start = self.pos.offset;
    let is_negative = self.consume_if(b'-')?;
    if self.consume_if(b'0')? {
      if self.peek().is_ascii_digit() {
        return parse_err!(self, "Leading zeros are not allowed");
      }
    } else {
      self.skip_digits();
    }
    if self.consume_if(b'.')? {
      is_float = true;
      if !self.peek().is_ascii_digit() {
        return parse_err!(self, "Expected digit after '.'");
      }
      self.skip_digits();
    }
    if self.consume_if(b'e')? || self.consume_if(b'E')? {
      is_float = true;
      if !self.consume_if(b'+')? {
        self.consume_if(b'-')?;
      }
      if !self.peek().is_ascii_digit() {
        return parse_err!(self, "Expected digit after exponent");
      }
      self.skip_digits();
    }
    let end = self.pos.offset;
    pos.extend_to(end);
    let slice = &self.source[start..end];
    let num_str = unsafe { str::from_utf8_unchecked(slice) };
    if is_float {
      num_str.parse::<f64>().map_or_else(
        |_| parse_err!(self, pos, "Invalid float value: {num_str}"),
        |float| Ok(WithPos { pos, value: Json::Float(Lit(float)) }),
      )
    } else {
      let digits = if is_negative { &slice[1..] } else { slice };
      let mut acc: i64 = 0;
      for &byte in digits {
        if !byte.is_ascii_digit() {
          return parse_err!(
            self,
            pos,
            "InternalError: Invalid digit in integer: {}",
            char::from(byte)
          );
        }
        let digit = i64::from(byte - b'0');
        if let Some(ok_acc) = acc.checked_mul(10).and_then(|val| val.checked_add(digit)) {
          acc = ok_acc;
        } else {
          return parse_err!(self, pos, "Integer overflow");
        }
      }
      let final_val = if is_negative {
        if let Some(ok_neg) = acc.checked_neg() {
          ok_neg
        } else {
          return parse_err!(self, pos, "Integer overflow");
        }
      } else {
        acc
      };
      Ok(WithPos { pos, value: Json::Int(Lit(final_val)) })
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
