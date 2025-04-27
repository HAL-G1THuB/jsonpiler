//! Implementation of the `Json`
use {
  super::{Json, JsonWithPos},
  core::fmt::{self, Write as _},
};
impl Json {
  /// Generate type name.
  pub fn is_literal(&self) -> bool {
    match self {
      Json::LInt(_)
      | Json::LFloat(_)
      | Json::LBool(_)
      | Json::LString(_)
      | Json::Null
      | Json::LObject(_)
      | Json::LArray(_) => true,
      Json::VObject(_)
      | Json::VFloat(_)
      | Json::VInt(_)
      | Json::VString(_)
      | Json::VBool(..)
      | Json::VArray(_)
      | Json::Function(_) => false,
    }
  }
  /// Generate type name.
  pub fn type_name(&self) -> &'static str {
    match self {
      Json::LInt(_) => "LInt",
      Json::VInt(_) => "VInt",
      Json::LFloat(_) => "LFloat",
      Json::VFloat(_) => "VFloat",
      Json::LBool(_) => "LBool",
      Json::VBool(..) => "VBool",
      Json::LString(_) => "LString",
      Json::VString(_) => "VString",
      Json::LArray(_) => "LArray",
      Json::VArray(_) => "VArray",
      Json::LObject(_) => "LObject",
      Json::VObject(_) => "VObject",
      Json::Function(_) => "Function",
      Json::Null => "Null",
    }
  }
}
impl fmt::Display for Json {
  /// Formats the `Json` object as a compact string without indentation.
  #[expect(clippy::min_ident_chars, reason = "default name is 'f'")]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Json::Null => f.write_str("Null"),
      Json::LArray(ar) => {
        f.write_str("[")?;
        iter_write(ar, f)?;
        f.write_str("]")
      }
      Json::VArray(va) => write!(f, "VArray(\"{va}\")"),
      Json::LBool(bo) => write!(f, "{bo}"),
      Json::VBool(vb) => write!(f, "VBool(\"{}\"-{})", vb.name, vb.bit),
      Json::LInt(int) => write!(f, "{int}"),
      Json::VInt(vi) => write!(f, "VInt(\"{vi}\")"),
      Json::LFloat(fl) => write!(f, "{fl}"),
      Json::VFloat(vf) => write!(f, "VFloat(\"{vf}\")"),
      Json::LString(st) => f.write_str(&escape_string(st)?),
      Json::VString(vs) => write!(f, "VString(\"{vs}\")"),
      Json::Function(fu) => {
        write!(f, "{}(", fu.name)?;
        iter_write(&fu.params, f)?;
        write!(f, ") -> ")?;
        (*fu.ret).clone().fmt(f)
      }
      Json::LObject(obj) => {
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
      Json::VObject(vo) => write!(f, "VObject({vo})"),
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
