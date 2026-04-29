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
  pub dep: Dependency,
  pub exports: BTreeMap<String, Pos<UserDefinedInfo>>,
  pub file: String,
  pub source: String,
  pub warns: Vec<Pos<Warning>>,
}
#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, Eq)]
pub(crate) struct Position {
  pub file: FileId,
  pub info: (bool, bool),
  pub line: u32,
  pub offset: u32,
  pub size: u32,
}
impl Position {
  pub(crate) fn contains_inclusive(&self, file: FileId, offset: u32) -> bool {
    self.file == file && self.offset <= offset && offset <= self.end()
  }
  pub(crate) fn end(self) -> u32 {
    self.offset + self.size
  }
  #[expect(dead_code)]
  pub(crate) fn in_range(self, offset: u32) -> bool {
    self.offset <= offset && offset < self.end()
  }
  pub(crate) fn new(file: FileId) -> Self {
    Self { file, info: INFO_NONE, line: 0, offset: 0, size: 0 }
  }
  pub(crate) fn with<V>(self, val: V) -> Pos<V> {
    Pos { val, pos: self }
  }
}
impl Pos<Parser> {
  fn check_eof(&self) -> ParseErrOR<()> {
    if self.val.source.len() <= self.pos.offset as usize { Err(self.eof_err()) } else { Ok(()) }
  }
  fn consume(&mut self) -> ParseErrOR<u8> {
    self.check_eof()?;
    let char = self.peek();
    self.pos.offset += 1;
    Ok(char)
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
  fn eof_err(&self) -> Pos<ParseErr> {
    Position { offset: self.val.source.len() as u32, ..self.pos }
      .with(UnexpectedToken(TokenKind::Eof))
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
    let Some(slice) = self.val.source.get(pos.offset as usize..pos.end() as usize) else {
      return parse_err!(pos, InvalidChar);
    };
    Ok(slice)
  }
  pub(crate) fn new(source: String, file_idx: u32, file: String, id: LabelId) -> Self {
    Position::new(file_idx).with(Parser {
      source,
      file,
      comments: BTreeMap::new(),
      exports: BTreeMap::new(),
      warns: vec![],
      dep: Dependency { id, uses: vec![] },
    })
  }
  fn peek(&self) -> u8 {
    self.val.source.as_bytes()[self.pos.offset as usize]
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
    loop {
      if self.consume_if_multi(b" \t\r\x0C")? {
      } else if self.consume_if(b'\n')? {
        self.pos.line += 1;
      } else {
        break Ok(());
      }
    }
  }
}
impl Pos<Parser> {
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
  pub(crate) fn parse_json(&mut self) -> ParseErrOR<Pos<Json>> {
    let result = self.parse_value()?;
    if self.skip_ws().is_err() {
      Ok(result)
    } else {
      parse_err!(self.pos, ExpectedToken(TokenKind::Eof))
    }
  }
  pub(crate) fn parse_jspl(&mut self) -> ParseErrOR<Pos<Json>> {
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
    while (self.pos.offset as usize) < self.val.source.len() {
      if self.consume_if(b'"')? {
        self.set_size(&mut pos);
        return String::from_utf8(bytes).or(parse_err!(self.pos, InvalidChar));
      }
      match self.consume()? {
        b'\n' => return parse_err!(self.pos, UnterminatedLiteral),
        b'\\' => match self.consume()? {
          b'u' => {
            let mut code_point = self.parse_unicode_hex4()?;
            match code_point {
              0xD800..=0xDBFF => {
                if self.consume()? != b'\\' || self.consume()? != b'u' {
                  return parse_err!(self.pos, UnexpectedToken(TokenKind::Esc('u')));
                }
                let low = self.parse_unicode_hex4()?;
                if !(0xDC00..=0xDFFF).contains(&low) {
                  return parse_err!(self.pos, UnexpectedToken(TokenKind::Esc('u')));
                }
                code_point = 0x1_0000 + ((code_point - 0xD800) << 10) + (low - 0xDC00);
              }
              0xDC00..=0xDFFF => {
                return parse_err!(self.pos, UnexpectedToken(TokenKind::Esc('u')));
              }
              _ => (),
            }
            let Some(ch) = char::from_u32(code_point) else {
              return parse_err!(self.pos, UnexpectedToken(TokenKind::Esc('u')));
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
    }
    Err(self.eof_err())
  }
  fn parse_unicode_hex4(&mut self) -> ParseErrOR<u32> {
    let mut code_point = 0;
    for _ in 0..4 {
      let ch = self.consume()?;
      code_point <<= 4;
      let Some(hex) = ascii2hex(ch) else {
        return parse_err!(self.pos, UnexpectedToken(TokenKind::Esc('u')));
      };
      code_point |= hex as u32;
    }
    Ok(code_point)
  }
  fn parse_value(&mut self) -> ParseErrOR<Pos<Json>> {
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
