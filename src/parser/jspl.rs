use crate::{Bind::Lit, ErrOR, Json, Parser, WithPos, parse_err};
impl Parser {
  pub(crate) fn parse_block(&mut self, is_top_level: bool) -> ErrOR<WithPos<Json>> {
    let mut pos = self.pos;
    let mut entries = vec![];
    if !is_top_level {
      self.expect(b'{')?;
    }
    loop {
      if is_top_level && self.skip_ws_and_comment().is_err() {
        break;
      }
      if !is_top_level {
        self.skip_ws_and_comment()?;
      }
      if !is_top_level && self.peek() == b'}' {
        break;
      }
      match self.try_three_tokens()? {
        WithPos { value: Json::Object(Lit(vec)), .. } => entries.extend(vec),
        value => {
          entries.push((WithPos { pos: value.pos, value: "value".to_owned() }, value));
        }
      }
    }
    if !is_top_level {
      self.expect(b'}')?;
    }
    pos.extend_to(self.pos.offset);
    Ok(WithPos { pos, value: Json::Object(Lit(entries)) })
  }
  fn parse_call(&mut self) -> ErrOR<WithPos<Json>> {
    let mut pos = self.pos;
    self.expect(b'(')?;
    self.skip_ws_and_comment()?;
    let mut args = vec![];
    if self.peek() == b')' {
      self.advance(1)?;
      pos.extend_to(self.pos.offset);
      return Ok(WithPos { pos, value: Json::Array(Lit(args)) });
    }
    loop {
      self.skip_ws_and_comment()?;
      args.push(self.try_three_tokens()?);
      self.skip_ws_and_comment()?;
      if self.peek() == b')' {
        self.advance(1)?;
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
    let start = self.pos.offset;
    let mut end = start;
    while end < self.source.len() {
      let byte = self.source[end];
      if !(0x21..=0x7E).contains(&byte) {
        break;
      }
      match byte {
        b'(' | b')' | b'[' | b']' | b'{' | b'}' | b',' | b'"' | b':' => break,
        _ => end += 1,
      }
    }
    if start == end {
      return parse_err!(self, pos, "Expected identifier.");
    }
    self.pos.offset = end;
    pos.extend_to(end);
    let ident = String::from_utf8(self.source[start..end].to_vec())
      .map_err(|_| "Invalid UTF-8 in identifier.".to_owned())?;
    Ok(WithPos { pos, value: ident })
  }
  fn parse_ident_expect_string(&mut self) -> ErrOR<WithPos<String>> {
    let ident = self.parse_ident()?;
    if !ident.value.is_empty() && ident.value.chars().nth(0) == Some('$') {
      parse_err!(self, ident.pos, "Invalid identifier")
    } else {
      match ident.value.as_str() {
        "true" | "false" | "null" => parse_err!(self, ident.pos, "Invalid identifier"),
        _ => Ok(ident),
      }
    }
  }
  fn skip_space(&mut self) -> bool {
    while self.pos.offset < self.source.len() {
      let byte = self.source[self.pos.offset];
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
    while self.pos.offset < self.source.len() {
      let mut byte = self.source[self.pos.offset];
      if byte == b'#' {
        while self.pos.offset < self.source.len() {
          byte = self.source[self.pos.offset];
          if byte == b'\n' {
            self.pos.line += 1;
            break;
          }
          self.advance(1)?;
        }
        self.advance(1)?;
        continue;
      }
      if !byte.is_ascii_whitespace() {
        return Ok(());
      }
      if byte == b'\n' {
        self.pos.line += 1;
      }
      self.advance(1)?;
    }
    self.check_eof()
  }
  fn try_parse_ident(&mut self) -> Option<WithPos<String>> {
    let saved = self.pos;
    self.skip_ws_and_comment().ok()?;
    if let Ok(res) = self.parse_ident_expect_string() {
      Some(res)
    } else {
      self.pos = saved;
      None
    }
  }
  fn try_parse_value(&mut self) -> ErrOR<WithPos<Json>> {
    let mut saved = self.pos;
    match self.peek() {
      b'"' => self.parse_string(),
      b'0'..=b'9' => self.parse_number(),
      b'-' if matches!(self.source.get(self.pos.offset + 1), Some(b'0'..=b'9')) => {
        self.parse_number()
      }
      b'[' => {
        self.advance(1)?;
        let mut list = vec![];
        loop {
          self.skip_ws_and_comment()?;
          list.push(self.try_three_tokens()?);
          self.skip_ws_and_comment()?;
          if self.peek() == b',' {
            self.advance(1)?;
          } else {
            break;
          }
        }
        self.skip_ws_and_comment()?;
        self.expect(b']')?;
        saved.extend_to(self.pos.offset);
        Ok(WithPos { pos: saved, value: Json::Array(Lit(list)) })
      }
      b'{' => {
        let val = self.parse_block(false)?;
        Ok(val)
      }
      _ => {
        let ident = self.parse_ident()?;
        if !ident.value.is_empty() && ident.value.chars().nth(0) == Some('$') {
          #[expect(clippy::string_slice)]
          Ok(WithPos {
            pos: ident.pos,
            value: Json::Object(Lit(vec![(
              WithPos { pos: ident.pos, value: "$".to_owned() },
              WithPos { pos: ident.pos, value: Json::String(Lit(ident.value[1..].to_owned())) },
            )])),
          })
        } else {
          let before_jspl_skip_ws = self.pos;
          if self.skip_ws_and_comment().is_ok() {
            if self.peek() == b'(' {
              let args = self.parse_call()?;
              return Ok(WithPos { pos: ident.pos, value: Json::Object(Lit(vec![(ident, args)])) });
            } else if self.skip_ws_and_comment().is_ok() && self.peek() == b':' {
              self.advance(1)?;
              self.skip_ws_and_comment()?;
              let args = self.try_parse_value()?;
              return Ok(WithPos { pos: ident.pos, value: Json::Object(Lit(vec![(ident, args)])) });
            }
          }
          self.pos = before_jspl_skip_ws;
          match ident.value.as_str() {
            "true" => Ok(WithPos { pos: ident.pos, value: Json::Bool(Lit(true)) }),
            "false" => Ok(WithPos { pos: ident.pos, value: Json::Bool(Lit(false)) }),
            "null" => Ok(WithPos { pos: ident.pos, value: Json::Null }),
            _ => Ok(WithPos { pos: ident.pos, value: Json::String(Lit(ident.value)) }),
          }
        }
      }
    }
  }
  fn try_three_tokens(&mut self) -> ErrOR<WithPos<Json>> {
    let val1 = self.try_parse_value()?;
    let saved = self.pos;
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
    let mut val1_pos = val1.pos;
    val1_pos.extend_to(self.pos.offset);
    let array_val = Json::Array(Lit(vec![val1, val2]));
    let object_val = Json::Object(Lit(vec![(ident, WithPos { pos: val1_pos, value: array_val })]));
    Ok(WithPos { pos: val1_pos, value: object_val })
  }
}
