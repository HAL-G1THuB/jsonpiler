use crate::prelude::*;
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
      Err(self.pos.with(ExpectedToken(TokenKind::Eof)))
    }
  }
  #[expect(clippy::cast_possible_truncation)]
  fn parse_keyword(&mut self, keyword: &'static str, value: Json) -> ParseErrOR<Json> {
    let mut pos = self.pos;
    self.pos.offset += keyword.len() as u32;
    self.check_eof()?;
    self.set_size(&mut pos);
    let slice = self.get_slice(pos)?;
    if slice == keyword { Ok(value) } else { Err(pos.with(InvalidKeyword)) }
  }
  pub(crate) fn parse_number(&mut self) -> ParseErrOR<Json> {
    let mut pos = self.pos;
    let mut float = false;
    let negative = self.consume_if(b'-')?;
    if self.consume_if(b'0')? {
      if self.follow_digit() {
        return Err(pos.with(UnexpectedToken(TokenKind::Digits)));
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
      return Ok(Float(Lit(slice.parse::<f64>().map_err(|_err| pos.with(InvalidFloat))?)));
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
  pub(crate) fn parse_string(&mut self) -> ParseErrOR<String> {
    const fn build_escape_map() -> [u8; 256] {
      let mut table = [0u8; 256];
      table[b'"' as usize] = b'"';
      table[b'\\' as usize] = b'\\';
      table[b'/' as usize] = b'/';
      table[b'b' as usize] = b'\x08';
      table[b'f' as usize] = b'\x0C';
      table[b'r' as usize] = b'\r';
      table[b'n' as usize] = b'\n';
      table[b't' as usize] = b'\t';
      table
    }
    const ESCAPE_MAP: [u8; 256] = build_escape_map();
    let mut pos = self.pos;
    self.expect(b'"')?;
    let mut bytes = vec![];
    while (self.pos.offset as usize) < self.val.text.len() {
      if self.consume_if(b'"')? {
        self.set_size(&mut pos);
        return String::from_utf8(bytes).map_err(|_err| pos.with(InvalidChar));
      }
      match self.consume()? {
        b'\n' => return Err(self.pos.with(UnterminatedLiteral)),
        b'\\' => match self.consume()? {
          b'u' => {
            let mut code_point = self.parse_unicode_esc()?;
            match code_point {
              0xD800..=0xDBFF => {
                if self.consume()? != b'\\' || self.consume()? != b'u' {
                  return Err(self.pos.with(UnexpectedToken(TokenKind::Esc('u'))));
                }
                let low = self.parse_unicode_esc()?;
                if !(0xDC00..=0xDFFF).contains(&low) {
                  return Err(self.pos.with(UnexpectedToken(TokenKind::Esc('u'))));
                }
                code_point = 0x1_0000 + ((code_point - 0xD800) << 10) + (low - 0xDC00);
              }
              0xDC00..=0xDFFF => {
                return Err(self.pos.with(UnexpectedToken(TokenKind::Esc('u'))));
              }
              _ => (),
            }
            let Some(ch) = char::from_u32(code_point) else {
              return Err(self.pos.with(UnexpectedToken(TokenKind::Esc('u'))));
            };
            bytes.extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes());
          }
          esc => {
            let mapped = ESCAPE_MAP[esc as usize];
            if mapped != 0 {
              bytes.push(mapped);
              continue;
            }
            return Err(self.pos.with(if esc.is_ascii_control() {
              InvalidChar
            } else {
              UnexpectedToken(TokenKind::Esc(esc as char))
            }));
          }
        },
        ctrl if ctrl.is_ascii_control() => return Err(self.pos.with(InvalidChar)),
        byte => bytes.push(byte),
      }
    }
    Err(self.eof_err())
  }
  fn parse_unicode_esc(&mut self) -> ParseErrOR<u32> {
    let mut code_point = 0;
    for _ in 0..4 {
      let ch = self.consume()?;
      code_point <<= 4;
      let Some(hex) = ascii2hex(ch) else {
        return Err(self.pos.with(UnexpectedToken(TokenKind::Esc('u'))));
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
      ctrl if ctrl.is_ascii_control() => return Err(self.pos.with(InvalidChar)),
      other => return Err(self.pos.with(UnexpectedToken(TokenKind::Char(other as char)))),
    };
    self.set_size(&mut pos);
    Ok(pos.with(val))
  }
}
