use crate::prelude::*;
use std::fmt;
impl Json {
  pub(crate) fn describe(&self) -> String {
    format!(
      "{}{}",
      self.type_name(),
      match self {
        Bool(bind) => format!("{bind}"),
        Null => String::new(),
        Float(bind) => format!("{bind}"),
        Object(bind) => format!("{bind}"),
        Int(bind) => format!("{bind}"),
        Str(bind) => format!("{bind}"),
        Array(bind) => format!("{bind}"),
      }
    )
  }
  pub(crate) fn label(&mut self) -> Option<&mut Label> {
    match self {
      Int(Var(label)) | Float(Var(label)) | Str(Var(label)) | Bool(Var(label))
      | Array(Var(label)) | Object(Var(label)) => Some(label),
      Array(_) | Bool(_) | Float(_) | Int(_) | Null | Object(_) | Str(_) => None,
    }
  }
  pub(crate) fn type_name(&self) -> &'static str {
    match self {
      Bool(_) => "Bool",
      Null => "Null",
      Float(_) => "Float",
      Object(_) => "Object",
      Int(_) => "Int",
      Str(_) => "Str",
      Array(_) => "Array",
    }
  }
}
impl fmt::Display for Json {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Array(Lit(array)) => {
        let content = array.iter().map(|item| format!("{}", &item.val));
        write!(f, "[{}]", &content.collect::<Vec<_>>().join(", "))
      }
      Bool(Lit(lit)) => lit.fmt(f),
      Int(Lit(lit)) => lit.fmt(f),
      Float(Lit(lit)) => lit.fmt(f),
      Str(Lit(lit)) => lit.fmt(f),
      Object(Lit(obj)) => {
        let content = obj.iter().map(|(key, val)| format!("{}: {}", &key.val, val.val));
        write!(f, "{{{}}}", &content.collect::<Vec<_>>().join(", "))
      }
      Null => write!(f, "null"),
      Array(Var(label)) | Bool(Var(label)) | Int(Var(label)) | Float(Var(label))
      | Str(Var(label)) | Object(Var(label)) => label.fmt(f),
    }
  }
}
impl<T> fmt::Display for Bind<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Var(label) => write!(f, "{label}"),
      Lit(_) => f.write_str(" (Literal)"),
    }
  }
}
impl fmt::Display for Label {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self.0 {
      Local(Tmp, _) => Ok(()),
      Local(Long, _) => write!(f, " (Local variable)"),
      Global(_) => write!(f, " (Global variable)"),
    }
  }
}
