use super::utility::*;
use crate::prelude::*;
impl Pos<Parser> {
  pub(crate) fn format_block(
    &self,
    fmter: &mut Formatter,
    object: &[KeyVal],
    obj_size: u32,
  ) -> Option<()> {
    for (obj_idx, key_val) in object.iter().enumerate() {
      let size = self.sizeof_key_val(key_val)? + fmter.indentation * 2;
      let (key, val) = key_val;
      if obj_idx != 0 {
        if LINE_MAX > obj_size {
          fmter.out.push(';');
        }
        self.comment_sep(fmter, obj_size, val.pos.offset);
      }
      if &key.val == "$"
        && let Str(Lit(variable)) = &val.val
      {
        fmter.out.push_str(variable);
      } else if let Some(value) = value(key, val) {
        self.format_json(fmter, value)?;
      } else {
        match key.pos.info {
          INFO_NONE | INFO_FUNC => {
            self.format_type_func(fmter, key, val, size)?;
          }
          INFO_KEY_VAL => {
            fmter.out.push_str(&key.val);
            fmter.out.push_str(": ");
            if val.val.operator().is_some() {
              fmter.out.push_str("{ ");
              self.format_json(fmter, val)?;
              fmter.out.push_str(" }");
            } else {
              self.format_json(fmter, val)?;
            }
          }
          INFO_OP => self.format_type_op(fmter, key, val, size)?,
        }
      }
    }
    Some(())
  }
  pub(crate) fn format_object(
    &self,
    fmter: &mut Formatter,
    size: u32,
    pos: Position,
    object: &[KeyVal],
  ) -> Option<()> {
    if object.len() == 1 && object[0].0.pos.info != INFO_KEY_VAL {
      return self.format_block(fmter, object, size);
    }
    fmter.out.push('{');
    if !object.is_empty() {
      self.comment_sep(fmter, size, pos.offset + 1);
      self.format_block(fmter, object, size)?;
      self.with_indent(fmter, false, |_fmter| {
        self.comment_sep(_fmter, size, pos.end() - 1);
        Some(())
      })?;
    }
    fmter.out.push('}');
    Some(())
  }
  pub(crate) fn format_type_func(
    &self,
    fmter: &mut Formatter,
    key: &Pos<String>,
    val: &Pos<Json>,
    size: u32,
  ) -> Option<()> {
    fmter.out.push_str(&key.val);
    fmter.out.push('(');
    let val_size = (fmter.indentation + 1) * 2 + self.sizeof_json(val)?;
    let is_block = val.val.is_block()
      || if let Array(_, Lit(array)) = &val.val
        && array.len() == 1
        && array[0].val.is_block()
      {
        true
      } else {
        false
      };
    let is_single_if = if let Array(_, Lit(array)) = &val.val
      && array.len() == 2
      && !matches!(array[0].val.as_type(), ArrayT(_))
    {
      matches!(key.val.as_ref(), "if")
    } else {
      false
    };
    let is_while = key.val == "while";
    let is_define = key.val == "define";
    let do_indent = val_size < LINE_MAX || !is_block;
    let default_indent = LINE_MAX <= size && !is_define && !is_while && !is_single_if;
    if do_indent {
      self.with_indent(fmter, true, |_fmter| {
        self.comment(_fmter, val.pos.offset + 1, default_indent);
        Some(())
      })?;
    } else {
      self.comment(fmter, val.pos.offset + 1, default_indent);
    }
    let mut sig = 0;
    match val {
      Pos { val: Array(_, Lit(args)), .. } => {
        for (idx, item) in args.iter().enumerate() {
          if idx == 0 {
            self.with_indent(fmter, true, |_fmter| self.format_json(_fmter, item));
            continue;
          }
          fmter.out.push(',');
          if is_define && idx < 3 && sig < LINE_MAX {
            fmter.out.push(' ');
            self.with_indent(fmter, true, |_fmter| self.format_json(_fmter, item));
            sig += self.sizeof_json(item)?;
          } else {
            self.format_json_sep(fmter, size, item)?;
          }
        }
      }
      arg => self.with_indent(fmter, true, |_fmter| self.format_json(_fmter, arg))?,
    }
    self.comment(fmter, val.pos.end() - 1, LINE_MAX <= size);
    fmter.out.push(')');
    Some(())
  }
  pub(crate) fn format_type_op(
    &self,
    formatter: &mut Formatter,
    key: &Pos<String>,
    val: &Pos<Json>,
    size: u32,
  ) -> Option<()> {
    let Pos { val: Array(_, Lit(args)), .. } = val else { return None };
    for (idx, item) in args.iter().enumerate() {
      if idx != 0 {
        formatter.out.push_str(&format!(" {} ", key.val));
      }
      if ASSIGN_OP.contains(&key.val.as_ref()) {
        self.format_json(formatter, item)?;
      } else if LINE_MAX <= size || item.val.needs_braces(&key.val, idx) {
        formatter.out.push('{');
        self.format_json_sep(formatter, size, item)?;
        self.comment_sep(formatter, size, item.pos.end() - 1);
        formatter.out.push('}');
      } else {
        self.with_indent(formatter, true, |fmter| self.format_json(fmter, item));
      }
    }
    Some(())
  }
  pub(crate) fn sizeof_key_val(&self, (key, val): &KeyVal) -> Option<u32> {
    if &key.val == "$"
      && let Some(name) = &val.val.as_str()
    {
      u32::try_from(name.len()).ok()
    } else if let Some(value) = value(key, val) {
      self.sizeof_json(value)
    } else {
      let key_len = len_u32(key.val.as_bytes()).ok()?;
      Some(match key.pos.info {
        INFO_NONE | INFO_FUNC => {
          let mut acc = key_len + self.sizeof_json(val)?;
          if let Array(_, Lit(array)) = &val.val {
            if array.is_empty() {
              acc -= 2;
            }
          } else {
            acc += 2;
          }
          acc
        }
        INFO_KEY_VAL => key_len + 2 + self.sizeof_json(val)?,
        INFO_OP => {
          let Array(_, Lit(args)) = &val.val else { return None };
          let mut acc = 2;
          for (idx, item) in args.iter().enumerate() {
            if item.val.needs_braces(&key.val, idx) {
              acc += 4;
            }
            acc += self.sizeof_json(item)? + (key_len + 2);
          }
          acc - (key_len + 2)
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
  pub(crate) fn needs_braces(&self, parent: &str, idx: usize) -> bool {
    let Some(child) = self.operator() else {
      return false;
    };
    let child_prec = op_precedence(&child.val);
    let parent_prec = op_precedence(parent);
    match (child_prec, parent_prec) {
      (None, None | Some(_)) => true,
      (Some(_), None) => false,
      (Some(ch), Some(pa)) => {
        if ch > pa {
          return false;
        }
        if matches!(parent, "+" | "-") && matches!(child.val.as_ref(), "+" | "-") {
          return parent == "-" && idx != 0;
        }
        if matches!(parent, "*" | "/") && matches!(child.val.as_ref(), "*" | "/") {
          return parent == "/" && idx != 0;
        }
        parent != child.val
      }
    }
  }
  pub(crate) fn operator(&self) -> Option<&Pos<String>> {
    if let Object(Lit(obj)) = &self
      && obj.len() == 1
      && obj[0].0.pos.info == INFO_OP
    {
      Some(&obj[0].0)
    } else {
      None
    }
  }
}
pub(crate) fn value<'a>(key: &Pos<String>, val: &'a Pos<Json>) -> Option<&'a Pos<Json>> {
  if &key.val == "value"
    && let Array(_, Lit(array)) = &val.val
    && array.len() == 1
  {
    Some(&array[0])
  } else {
    None
  }
}
