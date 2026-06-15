use crate::prelude::*;
pub(crate) struct Formatter {
  pub comment_off: u32,
  pub indentation: u32,
  pub out: String,
}
impl Formatter {
  pub(crate) fn indent(&mut self) {
    self.out.push('\n');
    self.out.push_str(&"  ".repeat(self.indentation as usize));
  }
  pub(crate) fn new() -> Self {
    Formatter { out: String::new(), comment_off: 0, indentation: 0 }
  }
  pub(crate) fn update_indent(&mut self, plus: bool) {
    if plus {
      self.indentation += 1;
    } else {
      self.indentation = self.indentation.saturating_sub(1);
    }
  }
}
impl Pos<Parser> {
  pub(crate) fn comment(&self, fmter: &mut Formatter, offset: u32, default_indent: bool) {
    if self.val.comments.range(fmter.comment_off..=offset).next().is_none() {
      if default_indent {
        fmter.indent();
      }
      return;
    }
    let mut comments = self.val.comments.range(fmter.comment_off..=offset).clone().peekable();
    if let Some((_, comment)) = comments.peek() {
      if comment.leading {
        fmter.indent();
      } else {
        fmter.out.push(' ');
      }
    }
    for (_, comment) in comments {
      fmter.out.push_str(&comment.text);
      fmter.indent();
    }
    fmter.comment_off = offset;
  }
  pub(crate) fn comment_sep(&self, fmter: &mut Formatter, size: u32, offset: u32) {
    if size < LINE_MAX {
      fmter.out.push(' ');
    }
    self.comment(fmter, offset, size >= LINE_MAX);
  }
  pub(crate) fn format(&mut self, path: &str) -> Option<String> {
    let mut fmter = Formatter::new();
    let parsed = self.parse_jspl(path).ok()?;
    let size = self.sizeof_json(&parsed)?;
    for (_, comment) in self.val.comments.range(fmter.comment_off..=parsed.pos.offset) {
      fmter.out.push_str(&comment.text);
      fmter.out.push('\n');
    }
    fmter.comment_off = parsed.pos.offset;
    if matches!(parsed.val, Null(Lit(()))) {
      self.comment(&mut fmter, self.pos.end(), false);
      return Some(String::new());
    } else if let Object(Lit(object)) = parsed.val {
      self.format_block(&mut fmter, &object, size)?;
    } else {
      self.format_json(&mut fmter, &parsed)?;
    }
    self.comment(&mut fmter, self.pos.offset, true);
    Some(fmter.out)
  }
  pub(crate) fn format_array(
    &self,
    fmter: &mut Formatter,
    size: u32,
    pos: Position,
    array: &[Pos<Json>],
  ) -> Option<()> {
    fmter.out.push('[');
    for (idx, item) in array.iter().enumerate() {
      if idx != 0 {
        fmter.out.push(',');
      }
      self.format_json_sep(fmter, size, item)?;
    }
    if !array.is_empty() {
      self.comment_sep(fmter, size, pos.end() - 1);
    }
    fmter.out.push(']');
    Some(())
  }
  pub(crate) fn format_json(&self, fmter: &mut Formatter, json: &Pos<Json>) -> Option<()> {
    let size = self.sizeof_json(json)? + fmter.indentation * 2;
    match &json.val {
      Null(Lit(())) | Int(Lit(_)) | Bool(Lit(_)) | Str(Lit(_)) | Float(Lit(_)) => {
        fmter.out.push_str(self.get_slice(json.pos).ok()?);
      }
      Array(_, Lit(array)) => self.format_array(fmter, size, json.pos, array)?,
      Object(Lit(object)) => self.format_object(fmter, size, json.pos, object)?,
      Null(Var(_))
      | Int(Var(_))
      | Str(Var(_))
      | Object(Var(_))
      | Array(_, Var(_))
      | Bool(Var(_))
      | Float(Var(_)) => return None,
    }
    Some(())
  }
  pub(crate) fn format_json_sep(
    &self,
    fmter: &mut Formatter,
    size: u32,
    json: &Pos<Json>,
  ) -> Option<()> {
    let val_size = (fmter.indentation + 1) * 2 + self.sizeof_json(json)?;
    let do_indent = val_size < LINE_MAX || !json.val.is_block();
    if do_indent {
      self.with_indent(fmter, true, |_fmter| {
        self.comment_sep(_fmter, size, json.pos.offset + 1);
        Some(())
      });
    } else {
      self.comment_sep(fmter, size, json.pos.offset + 1);
    }
    self.with_indent(fmter, true, |_fmter| self.format_json(_fmter, json));
    Some(())
  }
  pub(crate) fn sizeof_json(&self, json: &Pos<Json>) -> Option<u32> {
    match &json.val {
      Null(Lit(())) | Int(Lit(_)) | Bool(Lit(_)) | Float(Lit(_)) | Str(Lit(_)) => {
        Some(json.pos.size)
      }
      Array(_, Lit(array)) => sizeof_container(array, |item| self.sizeof_json(item)),
      Object(Lit(object)) => {
        if object.len() == 1 {
          self.sizeof_key_val(&object[0])
        } else {
          sizeof_container(object, |key_val| self.sizeof_key_val(key_val))
        }
      }
      Null(_) | Array(..) | Bool(_) | Float(_) | Int(_) | Object(_) | Str(_) => None,
    }
  }
  pub(crate) fn with_indent<F: FnOnce(&mut Formatter) -> Option<()>>(
    &self,
    fmter: &mut Formatter,
    plus: bool,
    fmt_f: F,
  ) -> Option<()> {
    fmter.update_indent(plus);
    let result = fmt_f(fmter);
    fmter.update_indent(!plus);
    result
  }
}
pub(crate) fn sizeof_container<T, F: Fn(&T) -> Option<u32>>(
  container: &[T],
  sizeof_f: F,
) -> Option<u32> {
  container.iter().try_fold(2, |acc, item| Some(acc + sizeof_f(item)? + 2))
}
