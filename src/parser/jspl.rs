use crate::{
  Bind::Lit, CompilationErrKind::*, ErrOR, Json, JsonpilerErr::*, Parser, TokenKind, WithPos,
  parse_err,
};
impl Parser {
  pub(crate) fn parse_block(&mut self, is_top_level: bool) -> ErrOR<Json> {
    let pos = self.pos;
    self.check_eof()?;
    let mut exist_non_call = false;
    let mut entries = vec![];
    if !is_top_level {
      self.expect(b'{')?;
    }
    loop {
      let is_eof_or_not_sep = self.skip_block_ws_check_eof();
      if !is_top_level && self.source.get(self.pos.offset) == Some(&b'}') {
        let _: ErrOR<()> = self.advance(1);
        break;
      }
      let is_eof = is_eof_or_not_sep.is_err();
      if !entries.is_empty() && is_eof_or_not_sep.is_ok_and(|x| !x) {
        return parse_err!(self, ExpectedTokenError(TokenKind::NewLineOrSemiColon));
      }
      if is_eof {
        if is_top_level {
          break;
        }
        return parse_err!(self, pos, UnexpectedTokenError(TokenKind::Eof));
      }
      let val = self.try_multi_tokens()?;
      if exist_non_call {
        return parse_err!(self, val.pos, UnexpectedLiteral);
      }
      match val {
        WithPos { value: Json::Object(Lit(vec)), .. } => entries.extend(vec),
        WithPos { value: Json::Array(Lit(_)), .. } => {
          exist_non_call = true;
          entries.push((
            WithPos { pos: val.pos, value: "value".into() },
            WithPos { pos: val.pos, value: Json::Array(Lit(vec![val])) },
          ));
        }
        value => {
          exist_non_call = true;
          entries.push((WithPos { pos: value.pos, value: "value".into() }, value));
        }
      }
    }
    Ok(Json::Object(Lit(entries)))
  }
  fn parse_call(&mut self) -> ErrOR<WithPos<Json>> {
    let mut pos = self.pos;
    self.check_eof()?;
    self.expect(b'(')?;
    self.skip_ws_and_comment()?;
    let mut args = vec![];
    if self.consume_if(b')')? {
      pos.extend_to(self.pos.offset);
      return Ok(WithPos { pos, value: Json::Array(Lit(args)) });
    }
    loop {
      self.skip_ws_and_comment()?;
      args.push(self.try_multi_tokens()?);
      self.skip_ws_and_comment()?;
      if self.source.get(self.pos.offset) == Some(&b')') {
        let _: ErrOR<()> = self.advance(1);
        break;
      }
      self.expect(b',')?;
      self.skip_ws_and_comment()?;
    }
    pos.extend_to(self.pos.offset);
    Ok(WithPos { pos, value: Json::Array(Lit(args)) })
  }
  fn parse_ident(&mut self) -> ErrOR<WithPos<String>> {
    let mut pos = self.pos;
    self.check_eof()?;
    while self.pos.offset < self.source.len() {
      if self.check_eof().is_err() {
        break;
      }
      let byte = self.peek();
      if !(0x21..=0x7E).contains(&byte) {
        break;
      }
      match byte {
        b'(' | b')' | b'[' | b']' | b'{' | b'}' | b',' | b'"' | b':' | b';' => break,
        _ => self.pos.offset += 1,
      }
    }
    if pos.offset == self.pos.offset {
      return parse_err!(self, pos, InvalidIdentifier);
    }
    pos.extend_to(self.pos.offset);
    let Ok(ident) = String::from_utf8(self.source[pos.offset..self.pos.offset].to_vec()) else {
      return parse_err!(self, pos, InvalidChar);
    };
    Ok(WithPos { pos, value: ident })
  }
  fn parse_key(&mut self) -> ErrOR<WithPos<String>> {
    let ident = self.parse_ident()?;
    if ident.value.starts_with('$') {
      parse_err!(self, ident.pos, InvalidIdentifier)
    } else {
      match ident.value.as_str() {
        "true" | "false" | "null" => parse_err!(self, ident.pos, InvalidIdentifier),
        _ => Ok(ident),
      }
    }
  }
  fn skip_block_ws_check_eof(&mut self) -> ErrOR<bool> {
    let mut found_sep = false;
    while self.pos.offset < self.source.len() {
      if self.consume_if(b'#')? {
        while self.pos.offset < self.source.len() {
          if self.consume_if(b'\n')? {
            self.pos.line += 1;
            found_sep = true;
            break;
          }
          self.advance(1)?;
        }
        continue;
      }
      let ch = self.peek();
      if ch == b'\n' {
        self.pos.line += 1;
        found_sep = true;
        self.advance(1)?;
        continue;
      }
      if ch == b';' {
        found_sep = true;
        self.advance(1)?;
        continue;
      }
      if ch.is_ascii_whitespace() {
        self.advance(1)?;
        continue;
      }
      if !found_sep {
        return Ok(false);
      }
      return Ok(true);
    }
    parse_err!(self, UnexpectedTokenError(TokenKind::Eof))
  }
  fn skip_space(&mut self) -> bool {
    while self.pos.offset < self.source.len() {
      let byte = self.peek();
      match byte {
        b' ' | b'\t' => {
          if self.advance(1).is_err() {
            return true;
          }
        }
        b'\n' | b'\r' | b'#' => return true,
        _ if byte.is_ascii_whitespace() => return true,
        _ => return false,
      }
    }
    true
  }
  fn skip_ws_and_comment(&mut self) -> ErrOR<()> {
    self.check_eof()?;
    while self.pos.offset < self.source.len() {
      if self.consume_if(b'#')? {
        while self.pos.offset < self.source.len() {
          if self.consume_if(b'\n')? {
            self.pos.line += 1;
            break;
          }
          self.advance(1)?;
        }
        continue;
      }
      if !self.peek().is_ascii_whitespace() {
        return Ok(());
      }
      if self.peek() == b'\n' {
        self.pos.line += 1;
      }
      self.advance(1)?;
    }
    self.check_eof()
  }
  fn try_multi_tokens(&mut self) -> ErrOR<WithPos<Json>> {
    let val1 = self.try_parse_value()?;
    let mut saved = self.pos;
    let mut val1_pos = val1.pos;
    if self.skip_space() {
      self.pos = saved;
      return Ok(val1);
    }
    let Some(ident) = self.try_parse_ident() else {
      self.pos = saved;
      return Ok(val1);
    };
    if self.skip_space() {
      self.pos = saved;
      return Ok(val1);
    }
    let Ok(val2) = self.try_parse_value() else {
      self.pos = saved;
      return Ok(val1);
    };
    let mut operand_vec = vec![val1, val2];
    saved = self.pos;
    loop {
      if self.skip_space() {
        self.pos = saved;
        break;
      }
      let Some(rest_ident) = self.try_parse_ident() else {
        self.pos = saved;
        break;
      };
      if ident.value != rest_ident.value {
        self.pos = saved;
        break;
      }
      if self.skip_space() {
        self.pos = saved;
        break;
      }
      let Ok(rest_val) = self.try_parse_value() else {
        self.pos = saved;
        break;
      };
      saved = self.pos;
      operand_vec.push(rest_val);
    }
    val1_pos.extend_to(self.pos.offset);
    let array_val = Json::Array(Lit(operand_vec));
    let object_val = Json::Object(Lit(vec![(ident, WithPos { pos: val1_pos, value: array_val })]));
    Ok(WithPos { pos: val1_pos, value: object_val })
  }
  fn try_parse_ident(&mut self) -> Option<WithPos<String>> {
    let pos = self.pos;
    self.skip_ws_and_comment().ok()?;
    if let Ok(res) = self.parse_key() {
      Some(res)
    } else {
      self.pos = pos;
      None
    }
  }
  fn try_parse_value(&mut self) -> ErrOR<WithPos<Json>> {
    let mut pos = self.pos;
    let value = match self.peek() {
      b'"' => Json::String(Lit(self.parse_string()?)),
      b'0'..=b'9' => self.parse_number()?,
      b'-' if matches!(self.source.get(self.pos.offset + 1), Some(b'0'..=b'9')) => {
        self.parse_number()?
      }
      b'[' => {
        self.advance(1)?;
        let mut list = vec![];
        if self.peek() == b']' {
          let _: ErrOR<()> = self.advance(1);
          pos.extend_to(self.pos.offset);
          Json::Array(Lit(list))
        } else {
          loop {
            self.skip_ws_and_comment()?;
            list.push(self.try_multi_tokens()?);
            self.skip_ws_and_comment()?;
            if !self.consume_if(b',')? {
              break;
            }
          }
          self.skip_ws_and_comment()?;
          self.expect(b']')?;
          pos.extend_to(self.pos.offset);
          Json::Array(Lit(list))
        }
      }
      b'{' => self.parse_block(false)?,
      _ => {
        let ident = self.parse_ident()?;
        if ident.value.chars().nth(0) == Some('$') {
          #[expect(clippy::string_slice)]
          Json::Object(Lit(vec![(
            WithPos { pos: ident.pos, value: "$".into() },
            WithPos { pos: ident.pos, value: Json::String(Lit(ident.value[1..].into())) },
          )]))
        } else {
          let before_jspl_skip_ws = self.pos;
          let unreached_eof = self.skip_ws_and_comment().is_ok();
          if unreached_eof && self.peek() == b'(' {
            let args = self.parse_call()?;
            Json::Object(Lit(vec![(ident, args)]))
          } else if unreached_eof && self.consume_if(b':')? {
            self.skip_ws_and_comment()?;
            let args = self.try_multi_tokens()?;
            Json::Object(Lit(vec![(ident, args)]))
          } else {
            self.pos = before_jspl_skip_ws;
            match ident.value.as_str() {
              "true" => Json::Bool(Lit(true)),
              "false" => Json::Bool(Lit(false)),
              "null" => Json::Null,
              _ => Json::String(Lit(ident.value)),
            }
          }
        }
      }
    };
    pos.extend_to(self.pos.offset);
    Ok(WithPos { pos, value })
  }
}
