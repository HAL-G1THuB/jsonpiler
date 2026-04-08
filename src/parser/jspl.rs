use crate::prelude::*;
impl Parser {
  pub(crate) fn parse_block(&mut self, is_top_level: bool) -> ParseErrOR<Json> {
    self.check_eof()?;
    let mut entries = vec![];
    let mut entry_pos: Option<Position> = None;
    if !is_top_level {
      self.expect(b'{')?;
    }
    loop {
      let result = self.skip_ws_comment(true);
      let is_eof = result.is_err();
      let is_separated = result.is_ok_and(|bool| bool);
      if (is_top_level && is_eof) || (!is_top_level && self.consume_if(b'}')?) {
        break;
      }
      if !entries.is_empty() && !is_separated && !is_eof {
        return parse_err!(self.pos, ExpectedToken(TokenKind::Separate));
      }
      let value = self.try_operator(0)?;
      if let Some(pos) = entry_pos {
        self.warn(pos, UselessLiteral);
      }
      if let Object(Lit(object)) = value.val {
        entries.extend(object);
      } else {
        entry_pos = Some(value.pos);
        entries.push((value.pos.with("value".to_owned()), value.pos.with(Array(Lit(vec![value])))));
      }
    }
    Ok(Object(Lit(entries)))
  }
  fn parse_call(&mut self) -> ParseErrOR<WithPos<Json>> {
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
      args.push(self.try_operator(0)?);
      self.skip_ws_comment(false)?;
      let did_consume = self.consume_if(b',')?;
      if did_consume {
        self.skip_ws_comment(false)?;
      }
      if self.consume_if(b')')? {
        break;
      }
      if !did_consume {
        self.expect(b',')?;
      }
      self.skip_ws_comment(false)?;
    }
    self.set_size(&mut pos);
    Ok(pos.with(Array(Lit(args))))
  }
  fn parse_ident(&mut self) -> ParseErrOR<WithPos<String>> {
    let mut pos = self.pos;
    while (self.pos.offset as usize) < self.source.len() {
      let byte = self.peek();
      if byte.is_ascii_whitespace() || byte.is_ascii_control() || b"#()[,]{:;}\"".contains(&byte) {
        break;
      }
      self.pos.offset += 1;
    }
    if pos.offset == self.pos.offset {
      return parse_err!(pos, ExpectedIdent);
    }
    self.set_size(&mut pos);
    Ok(pos.with(self.get_slice(pos)?.into()))
  }
  fn skip_space_check_sep(&mut self) -> bool {
    while (self.pos.offset as usize) < self.source.len() {
      match self.peek() {
        b' ' | b'\t' => {
          self.pos.offset += 1;
          if self.check_eof().is_err() {
            return true;
          }
        }
        b'\n' | b'#' => return true,
        ws if ws.is_ascii_whitespace() => return true,
        _ => return false,
      }
    }
    true
  }
  pub(crate) fn skip_ws_comment(&mut self, is_block: bool) -> ParseErrOR<bool> {
    let mut is_separated = false;
    while (self.pos.offset as usize) < self.source.len() {
      if self.consume_if(b'#')? {
        let mut pos = self.pos;
        pos.offset -= 1;
        loop {
          let result = self.consume_if(b'\n').ok();
          if result.is_none_or(|boolean| boolean) {
            self.set_size(&mut pos);
            if result.is_some() {
              pos.size -= 1;
            }
            self.comments.insert(
              pos.offset,
              Comment { leading: is_separated, text: self.get_slice(pos)?.to_owned() },
            );
            self.pos.line += 1;
            is_separated = true;
            break;
          }
          self.pos.offset += 1;
        }
        continue;
      }
      match self.peek() {
        b'\n' => {
          is_separated = true;
          self.pos.line += 1;
          self.pos.offset += 1;
        }
        b';' if is_block => {
          is_separated = true;
          self.pos.offset += 1;
        }
        ws if ws.is_ascii_whitespace() => self.pos.offset += 1,
        _ => return Ok(is_separated),
      }
    }
    Err(self.eof_err())
  }
  fn try_concat_op(
    &mut self,
    prec: usize,
    operator: &mut WithPos<String>,
    left: &mut WithPos<Json>,
  ) -> ParseErrOR<()> {
    let right = self.try_operator(prec)?;
    let mut pos = left.pos;
    self.set_size(&mut pos);
    operator.pos.info = INFO_OP;
    if let Object(Lit(obj)) = &mut left.val
      && obj.len() == 1
      && operator.val == obj[0].0.val
      && !matches!(obj[0].0.val.as_ref(), "<<" | ">>" | "%")
      && let Array(Lit(args)) = &mut obj[0].1.val
    {
      args.push(right);
    } else {
      let args = pos.with(Array(Lit(vec![take(left), right])));
      *left = pos.with(Object(Lit(vec![(operator.clone(), args)])));
    }
    Ok(())
  }
  fn try_operator(&mut self, min_prec: usize) -> ParseErrOR<WithPos<Json>> {
    let mut left = self.try_parse_value()?;
    let mut unknown_op: Option<String> = None;
    loop {
      let save = self.pos;
      if self.skip_space_check_sep() {
        self.pos = save;
        break;
      }
      let Some(mut operator) = self.try_parse_ident() else {
        break;
      };
      if let Some(prec) = op_precedence(&operator.val) {
        if prec < min_prec || self.skip_space_check_sep() {
          self.pos = save;
          break;
        }
        self.try_concat_op(prec + 1, &mut operator, &mut left)?;
        continue;
      }
      match &unknown_op {
        None => unknown_op = Some(operator.val.clone()),
        Some(op) if op != &operator.val => {
          self.pos = save;
          break;
        }
        _ => (),
      }
      if self.skip_space_check_sep() {
        self.pos = save;
        break;
      }
      self.try_concat_op(0, &mut operator, &mut left)?;
    }
    Ok(left)
  }
  fn try_parse_ident(&mut self) -> Option<WithPos<String>> {
    let saved = self.pos;
    self.skip_ws_comment(false).ok()?;
    let ident = self.parse_ident().ok()?;
    match ident.val.as_str() {
      "true" | "false" | "null" => {
        self.pos = saved;
        None
      }
      _ => Some(ident),
    }
  }
  fn try_parse_value(&mut self) -> ParseErrOR<WithPos<Json>> {
    let mut pos = self.pos;
    let val = match self.peek() {
      b'"' => Str(Lit(self.parse_string()?)),
      b'0'..=b'9' => self.parse_number()?,
      b'-' if self.source.get((self.pos.offset + 1) as usize).is_some_and(u8::is_ascii_digit) => {
        self.parse_number()?
      }
      b'[' => {
        self.pos.offset += 1;
        self.skip_ws_comment(false)?;
        if self.consume_if(b']')? {
          self.set_size(&mut pos);
          Array(Lit(vec![]))
        } else {
          let mut array = vec![];
          loop {
            array.push(self.try_operator(0)?);
            self.skip_ws_comment(false)?;
            let did_consume = self.consume_if(b',')?;
            if did_consume {
              self.skip_ws_comment(false)?;
            }
            if self.consume_if(b']')? {
              self.set_size(&mut pos);
              break Array(Lit(array));
            }
            if !did_consume {
              self.expect(b',')?;
            }
            self.skip_ws_comment(false)?;
          }
        }
      }
      b'{' => self.parse_block(false)?,
      _ => {
        let mut ident = self.parse_ident()?;
        let save = self.pos;
        let not_eof = self.skip_ws_comment(false).is_ok();
        if not_eof && self.peek() == b'(' {
          let args = self.parse_call()?;
          ident.pos.info = INFO_FUNC;
          Object(Lit(vec![(ident, args)]))
        } else if not_eof && self.consume_if(b':')? {
          self.skip_ws_comment(false)?;
          let args = self.try_operator(0)?;
          ident.pos.info = INFO_KEY_VAL;
          Object(Lit(vec![(ident, args)]))
        } else {
          self.pos = save;
          match ident.val.as_str() {
            "true" => Bool(Lit(true)),
            "false" => Bool(Lit(false)),
            "null" => Null(Lit(())),
            _ => {
              Object(Lit(vec![(ident.pos.with("$".into()), ident.map(|string| Str(Lit(string))))]))
            }
          }
        }
      }
    };
    self.set_size(&mut pos);
    Ok(pos.with(val))
  }
}
