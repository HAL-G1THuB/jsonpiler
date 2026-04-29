use crate::prelude::*;
impl Pos<Parser> {
  pub(crate) fn format_block(
    &mut self,
    out: &mut String,
    object: &[KeyVal],
    obj_size: u32,
    indentation: u32,
  ) -> Option<()> {
    for (obj_idx, key_val) in object.iter().enumerate() {
      let size = self.sizeof_key_val(key_val)? + indentation * 2;
      let (key, val) = key_val;
      if obj_idx != 0 {
        if LINE_MAX > obj_size {
          out.push(';');
        }
        self.comment_sep(out, obj_size, val.pos.offset, indentation);
      }
      if &key.val == "$"
        && let Str(Lit(variable)) = &val.val
      {
        out.push_str(variable);
      } else if let Some(value) = value(key, val) {
        self.format_json(out, value, indentation)?;
      } else {
        match key.pos.info {
          INFO_NONE | INFO_FUNC => {
            self.format_type_func(out, key, val, size, indentation)?;
          }
          INFO_KEY_VAL => {
            out.push_str(&key.val);
            out.push_str(": ");
            if val.val.operator().is_some() {
              out.push_str("{ ");
              self.format_json(out, val, indentation)?;
              out.push_str(" }");
            } else {
              self.format_json(out, val, indentation)?;
            }
          }
          INFO_OP => self.format_type_op(out, key, val, size, indentation)?,
        }
      }
    }
    Some(())
  }
  pub(crate) fn format_object(
    &mut self,
    out: &mut String,
    size: u32,
    pos: Position,
    object: &[KeyVal],
    indentation: u32,
  ) -> Option<()> {
    if object.is_empty() {
      out.push_str("{}");
      return Some(());
    }
    if object.len() == 1 && object[0].0.pos.info != INFO_KEY_VAL {
      return self.format_block(out, object, size, indentation);
    }
    out.push('{');
    self.comment_sep(out, size, pos.offset + 1, indentation);
    self.format_block(out, object, size, indentation)?;
    self.comment_sep(out, size, pos.end() - 1, indentation.saturating_sub(1));
    out.push('}');
    Some(())
  }
  pub(crate) fn format_type_func(
    &mut self,
    out: &mut String,
    key: &Pos<String>,
    val: &Pos<Json>,
    size: u32,
    indentation: u32,
  ) -> Option<()> {
    out.push_str(&key.val);
    out.push('(');
    let val_size = (indentation + 1) * 2 + self.sizeof_json(val)?;
    let is_block = val.val.is_block()
      || if let Array(Lit(array)) = &val.val
        && array.len() == 1
        && array[0].val.is_block()
      {
        true
      } else {
        false
      };
    let is_single_if = if let Array(Lit(array)) = &val.val
      && array.len() == 2
      && array[0].val.as_type() != ArrayT
    {
      matches!(key.val.as_ref(), "if")
    } else {
      false
    };
    let is_while = key.val == "while";
    let is_define = key.val == "define";
    self.comment(
      out,
      val.pos.offset + 1,
      indentation + u32::from(val_size < LINE_MAX || !is_block),
      LINE_MAX <= size && !is_define && !is_while && !is_single_if,
    );
    let mut signature = 0;
    match val {
      Pos { val: Array(Lit(args)), .. } => {
        for (idx, item) in args.iter().enumerate() {
          if idx == 0 {
            self.format_json(out, item, indentation + 1)?;
            continue;
          }
          out.push(',');
          if is_define && idx < 3 && signature < LINE_MAX {
            out.push(' ');
            self.format_json(out, item, indentation + 1)?;
            signature += self.sizeof_json(item)?;
          } else {
            self.format_json_sep(out, size, item.pos.offset + 1, item, indentation)?;
          }
        }
      }
      arg => self.format_json(out, arg, indentation + 1)?,
    }
    self.comment(out, val.pos.end() - 1, indentation, LINE_MAX <= size);
    out.push(')');
    Some(())
  }
  pub(crate) fn format_type_op(
    &mut self,
    out: &mut String,
    key: &Pos<String>,
    val: &Pos<Json>,
    size: u32,
    indentation: u32,
  ) -> Option<()> {
    let Pos { val: Array(Lit(args)), .. } = val else { return None };
    for (idx, item) in args.iter().enumerate() {
      if idx != 0 {
        out.push(' ');
        out.push_str(&key.val);
        out.push(' ');
      }
      if ASSIGN_OP.contains(&key.val.as_ref()) {
        self.format_json(out, item, indentation)?;
      } else if LINE_MAX <= size || item.val.needs_braces(&key.val) {
        out.push('{');
        self.format_json_sep(out, size, item.pos.offset + 1, item, indentation)?;
        self.comment_sep(out, size, item.pos.end() - 1, indentation);
        out.push('}');
      } else {
        self.format_json(out, item, indentation + 1)?;
      }
    }
    Some(())
  }
  #[expect(clippy::cast_possible_truncation)]
  pub(crate) fn sizeof_key_val(&self, (key, val): &KeyVal) -> Option<u32> {
    if &key.val == "$"
      && let Some(variable) = &val.val.as_str()
    {
      Some(variable.len() as u32)
    } else if let Some(value) = value(key, val) {
      self.sizeof_json(value)
    } else {
      Some(match key.pos.info {
        INFO_NONE | INFO_FUNC => {
          let mut acc = key.val.len() as u32 + self.sizeof_json(val)?;
          if let Array(Lit(array)) = &val.val {
            if array.is_empty() {
              acc -= 2;
            }
          } else {
            acc += 2;
          }
          acc
        }
        INFO_KEY_VAL => key.val.len() as u32 + 2 + self.sizeof_json(val)?,
        INFO_OP => {
          let Array(Lit(args)) = &val.val else { return None };
          let mut acc = ((key.val.len() + 2) * (args.len().saturating_sub(1)) + 2) as u32;
          for item in args {
            if item.val.needs_braces(&key.val) {
              acc += 4;
            }
            acc += self.sizeof_json(item)?;
          }
          acc
        }
      })
    }
  }
}
impl Json {
  pub(crate) fn is_block(&self) -> bool {
    if let Object(Lit(obj)) = &self
      && obj.len() != 1
    {
      true
    } else {
      false
    }
  }
  pub(crate) fn needs_braces(&self, parent: &str) -> bool {
    let Some(child) = self.operator() else {
      return false;
    };
    let child_prec = op_precedence(&child.val);
    let parent_prec = op_precedence(parent);
    match (child_prec, parent_prec) {
      (None, None | Some(_)) => true,
      (Some(_), None) => false,
      (Some(ch), Some(pa)) => {
        ch <= pa
          && parent != child.val
          && !(parent == "-" && child.val == "+")
          && !(parent == "+" && child.val == "-")
      }
    }
  }
  pub(crate) fn operator(&self) -> Option<Pos<String>> {
    if let Object(Lit(obj)) = &self
      && obj.len() == 1
      && obj[0].0.pos.info == INFO_OP
    {
      Some(obj[0].0.clone())
    } else {
      None
    }
  }
}
pub(crate) fn value<'a>(key: &Pos<String>, val: &'a Pos<Json>) -> Option<&'a Pos<Json>> {
  if &key.val == "value"
    && let Array(Lit(array)) = &val.val
    && array.len() == 1
  {
    Some(&array[0])
  } else {
    None
  }
}
