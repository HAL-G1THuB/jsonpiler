use super::{
  Bind::{Lit, Var},
  Json, Label,
};
use core::fmt::{self, Write as _};
impl Json {
  pub(crate) fn get_label(self) -> Option<Label> {
    match self {
      Json::Int(Var(label))
      | Json::Float(Var(label))
      | Json::String(Var(label))
      | Json::Bool(Var(label))
      | Json::Array(Var(label))
      | Json::Object(Var(label)) => Some(label),
      Json::Array(_)
      | Json::Bool(_)
      | Json::Float(_)
      | Json::Int(_)
      | Json::Null
      | Json::Object(_)
      | Json::String(_) => None,
    }
  }
  pub(crate) fn type_name(&self) -> String {
    match self {
      Json::Bool(bind) => bind.describe("Bool"),
      Json::Null => "Null".to_owned(),
      Json::Float(bind) => bind.describe("Float"),
      Json::Object(bind) => bind.describe("Object"),
      Json::Int(bind) => bind.describe("Int"),
      Json::String(bind) => bind.describe("String"),
      Json::Array(bind) => bind.describe("Array"),
    }
  }
}
impl fmt::Display for Json {
  #[expect(clippy::min_ident_chars)]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Json::Null => f.write_str("Null"),
      Json::Array(bind) => match bind {
        Lit(array) => {
          f.write_str("[")?;
          for (i, item) in array.iter().enumerate() {
            if i > 0 {
              f.write_str(", ")?;
            }
            write!(f, "{}", item.value)?;
          }
          f.write_str("]")
        }
        Var(_) => f.write_str(&bind.describe("Array")),
      },
      Json::Bool(bind) => match bind {
        Lit(l_bool) => write!(f, "{l_bool}"),
        Var(_) => f.write_str(&bind.describe("Bool")),
      },
      Json::Int(bind) => match bind {
        Lit(l_int) => write!(f, "{l_int}"),
        Var(_) => f.write_str(&bind.describe("Int")),
      },
      Json::Float(bind) => match bind {
        Lit(l_float) => write!(f, "{l_float}"),
        Var(_) => f.write_str(&bind.describe("Float")),
      },
      Json::String(bind) => match bind {
        Lit(l_st) => f.write_str(&escape_string(l_st)?),
        Var(_) => f.write_str(&bind.describe("String")),
      },
      Json::Object(bind) => match bind {
        Lit(obj) => {
          f.write_str("{")?;
          for (i, key_val) in obj.iter().enumerate() {
            if i > 0 {
              f.write_str(", ")?;
            }
            write!(f, "{}: ", escape_string(&key_val.0.value)?)?;
            key_val.1.value.fmt(f)?;
          }
          f.write_str("}")
        }
        Var(_) => f.write_str(&bind.describe("Object")),
      },
    }
  }
}
pub(crate) fn escape_string(unescaped: &str) -> Result<String, fmt::Error> {
  let mut escaped = String::new();
  escaped.push('"');
  let mut chars = unescaped.chars().peekable();
  let mut bytes = Vec::new();
  while let Some(char) = chars.next() {
    if char == '\\' {
      let mut oct_digits = String::new();
      for _ in 0u8..3u8 {
        match chars.next() {
          Some(digit @ '0'..='7') => oct_digits.push(digit),
          Some(ch) => {
            escaped.push('\\');
            escaped.push(ch);
            break;
          }
          None => return Err(fmt::Error),
        }
      }
      let byte = u8::from_str_radix(&oct_digits, 8).map_err(|_| fmt::Error)?;
      bytes.push(byte);
    } else {
      return Err(fmt::Error);
    }
  }
  let utf8_str = str::from_utf8(&bytes).map_err(|_| fmt::Error)?;
  for ch in utf8_str.chars() {
    write!(escaped, "\\u{:04X}", u32::from(ch))?;
  }
  escaped.push('"');
  Ok(escaped)
}
