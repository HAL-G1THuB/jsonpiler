use crate::prelude::*;
impl Pos<Parser> {
  pub(crate) fn check_eof(&self) -> ParseErrOR<()> {
    if self.val.text.len() <= self.pos.offset as usize { Err(self.eof_err()) } else { Ok(()) }
  }
  pub(crate) fn consume(&mut self) -> ParseErrOR<u8> {
    self.check_eof()?;
    let char = self.peek();
    self.pos.offset += 1;
    Ok(char)
  }
  pub(crate) fn consume_if(&mut self, expected: u8) -> ParseErrOR<bool> {
    self.check_eof()?;
    Ok((self.peek() == expected).then(|| self.pos.offset += 1).is_some())
  }
  pub(crate) fn consume_if_multi(&mut self, expected: &[u8]) -> ParseErrOR<bool> {
    let mut last_err = None;
    for &byte in expected {
      match self.consume_if(byte) {
        Ok(found) => {
          if found {
            return Ok(true);
          }
        }
        Err(err) => last_err = Some(err),
      }
    }
    last_err.map_or(Ok(false), Err)
  }
  #[expect(clippy::cast_possible_truncation)]
  pub(crate) fn eof_err(&self) -> Pos<ParseErr> {
    Position { offset: self.val.text.len() as u32, ..self.pos }
      .with(UnexpectedToken(TokenKind::Eof))
  }
  pub(crate) fn expect(&mut self, expected: u8) -> ParseErrOR<()> {
    if self.consume_if(expected)? {
      Ok(())
    } else {
      Err(self.pos.with(ExpectedToken(TokenKind::Char(expected as char))))
    }
  }
  pub(crate) fn follow_digit(&self) -> bool {
    self.check_eof().is_ok() && self.peek().is_ascii_digit()
  }
  pub(crate) fn get_slice(&self, pos: Position) -> ParseErrOR<&str> {
    let Some(slice) = self.val.text.get(pos.offset as usize..pos.end() as usize) else {
      return Err(pos.with(InvalidChar));
    };
    Ok(slice)
  }
  pub(crate) fn peek(&self) -> u8 {
    self.val.text.as_bytes()[self.pos.offset as usize]
  }
  pub(crate) fn set_size(&self, pos: &mut Position) {
    pos.size = self.pos.offset - pos.offset;
  }
  pub(crate) fn skip_digits(&mut self, pos: Position) -> ParseErrOR<()> {
    self.check_eof()?;
    if !self.follow_digit() {
      return Err(pos.with(ExpectedToken(TokenKind::Digits)));
    }
    while self.follow_digit() {
      self.pos.offset += 1;
    }
    Ok(())
  }
  pub(crate) fn skip_ws(&mut self) -> ParseErrOR<()> {
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
pub(crate) fn op_precedence(op: &str) -> Option<usize> {
  OP_PRECEDENCE.iter().position(|ops| ops.contains(&op))
}
