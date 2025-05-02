//! Implementation of the `Json`
use super::{
  AsmBool, Bind, Json, JsonWithPos, Name,
  Var::{self, Global, Local, Tmp},
};
use core::{
  fmt::{self, Write as _},
  mem::take,
};
impl Json {
  /// Determines if it is a temporary value.
  pub fn tmp(&self) -> Option<(usize, usize)> {
    match self {
      Json::LBool(_) | Json::Null | Json::VBool(_) | Json::Function(_) => None,
      Json::Object(bind) => bind_match(bind, 8),
      Json::Float(bind) => bind_match(bind, 8),
      Json::Int(bind) => bind_match(bind, 8),
      Json::String(bind) => bind_match(bind, 8),
      Json::Array(bind) => bind_match(bind, 8),
    }
  }
  /// Converts temporary value to Local variable.
  pub fn tmp_to_local(&mut self) -> Self {
    match *self {
      Json::Object(Bind::Var(Name { var: Tmp, seed })) => {
        Json::Object(Bind::Var(Name { var: Local, seed }))
      }
      Json::Array(Bind::Var(Name { var: Tmp, seed })) => {
        Json::Array(Bind::Var(Name { var: Local, seed }))
      }
      Json::Float(Bind::Var(Name { var: Tmp, seed })) => {
        Json::Float(Bind::Var(Name { var: Local, seed }))
      }
      Json::String(Bind::Var(Name { var: Tmp, seed })) => {
        Json::String(Bind::Var(Name { var: Local, seed }))
      }
      Json::Int(Bind::Var(Name { var: Tmp, seed })) => {
        Json::Int(Bind::Var(Name { var: Local, seed }))
      }
      Json::VBool(AsmBool { name: Name { var: Tmp, seed }, bit }) => {
        Json::VBool(AsmBool { name: Name { var: Local, seed }, bit })
      }
      Json::String(_)
      | Json::Int(_)
      | Json::Object(_)
      | Json::Float(_)
      | Json::Array(_)
      | Json::LBool(_)
      | Json::VBool(_)
      | Json::Function(_)
      | Json::Null => take(self),
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
  /// Determines if it is a global variable.
  pub fn var(&self) -> Option<Var> {
    match self {
      Json::Function(_) => Some(Global),
      Json::VBool(AsmBool { name, .. })
      | Json::Object(Bind::Var(name))
      | Json::Float(Bind::Var(name))
      | Json::Int(Bind::Var(name))
      | Json::String(Bind::Var(name))
      | Json::Array(Bind::Var(name)) => Some(name.var),
      Json::String(_)
      | Json::Int(_)
      | Json::Object(_)
      | Json::Float(_)
      | Json::Array(_)
      | Json::LBool(_)
      | Json::Null => None,
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
        Bind::Var(name) => write!(f, "VArray(qword{name})"),
      },
      Json::LBool(bo) => write!(f, "{bo}"),
      Json::VBool(asm_bool) => write!(f, "VBool(qword{}-{})", asm_bool.name, asm_bool.bit),
      Json::Int(int) => match int {
        Bind::Lit(l_int) => write!(f, "{l_int}"),
        Bind::Var(name) => write!(f, "VInt(qword{name})"),
      },
      Json::Float(float) => match float {
        Bind::Lit(l_float) => write!(f, "{l_float}"),
        Bind::Var(name) => write!(f, "VFloat(qword{name})"),
      },
      Json::String(st) => match st {
        Bind::Lit(l_st) => f.write_str(&escape_string(l_st)?),
        Bind::Var(name) => write!(f, "VString(qword{name})"),
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
        Bind::Var(name) => write!(f, "VObject(qword{name})"),
      },
    }
  }
}
/// Pattern match of the `Bind`.
fn bind_match<T>(bind: &Bind<T>, byte: usize) -> Option<(usize, usize)> {
  match bind {
    Bind::Var(name) => Some((name.seed, byte)),
    Bind::Lit(_) => None,
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
