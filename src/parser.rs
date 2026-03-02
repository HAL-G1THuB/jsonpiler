pub(crate) mod err_msg;
mod jspl;
use crate::prelude::*;
#[derive(Clone)]
pub(crate) struct Parser {
  pub file: String,
  pub pos: Position,
  pub source: Vec<u8>,
}
impl Parser {
  fn check_eof(&mut self) -> ErrOR<()> {
    if self.source.len() <= self.pos.offset {
      err!(
        Position { offset: self.source.len().saturating_sub(1), ..self.pos },
        UnexpectedToken(TokenKind::Eof)
      )
    } else {
      Ok(())
    }
  }
  fn consume_if(&mut self, expected: u8) -> ErrOR<bool> {
    self.check_eof()?;
    Ok((self.peek() == expected).then(|| self.pos.offset += 1).is_some())
  }
  fn consume_if_ex(&mut self, expected: &[u8]) -> ErrOR<bool> {
    let mut last_err = None;
    for &byte in expected {
      match self.consume_if(byte) {
        Ok(cond) => {
          if cond {
            return Ok(true);
          }
        }
        Err(err) => last_err = Some(err),
      }
    }
    last_err.map_or(Ok(false), Err)
  }
  fn expect(&mut self, expected: u8) -> ErrOR<()> {
    if self.consume_if(expected)? {
      Ok(())
    } else {
      err!(self.pos, ExpectedToken(TokenKind::Char(char::from(expected))))
    }
  }
  pub(crate) fn from(source: Vec<u8>, file: usize, file_name: String) -> Self {
    Self { pos: Position { line: 1, offset: 0, size: 0, file }, source, file: file_name }
  }
  fn peek(&self) -> u8 {
    self.source[self.pos.offset]
  }
  fn set_size(&self, pos: &mut Position) {
    pos.size = self.pos.offset - pos.offset;
  }
  fn skip_digits(&mut self) {
    while self.source.get(self.pos.offset).is_some_and(u8::is_ascii_digit) {
      self.pos.offset += 1;
    }
  }
  fn skip_ws(&mut self) -> ErrOR<()> {
    while self.pos.offset < self.source.len() {
      if self.peek().is_ascii_whitespace() {
        if self.peek() == b'\n' {
          self.pos.line += 1;
        }
        self.pos.offset += 1;
        continue;
      }
      return Ok(());
    }
    self.check_eof()
  }
}
impl Parser {
  pub(crate) fn parse(&mut self, is_jspl: bool) -> ErrOR<WithPos<Json>> {
    if is_jspl {
      let mut pos = self.pos;
      let val = self.parse_block(true)?;
      self.set_size(&mut pos);
      Ok(pos.with(val))
    } else {
      self.parse_json()
    }
  }
  fn parse_array(&mut self) -> ErrOR<Json> {
    self.expect(b'[')?;
    self.skip_ws()?;
    if self.consume_if(b']')? {
      return Ok(Array(Lit(vec![])));
    }
    let mut array = vec![];
    loop {
      array.push(self.parse_value()?);
      self.skip_ws()?;
      if self.consume_if(b']')? {
        return Ok(Array(Lit(array)));
      }
      self.expect(b',')?;
    }
  }
  pub(crate) fn parse_json(&mut self) -> ErrOR<WithPos<Json>> {
    let result = self.parse_value()?;
    if self.skip_ws().is_err() { Ok(result) } else { err!(self.pos, ExpectedToken(TokenKind::Eof)) }
  }
  fn parse_keyword(&mut self, keyword: &'static str, value: Json) -> ErrOR<Json> {
    let pos = self.pos;
    self.pos.offset += keyword.len();
    self.check_eof()?;
    let slice = &self.source[pos.offset..self.pos.offset];
    if slice == keyword.as_bytes() { Ok(value) } else { err!(pos, ParseError(keyword)) }
  }
  fn parse_number(&mut self) -> ErrOR<Json> {
    let mut pos = self.pos;
    let mut float = false;
    let negative = self.consume_if(b'-')?;
    let start = self.pos.offset;
    if self.consume_if_ex(b"0")? {
      if self.check_eof().is_ok() && self.peek().is_ascii_digit() {
        return err!(self.pos, StartsWithZero);
      }
    } else {
      self.skip_digits();
    }
    if self.consume_if_ex(b".").is_ok_and(|bool| bool) {
      float = true;
      self.check_eof()?;
      if !self.peek().is_ascii_digit() {
        return err!(self.pos, ParseError("Float"));
      }
      self.skip_digits();
    }
    if self.consume_if_ex(b"eE").is_ok_and(|bool| bool) {
      float = true;
      self.consume_if_ex(b"+-")?;
      self.check_eof()?;
      if !self.peek().is_ascii_digit() {
        return err!(self.pos, ParseError("Float"));
      }
      self.skip_digits();
    }
    self.set_size(&mut pos);
    let slice = &self.source[start..self.pos.offset];
    if float {
      let mut num_str = if negative { "-" } else { "" }.to_owned();
      num_str.push_str(str::from_utf8(slice).or(err!(pos, InvalidChar))?);
      Ok(Float(Lit(num_str.parse::<f64>().or(err!(pos, ParseError("Float")))?)))
    } else {
      let mut acc: i64 = 0;
      for byte in slice {
        let checked = if negative { i64::checked_sub } else { i64::checked_add };
        let digit = i64::from(byte - b'0');
        if let Some(new_acc) = acc.checked_mul(10).and_then(|val| checked(val, digit)) {
          acc = new_acc;
        } else {
          return err!(pos, IntegerOutOfRange);
        }
      }
      Ok(Int(Lit(acc)))
    }
  }
  fn parse_object(&mut self) -> ErrOR<Json> {
    self.expect(b'{')?;
    self.skip_ws()?;
    if self.consume_if(b'}')? {
      return Ok(Object(Lit(vec![])));
    }
    let mut object = vec![];
    loop {
      let mut pos = self.pos;
      let key = self.parse_string()?;
      self.set_size(&mut pos);
      self.skip_ws()?;
      self.expect(b':')?;
      object.push((pos.with(key), self.parse_value()?));
      self.skip_ws()?;
      if self.consume_if(b'}')? {
        return Ok(Object(Lit(object)));
      }
      self.expect(b',')?;
      self.skip_ws()?;
    }
  }
  fn parse_string(&mut self) -> ErrOR<String> {
    let mut pos = self.pos;
    self.expect(b'"')?;
    let mut bytes = vec![];
    while self.pos.offset < self.source.len() {
      match self.peek() {
        b'"' => {
          self.pos.offset += 1;
          self.set_size(&mut pos);
          return String::from_utf8(bytes).or(err!(pos, InvalidChar));
        }
        b'\r' | b'\n' => return err!(self.pos, UnterminatedLiteral),
        b'\\' => {
          self.pos.offset += 1;
          self.check_eof()?;
          match self.peek() {
            b'u' => {
              let mut code_point: u32 = 0;
              for _ in 0..4 {
                self.pos.offset += 1;
                self.check_eof()?;
                let ch = self.peek();
                code_point = (code_point << 4)
                  | u32::from(match ch {
                    b'0'..=b'9' => ch - b'0',
                    b'a'..=b'f' => ch - b'a' + 10,
                    b'A'..=b'F' => ch - b'A' + 10,
                    _ => return err!(pos, UnexpectedToken(TokenKind::Esc('u'))),
                  });
              }
              let ch =
                or_err!((char::from_u32(code_point)), pos, UnexpectedToken(TokenKind::Esc('u')))?;
              bytes.extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes());
            }
            b'/' => bytes.push(b'/'),
            b'"' => bytes.push(b'"'),
            b'\\' => bytes.push(b'\\'),
            b'b' => bytes.push(b'\x08'),
            b'f' => bytes.push(b'\x0C'),
            b'r' => bytes.push(b'\r'),
            b'n' => bytes.push(b'\n'),
            b't' => bytes.push(b'\t'),
            ctrl if ctrl.is_ascii_control() => return err!(self.pos, InvalidChar),
            esc => return err!(self.pos, UnexpectedToken(TokenKind::Esc(char::from(esc)))),
          }
        }
        ctrl if ctrl.is_ascii_control() => return err!(self.pos, InvalidChar),
        byte => bytes.push(byte),
      }
      self.pos.offset += 1;
    }
    self.check_eof().map(|()| String::new())
  }
  fn parse_value(&mut self) -> ErrOR<WithPos<Json>> {
    self.skip_ws()?;
    let mut pos = self.pos;
    let val = match self.peek() {
      b'"' => Str(Lit(self.parse_string()?)),
      b'{' => self.parse_object()?,
      b'[' => self.parse_array()?,
      b't' => self.parse_keyword("true", Bool(Lit(true)))?,
      b'f' => self.parse_keyword("false", Bool(Lit(false)))?,
      b'n' => self.parse_keyword("null", Null)?,
      b'0'..=b'9' | b'-' => self.parse_number()?,
      ctrl if ctrl.is_ascii_control() => return err!(self.pos, InvalidChar),
      other => return err!(self.pos, UnexpectedToken(TokenKind::Char(char::from(other)))),
    };
    self.set_size(&mut pos);
    Ok(pos.with(val))
  }
}
