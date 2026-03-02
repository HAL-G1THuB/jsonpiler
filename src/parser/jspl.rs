use crate::prelude::*;
impl Parser {
  pub(crate) fn parse_block(&mut self, is_top_level: bool) -> ErrOR<Json> {
    self.check_eof()?;
    let mut entries = vec![];
    let mut entry_pos = None;
    if !is_top_level {
      self.expect(b'{')?;
    }
    loop {
      let result = self.skip_ws_comment(true);
      let is_eof = result.is_err();
      let is_sep = result.is_ok_and(|bool| bool);
      if (is_top_level && is_eof) || (!is_top_level && self.consume_if(b'}')?) {
        break;
      }
      if !entries.is_empty() && !is_sep && !is_eof {
        return err!(self.pos, ExpectedToken(TokenKind::Separate));
      }
      let val = self.try_multi_tokens()?;
      if let Some(pos) = entry_pos {
        return err!(pos, UnexpectedLiteral);
      }
      if let Object(Lit(object)) = val.val {
        entries.extend(object);
      } else {
        entry_pos = Some(val.pos);
        entries.push((val.pos.with("value".into()), val));
      }
    }
    Ok(Object(Lit(entries)))
  }
  fn parse_call(&mut self) -> ErrOR<WithPos<Json>> {
    let mut pos = self.pos;
    self.expect(b'(')?;
    self.skip_ws_comment(false)?;
    let mut args = vec![];
    if self.consume_if(b')')? {
      self.set_size(&mut pos);
      return Ok(pos.with(Array(Lit(args))));
    }
    loop {
      self.skip_ws_comment(false)?;
      args.push(self.try_multi_tokens()?);
      self.skip_ws_comment(false)?;
      if self.consume_if(b')')? {
        break;
      }
      self.expect(b',')?;
      self.skip_ws_comment(false)?;
    }
    self.set_size(&mut pos);
    Ok(pos.with(Array(Lit(args))))
  }
  fn parse_ident(&mut self) -> ErrOR<WithPos<String>> {
    let mut pos = self.pos;
    while self.pos.offset < self.source.len() {
      let byte = self.peek();
      if !(0x21..=0x7E).contains(&byte) {
        break;
      }
      if b"()[,]{:}\";".contains(&byte) {
        break;
      }
      self.pos.offset += 1;
    }
    if pos.offset == self.pos.offset {
      return err!(pos, ExpectedIdent);
    }
    self.set_size(&mut pos);
    let slice = self.source[pos.offset..self.pos.offset].to_vec();
    Ok(pos.with(String::from_utf8(slice).or(err!(pos, InvalidChar))?))
  }
  fn skip_space(&mut self) -> bool {
    while self.pos.offset < self.source.len() {
      match self.peek() {
        b' ' | b'\t' => {
          self.pos.offset += 1;
          if self.check_eof().is_err() {
            return true;
          }
        }
        b'\n' | b'\r' | b'#' => return true,
        ws if ws.is_ascii_whitespace() => return true,
        _ => return false,
      }
    }
    true
  }
  fn skip_ws_comment(&mut self, is_block: bool) -> ErrOR<bool> {
    let mut found_sep = false;
    while self.pos.offset < self.source.len() {
      if self.consume_if(b'#')? {
        while self.pos.offset < self.source.len() {
          if self.consume_if(b'\n')? {
            self.pos.line += 1;
            found_sep = true;
            break;
          }
          self.pos.offset += 1;
        }
        continue;
      }
      match self.peek() {
        b'\n' => {
          found_sep = true;
          self.pos.line += 1;
          self.pos.offset += 1;
        }
        b';' if is_block => {
          found_sep = true;
          self.pos.offset += 1;
        }
        ws if ws.is_ascii_whitespace() => self.pos.offset += 1,
        _ => return Ok(found_sep),
      }
    }
    self.check_eof().map(|()| false)
  }
  fn try_multi_tokens(&mut self) -> ErrOR<WithPos<Json>> {
    let first = self.try_parse_value()?;
    let mut save = self.pos;
    let mut pos = first.pos;
    let mut operands = vec![first.clone()];
    let mut op_opt: Option<WithPos<String>> = None;
    loop {
      if (!self.skip_space()
        && self.try_parse_ident().is_some_and(|rest| {
          if op_opt.is_none() {
            op_opt = Some(rest.clone());
          }
          op_opt.as_ref().is_some_and(|op| op.val == rest.val)
        })
        && !self.skip_space())
        && let Ok(rest) = self.try_parse_value()
      {
        save = self.pos;
        operands.push(rest);
        continue;
      }
      break;
    }
    self.pos = save;
    self.set_size(&mut pos);
    if let Some(operator) = op_opt {
      Ok(WithPos { pos, val: Object(Lit(vec![(operator, pos.with(Array(Lit(operands))))])) })
    } else {
      Ok(first)
    }
  }
  fn try_parse_ident(&mut self) -> Option<WithPos<String>> {
    let pos = self.pos;
    self.skip_ws_comment(false).ok()?;
    let ident = self.parse_ident().ok()?;
    match ident.val.as_str() {
      var if var.starts_with('$') || matches!(var, "true" | "false" | "null") => {
        self.pos = pos;
        None
      }
      _ => Some(ident),
    }
  }
  fn try_parse_value(&mut self) -> ErrOR<WithPos<Json>> {
    let mut pos = self.pos;
    let val = match self.peek() {
      b'"' => Str(Lit(self.parse_string()?)),
      b'0'..=b'9' => self.parse_number()?,
      b'-' if matches!(self.source.get(self.pos.offset + 1), Some(b'0'..=b'9')) => {
        self.parse_number()?
      }
      b'[' => {
        self.pos.offset += 1;
        if self.consume_if(b']')? {
          self.set_size(&mut pos);
          Array(Lit(vec![]))
        } else {
          let mut array = vec![];
          loop {
            self.skip_ws_comment(false)?;
            array.push(self.try_multi_tokens()?);
            self.skip_ws_comment(false)?;
            if self.consume_if(b']')? {
              self.set_size(&mut pos);
              break Array(Lit(array));
            }
            self.expect(b',')?;
          }
        }
      }
      b'{' => self.parse_block(false)?,
      _ => {
        let mut ident = self.parse_ident()?;
        if ident.val.as_bytes().first() == Some(&b'$') {
          ident.pos.size -= 1;
          #[expect(clippy::string_slice)]
          Object(Lit(vec![(
            Position { size: 1, ..ident.pos }.with("$".into()),
            Position { offset: ident.pos.offset + 1, ..ident.pos }
              .with(Str(Lit(ident.val[1..].into()))),
          )]))
        } else {
          let save = self.pos;
          let not_eof = self.skip_ws_comment(false).is_ok();
          if not_eof && self.peek() == b'(' {
            let args = self.parse_call()?;
            Object(Lit(vec![(ident, args)]))
          } else if not_eof && self.consume_if(b':')? {
            self.skip_ws_comment(false)?;
            let args = self.try_multi_tokens()?;
            Object(Lit(vec![(ident, args)]))
          } else {
            self.pos = save;
            match ident.val.as_str() {
              "true" => Bool(Lit(true)),
              "false" => Bool(Lit(false)),
              "null" => Null,
              _ => Str(Lit(ident.val)),
            }
          }
        }
      }
    };
    self.set_size(&mut pos);
    Ok(pos.with(val))
  }
}
