//! Implementation of the `JValue`
use {
  super::{AsmFunc, JValue, Json, functions::escape_string},
  core::fmt,
};

impl fmt::Display for JValue {
  /// Formats the `Json` object as a compact string without indentation.
  #[expect(clippy::min_ident_chars, reason = "default name is 'f'")]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      JValue::Null => f.write_str("null"),
      JValue::LBool(bo) => write!(f, "{bo}"),
      JValue::VBool(bv, bit) => write!(f, "({bv}-{bit}: bool)"),
      JValue::LInt(int) => write!(f, "{int}"),
      JValue::VInt(iv) => write!(f, "({iv}: int)"),
      JValue::LFloat(fl) => write!(f, "{fl}"),
      JValue::VFloat(fv) => write!(f, "({fv}: float)"),
      JValue::LString(st) => f.write_str(&escape_string(st)?),
      JValue::VString(sv) => write!(f, "({sv}: string)"),
      JValue::LArray(ar) => {
        f.write_str("[")?;
        iter_write(ar, f)?;
        f.write_str("]")
      }
      JValue::VArray(av) => write!(f, "({av}: array)"),
      JValue::Function(AsmFunc { name: na, params: pa, ret: re }) => {
        write!(f, "{na}(")?;
        iter_write(pa, f)?;
        write!(f, ") -> ")?;
        (*re).clone().fmt(f)
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
      JValue::VObject(ov) => write!(f, "({ov}: object)"),
    }
  }
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
