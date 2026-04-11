pub(crate) mod error;
mod format_block;
mod formatter;
mod jspl;
use crate::prelude::*;
#[derive(Clone)]
pub(crate) struct Comment {
  leading: bool,
  text: String,
}
#[derive(Clone)]
pub(crate) struct Parser {
  comments: BTreeMap<u32, Comment>,
  pub exports: BTreeMap<String, WithPos<UserDefinedInfo>>,
  pub file: String,
  pub pos: Position,
  pub root_file: String,
  pub source: Vec<u8>,
  pub warns: Vec<WithPos<Warning>>,
}
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Position {
  pub file: FileId,
  pub info: (bool, bool),
  pub line: u32,
  pub offset: u32,
  pub size: u32,
}
impl Position {
  pub(crate) fn end(self) -> u32 {
    self.offset + self.size
  }
  pub(crate) fn with<V>(self, val: V) -> WithPos<V> {
    WithPos { val, pos: self }
  }
}
impl Parser {
  fn check_eof(&self) -> ParseErrOR<()> {
    if self.source.len() <= self.pos.offset as usize { Err(self.eof_err()) } else { Ok(()) }
  }
  fn consume_if(&mut self, expected: u8) -> ParseErrOR<bool> {
    self.check_eof()?;
    Ok((self.peek() == expected).then(|| self.pos.offset += 1).is_some())
  }
  fn consume_if_multi(&mut self, expected: &[u8]) -> ParseErrOR<bool> {
    let mut last_err = None;
    for &byte in expected {
      match self.consume_if(byte) {
        Ok(true) => return Ok(true),
        Ok(false) => (),
        Err(err) => last_err = Some(err),
      }
    }
    last_err.map_or(Ok(false), Err)
  }
  #[expect(clippy::cast_possible_truncation)]
  fn eof_err(&self) -> WithPos<ParseErr> {
    Position { offset: self.source.len() as u32, ..self.pos }.with(UnexpectedToken(TokenKind::Eof))
  }
  fn expect(&mut self, expected: u8) -> ParseErrOR<()> {
    if self.consume_if(expected)? {
      Ok(())
    } else {
      parse_err!(self.pos, ExpectedToken(TokenKind::Char(char::from(expected))))
    }
  }
  fn follow_digit(&self) -> bool {
    self.check_eof().is_ok() && self.peek().is_ascii_digit()
  }
  pub(crate) fn get_slice(&self, pos: Position) -> ParseErrOR<&str> {
    str::from_utf8(&self.source[pos.offset as usize..pos.end() as usize])
      .or(parse_err!(pos, InvalidChar))
  }
  pub(crate) fn new(source: Vec<u8>, file_idx: u32, file: String, root_file: String) -> Self {
    Self {
      pos: Position { line: 1, offset: 0, size: 0, file: file_idx, info: INFO_NONE },
      source,
      file,
      root_file,
      comments: BTreeMap::new(),
      exports: BTreeMap::new(),
      warns: vec![],
    }
  }
  fn next(&mut self) -> ParseErrOR<u8> {
    self.pos.offset += 1;
    self.check_eof()?;
    Ok(self.peek())
  }
  fn peek(&self) -> u8 {
    self.source[self.pos.offset as usize]
  }
  fn set_size(&self, pos: &mut Position) {
    pos.size = self.pos.offset - pos.offset;
  }
  fn skip_digits(&mut self, pos: Position) -> ParseErrOR<()> {
    self.check_eof()?;
    if !self.follow_digit() {
      return parse_err!(pos, ExpectedToken(TokenKind::Digits));
    }
    while self.follow_digit() {
      self.pos.offset += 1;
    }
    Ok(())
  }
  fn skip_ws(&mut self) -> ParseErrOR<()> {
    while (self.pos.offset as usize) < self.source.len() {
      if self.peek().is_ascii_whitespace() {
        if self.peek() == b'\n' {
          self.pos.line += 1;
        }
        self.pos.offset += 1;
        continue;
      }
      return Ok(());
    }
    Err(self.eof_err())
  }
}
impl Parser {
  fn parse_array(&mut self) -> ParseErrOR<Json> {
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
  pub(crate) fn parse_json(&mut self) -> ParseErrOR<WithPos<Json>> {
    let result = self.parse_value()?;
    if self.skip_ws().is_err() {
      Ok(result)
    } else {
      parse_err!(self.pos, ExpectedToken(TokenKind::Eof))
    }
  }
  pub(crate) fn parse_jspl(&mut self) -> ParseErrOR<WithPos<Json>> {
    if self.skip_ws_comment(true).is_err() {
      return Ok(self.pos.with(Null(Lit(()))));
    }
    let mut pos = self.pos;
    let val = self.parse_block(true)?;
    self.set_size(&mut pos);
    if self.skip_ws_comment(true).is_err() {
      Ok(pos.with(val))
    } else {
      parse_err!(self.pos, ExpectedToken(TokenKind::Eof))
    }
  }
  #[expect(clippy::cast_possible_truncation)]
  fn parse_keyword(&mut self, keyword: &'static str, value: Json) -> ParseErrOR<Json> {
    let mut pos = self.pos;
    self.pos.offset += keyword.len() as u32;
    self.check_eof()?;
    self.set_size(&mut pos);
    let slice = self.get_slice(pos)?;
    if slice == keyword { Ok(value) } else { parse_err!(pos, InvalidKeyword) }
  }
  fn parse_number(&mut self) -> ParseErrOR<Json> {
    let mut pos = self.pos;
    let mut float = false;
    let negative = self.consume_if(b'-')?;
    if self.consume_if(b'0')? {
      if self.follow_digit() {
        return parse_err!(pos, UnexpectedToken(TokenKind::Digits));
      }
    } else {
      self.skip_digits(pos)?;
    }
    if self.consume_if(b'.').is_ok_and(|bool| bool) {
      float = true;
      self.skip_digits(pos)?;
    }
    if self.consume_if_multi(b"eE").is_ok_and(|bool| bool) {
      float = true;
      self.consume_if_multi(b"+-")?;
      self.skip_digits(pos)?;
    }
    self.set_size(&mut pos);
    let slice = self.get_slice(pos)?;
    if float {
      return Ok(Float(Lit(slice.parse::<f64>().or(parse_err!(pos, InvalidFloat))?)));
    }
    let mut acc: i64 = 0;
    let mut chars = slice.chars();
    if negative {
      chars.next();
    }
    for byte in chars {
      let checked = if negative { i64::checked_sub } else { i64::checked_add };
      let digit = i64::from(u32::from(byte) - u32::from(b'0'));
      acc =
        acc.checked_mul(10).and_then(|val| checked(val, digit)).ok_or(pos.with(IntOutOfRange))?;
    }
    Ok(Int(Lit(acc)))
  }
  fn parse_object(&mut self) -> ParseErrOR<Json> {
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
  fn parse_string(&mut self) -> ParseErrOR<String> {
    let mut pos = self.pos;
    self.expect(b'"')?;
    let mut bytes = vec![];
    while (self.pos.offset as usize) < self.source.len() {
      if self.consume_if(b'"')? {
        self.set_size(&mut pos);
        return String::from_utf8(bytes).or(parse_err!(pos, InvalidChar));
      }
      match self.peek() {
        b'\n' => return parse_err!(self.pos, UnterminatedLiteral),
        b'\\' => match self.next()? {
          b'u' => {
            let mut code_point = 0;
            for _ in 0..4 {
              let ch = self.next()?;
              code_point = (code_point << 4)
                | u32::from(match ch {
                  b'0'..=b'9' => ch - b'0',
                  b'a'..=b'f' => ch - b'a' + 10,
                  b'A'..=b'F' => ch - b'A' + 10,
                  _ => return parse_err!(pos, UnexpectedToken(TokenKind::Esc('u'))),
                });
            }
            let Some(ch) = char::from_u32(code_point) else {
              return parse_err!(pos, UnexpectedToken(TokenKind::Esc('u')));
            };
            bytes.extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes());
          }
          esc @ (b'/' | b'"' | b'\\') => bytes.push(esc),
          b'b' => bytes.push(b'\x08'),
          b'f' => bytes.push(b'\x0C'),
          b'r' => bytes.push(b'\r'),
          b'n' => bytes.push(b'\n'),
          b't' => bytes.push(b'\t'),
          ctrl if ctrl.is_ascii_control() => return parse_err!(self.pos, InvalidChar),
          esc => return parse_err!(self.pos, UnexpectedToken(TokenKind::Esc(char::from(esc)))),
        },
        ctrl if ctrl.is_ascii_control() => return parse_err!(self.pos, InvalidChar),
        byte => bytes.push(byte),
      }
      self.pos.offset += 1;
    }
    Err(self.eof_err())
  }
  fn parse_value(&mut self) -> ParseErrOR<WithPos<Json>> {
    self.skip_ws()?;
    let mut pos = self.pos;
    let val = match self.peek() {
      b'"' => Str(Lit(self.parse_string()?)),
      b'{' => self.parse_object()?,
      b'[' => self.parse_array()?,
      b't' => self.parse_keyword("true", Bool(Lit(true)))?,
      b'f' => self.parse_keyword("false", Bool(Lit(false)))?,
      b'n' => self.parse_keyword("null", Null(Lit(())))?,
      b'0'..=b'9' | b'-' => self.parse_number()?,
      ctrl if ctrl.is_ascii_control() => return parse_err!(self.pos, InvalidChar),
      other => return parse_err!(self.pos, UnexpectedToken(TokenKind::Char(char::from(other)))),
    };
    self.set_size(&mut pos);
    Ok(pos.with(val))
  }
}
