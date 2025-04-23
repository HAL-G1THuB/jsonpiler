//! Implementation of the `JValue`
use {
  super::{JValue, Json},
  core::fmt::{self, Write as _},
};
impl fmt::Display for JValue {
  /// Formats the `Json` object as a compact string without indentation.
  #[expect(clippy::min_ident_chars, reason = "default name is 'f'")]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      JValue::Null => f.write_str("Null"),
      JValue::LBool(bo) => write!(f, "{bo}"),
      JValue::VBool(bv, bit) => write!(f, "({bv}-{bit}: VBool)"),
      JValue::LInt(int) => write!(f, "{int}"),
      JValue::VInt(iv) => write!(f, "({iv}: VInt)"),
      JValue::LFloat(fl) => write!(f, "{fl}"),
      JValue::VFloat(fv) => write!(f, "({fv}: VFloat)"),
      JValue::LString(st) => f.write_str(&escape_string(st)?),
      JValue::VString(sv) => write!(f, "({sv}: VString)"),
      JValue::LArray(ar) => {
        f.write_str("[")?;
        iter_write(ar, f)?;
        f.write_str("]")
      }
      JValue::VArray(av) => write!(f, "({av}: VArray)"),
      JValue::Function(fu) => {
        write!(f, "{}(", fu.name)?;
        iter_write(&fu.params, f)?;
        write!(f, ") -> ")?;
        (*fu.ret).clone().fmt(f)
      }
      JValue::LObject(obj) => {
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
      JValue::VObject(ov) => write!(f, "({ov}: VObject)"),
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
fn iter_write(list: &[Json], out: &mut fmt::Formatter) -> fmt::Result {
  for (i, item) in list.iter().enumerate() {
    if i > 0 {
      out.write_str(", ")?;
    }
    write!(out, "{}", item.value)?;
  }
  Ok(())
}
