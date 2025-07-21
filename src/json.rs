//! Implementation of the `Json`.
use super::{
  AsmBool,
  Bind::{self, Lit, Var},
  Json, JsonWithPos, Name,
  VarKind::Tmp,
};
use core::fmt::{self, Write as _};
impl Json {
  /// Determines if it is a temporary value.
  pub fn tmp(&self) -> Option<(usize, usize)> {
    fn get_id<T>(bind: &Bind<T>) -> Option<usize> {
      match bind {
        Var(Name { var: Tmp, id }) => Some(*id),
        Var(_) | Lit(_) => None,
      }
    }
    match self {
      Json::LBool(_) | Json::Null | Json::VBool(_) | Json::Function(_) => None,
      Json::Object(bind) => Some((get_id(bind)?, 8)),
      Json::Float(bind) => Some((get_id(bind)?, 8)),
      Json::Int(bind) => Some((get_id(bind)?, 8)),
      Json::String(bind) => Some((get_id(bind)?, 8)),
      Json::Array(bind) => Some((get_id(bind)?, 8)),
    }
  }
  /// Generate type name.
  pub fn type_name(&self) -> String {
    match self {
      Json::LBool(_) => "Bool (Literal)".to_owned(),
      Json::VBool(AsmBool { name, .. }) => format!("Bool ({})", name.describe()),
      Json::Null => "Null".to_owned(),
      Json::Function(_) => "Function".to_owned(),
      Json::Float(bind) => bind.describe("Float"),
      Json::Object(bind) => bind.describe("Object"),
      Json::Int(bind) => bind.describe("Int"),
      Json::String(bind) => bind.describe("String"),
      Json::Array(bind) => bind.describe("Array"),
    }
  }
}
impl fmt::Display for Json {
  /// Formats the `Json` object as a compact string without indentation.
  #[expect(clippy::min_ident_chars, reason = "default name is 'f'")]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Json::Null => f.write_str("Null"),
      Json::Array(bind) => match bind {
        Lit(array) => {
          f.write_str("[")?;
          iter_write(array, f)?;
          f.write_str("]")
        }
        Var(_) => f.write_str(&bind.describe("Array")),
      },
      Json::LBool(bo) => write!(f, "{bo}"),
      Json::VBool(_) => write!(f, "Bool"),
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
      Json::Function(asm_func) => {
        write!(f, "{}(", asm_func.name)?;
        iter_write(&asm_func.params, f)?;
        write!(f, ") -> ")?;
        (*asm_func.ret).fmt(f)
      }
      Json::Object(bind) => match bind {
        Lit(obj) => {
          f.write_str("{")?;
          for (i, key_val) in obj.iter().enumerate() {
            if i > 0 {
              f.write_str(", ")?;
            }
            write!(f, "{}: ", escape_string(&key_val.0)?)?;
            key_val.1.value.fmt(f)?;
          }
          f.write_str("}")
        }
        Var(_) => f.write_str(&bind.describe("Object")),
      },
    }
  }
}
/// Escapes special characters in a string for proper JSON formatting.
pub(crate) fn escape_string(unescaped: &str) -> Result<String, fmt::Error> {
  let mut escaped = String::new();
  escaped.push('"');
  for ch in unescaped.chars() {
    match ch {
      '"' => write!(escaped, r#"\""#)?,
      '\\' => write!(escaped, r"\\")?,
      '\n' => write!(escaped, r"\n")?,
      '\t' => write!(escaped, r"\t")?,
      '\r' => write!(escaped, r"\r")?,
      '\u{08}' => write!(escaped, r"\b")?,
      '\u{0C}' => write!(escaped, r"\f")?,
      u_ch if u_ch < '\u{20}' => write!(escaped, r"\u{:04x}", u32::from(ch))?,
      _ => escaped.push(ch),
    }
  }
  escaped.push('"');
  Ok(escaped)
}
/// Iterates over a list of `Json` objects and writes them without indentation.
fn iter_write(list: &[JsonWithPos], out: &mut fmt::Formatter) -> fmt::Result {
  for (i, item) in list.iter().enumerate() {
    if i > 0 {
      out.write_str(", ")?;
    }
    write!(out, "{}", item.value)?;
  }
  Ok(())
}
