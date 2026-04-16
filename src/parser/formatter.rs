use crate::prelude::*;
impl Pos<Parser> {
  pub(crate) fn comment(
    &mut self,
    out: &mut String,
    offset: u32,
    indentation: u32,
    default_indent: bool,
  ) {
    if self.val.comments.range(..=offset).next().is_none() {
      if default_indent {
        indent(out, indentation);
      }
      return;
    }
    let mut comments = self.val.comments.range(..=offset).clone().peekable();
    if let Some((_, comment)) = comments.peek() {
      if comment.leading {
        indent(out, indentation);
      } else {
        out.push(' ');
      }
    }
    for (_, comment) in comments {
      out.push_str(&comment.text);
      indent(out, indentation);
    }
    self.val.comments.retain(|&key, _| key > offset);
  }
  pub(crate) fn comment_sep(&mut self, out: &mut String, size: u32, offset: u32, indentation: u32) {
    if size < LINE_MAX {
      out.push(' ');
    }
    self.comment(out, offset, indentation, size >= LINE_MAX);
  }
  pub(crate) fn format(&mut self) -> Option<String> {
    let parsed = self.parse_jspl().ok()?;
    let mut out = String::new();
    let size = self.sizeof_json(&parsed)?;
    for (_, comment) in self.val.comments.range(..=parsed.pos.offset) {
      out.push_str(&comment.text);
      out.push('\n');
    }
    self.val.comments.retain(|&key, _| key > parsed.pos.offset);
    if let Object(Lit(object)) = parsed.val {
      self.format_block(&mut out, &object, size, 0)?;
    } else {
      self.format_json(&mut out, &parsed, 0)?;
    }
    self.comment(&mut out, self.pos.offset, 0, true);
    Some(out)
  }
  pub(crate) fn format_array(
    &mut self,
    out: &mut String,
    size: u32,
    array: &[Pos<Json>],
    indentation: u32,
  ) -> Option<()> {
    if array.is_empty() {
      out.push_str("[]");
      return Some(());
    }
    out.push('[');
    for (idx, item) in array.iter().enumerate() {
      if idx != 0 {
        out.push(',');
      }
      self.format_json_sep(out, size, item.pos.offset + 1, item, indentation)?;
    }
    self.comment_sep(out, size, self.pos.offset, indentation);
    out.push(']');
    Some(())
  }
  pub(crate) fn format_json(
    &mut self,
    out: &mut String,
    json: &Pos<Json>,
    indentation: u32,
  ) -> Option<()> {
    match &json.val {
      Null(Lit(())) | Int(Lit(_)) | Bool(Lit(_)) | Str(Lit(_)) | Float(Lit(_)) => {
        out.push_str(self.get_slice(json.pos).ok()?);
      }
      Array(Lit(array)) => {
        let size = self.sizeof_json(json)? + indentation * 2;
        self.format_array(out, size, array, indentation)?;
      }
      Object(Lit(object)) => {
        let size = self.sizeof_json(json)? + indentation * 2;
        self.format_object(out, size, json.pos, object, indentation)?;
      }
      Null(Var(_)) | Int(Var(_)) | Str(Var(_)) | Object(Var(_)) | Array(Var(_)) | Bool(Var(_))
      | Float(Var(_)) => return None,
    }
    Some(())
  }
  pub(crate) fn format_json_sep(
    &mut self,
    out: &mut String,
    size: u32,
    offset: u32,
    json: &Pos<Json>,
    indentation: u32,
  ) -> Option<()> {
    let val_size = (indentation + 1) * 2 + self.sizeof_json(json)?;
    self.comment_sep(
      out,
      size,
      offset,
      indentation + u32::from(val_size < LINE_MAX || !json.val.is_block()),
    );
    self.format_json(out, json, indentation + 1)
  }
  pub(crate) fn sizeof_json(&self, json: &Pos<Json>) -> Option<u32> {
    match &json.val {
      Null(Lit(())) | Int(Lit(_)) | Bool(Lit(_)) | Float(Lit(_)) | Str(Lit(_)) => {
        Some(json.pos.size)
      }
      Array(Lit(array)) => {
        let mut acc = 2;
        for item in array {
          acc += self.sizeof_json(item)? + 2;
        }
        Some(acc)
      }
      Object(Lit(object)) => {
        if object.len() == 1 {
          self.sizeof_key_val(&object[0])
        } else {
          let mut acc = 2;
          for key_val in object {
            acc += self.sizeof_key_val(key_val)? + 2;
          }
          Some(acc)
        }
      }
      Null(_) | Array(_) | Bool(_) | Float(_) | Int(_) | Object(_) | Str(_) => None,
    }
  }
}
fn indent(out: &mut String, indentation: u32) {
  out.push('\n');
  out.push_str(&"  ".repeat(indentation as usize));
}
