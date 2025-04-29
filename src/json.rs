//! Implementation of the `Json`
use super::{Bind, Json, JsonWithPos};
use core::{
  fmt::{self, Write as _},
  mem::take,
};
impl Json {
  /// Determines if it is a literal.
  pub fn is_literal(&self) -> bool {
    match self {
      Json::LBool(_) | Json::Null => true,
      Json::VBool(..) | Json::Function(_) => false,
      Json::Object(object) => matches!(object, Bind::Lit(_)),
      Json::Float(float) => matches!(float, Bind::Lit(_)),
      Json::Int(int) => matches!(int, Bind::Lit(_)),
      Json::String(string) => matches!(string, Bind::Lit(_)),
      Json::Array(array) => matches!(array, Bind::Lit(_)),
    }
  }
  /// Determines if it is a temporary value.
  pub fn tmp(&self) -> Option<(usize, usize)> {
    match self {
      Json::LBool(_) | Json::Null | Json::VBool(_) | Json::Function(_) => None,
      Json::Object(object) => match object {
        Bind::Tmp(local) => Some((*local, 8)),
        _ => None,
      },
      Json::Float(float) => match float {
        Bind::Tmp(local) => Some((*local, 8)),
        _ => None,
      },
      Json::Int(int) => match int {
        Bind::Tmp(local) => Some((*local, 8)),
        _ => None,
      },
      Json::String(string) => match string {
        Bind::Tmp(local) => Some((*local, 8)),
        _ => None,
      },
      Json::Array(array) => match array {
        Bind::Tmp(local) => Some((*local, 8)),
        _ => None,
      },
    }
  }
  /// Converts temporary value to Local variable.
  pub fn tmp_to_local(&mut self) -> Self {
    match self {
      Json::Object(Bind::Tmp(local)) => Json::Object(Bind::Local(*local)),
      Json::Array(Bind::Tmp(local)) => Json::Array(Bind::Local(*local)),
      Json::Float(Bind::Tmp(local)) => Json::Float(Bind::Local(*local)),
      Json::String(Bind::Tmp(local)) => Json::String(Bind::Local(*local)),
      Json::Int(Bind::Tmp(local)) => Json::Int(Bind::Local(*local)),
      _ => take(self),
    }
  }
  /// Generate type name.
  pub fn type_name(&self) -> String {
    match self {
      Json::LBool(_) => "LBool".to_owned(),
      Json::Null => "Null".to_owned(),
      Json::VBool(_) => "VBool".to_owned(),
      Json::Function(_) => "Function".to_owned(),
      Json::Float(float) => bind_name(float, "Float"),
      Json::Object(obj) => bind_name(obj, "Object"),
      Json::Int(int) => bind_name(int, "Int"),
      Json::String(st) => bind_name(st, "String"),
      Json::Array(arr) => bind_name(arr, "Array"),
    }
  }
}
impl fmt::Display for Json {
  /// Formats the `Json` object as a compact string without indentation.
  #[expect(clippy::min_ident_chars, reason = "default name is 'f'")]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Json::Null => f.write_str("Null"),
      Json::Array(ar) => match ar {
        Bind::Lit(array) => {
          f.write_str("[")?;
          iter_write(array, f)?;
          f.write_str("]")
        }
        Bind::Var(var) => write!(f, "VArray(\"{var}\")"),
        Bind::Local(local) | Bind::Tmp(local) => write!(f, "VArray(\"{local}\")"),
      },
      Json::LBool(bo) => write!(f, "{bo}"),
      Json::VBool(vb) => write!(f, "VBool(\"{}\"-{})", vb.name, vb.bit),
      Json::Int(int) => match int {
        Bind::Lit(l_int) => write!(f, "{l_int}"),
        Bind::Var(var) => write!(f, "VInt(\"{var}\")"),
        Bind::Local(local) | Bind::Tmp(local) => write!(f, "VInt(\"{local}\")"),
      },
      Json::Float(float) => match float {
        Bind::Lit(flt) => write!(f, "{flt}"),
        Bind::Var(var) => write!(f, "VFloat(\"{var}\")"),
        Bind::Local(local) | Bind::Tmp(local) => write!(f, "VFloat(\"{local}\")"),
      },
      Json::String(st) => match st {
        Bind::Lit(lstr) => f.write_str(&escape_string(lstr)?),
        Bind::Var(var) => write!(f, "VString(\"{var}\")"),
        Bind::Local(local) | Bind::Tmp(local) => write!(f, "VString(\"{local}\")"),
      },
      Json::Function(fu) => {
        write!(f, "{}(", fu.name)?;
        iter_write(&fu.params, f)?;
        write!(f, ") -> ")?;
        (*fu.ret).clone().fmt(f)
      }
      Json::Object(object) => match object {
        Bind::Lit(obj) => {
          f.write_str("{")?;
          for (i, kv) in obj.iter().enumerate() {
            if i > 0 {
              f.write_str(", ")?;
            }
            write!(f, "{}: ", escape_string(&kv.0)?)?;
            kv.1.value.fmt(f)?;
          }
          f.write_str("}")
        }
        Bind::Var(var) => write!(f, "VObject({var})"),
        Bind::Local(local) | Bind::Tmp(local) => write!(f, "VObject(\"{local}\")"),
      },
    }
  }
}
/// gets name of the `Bind`.
fn bind_name<T>(bind: &Bind<T>, ty: &str) -> String {
  let mut l_or_v = if matches!(bind, Bind::Lit(_)) { "L" } else { "V" }.to_owned();
  l_or_v.push_str(ty);
  l_or_v
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
