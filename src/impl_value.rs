//! Implementation of the `JValue`
use super::{JValue, Json, functions::escape_string};
use core::fmt;
impl JValue {
  /// Recursively writes the `Json` value to the formatter, with indentation based on depth.
  fn write_json(&self, out: &mut fmt::Formatter, depth: usize) -> fmt::Result {
    match self {
      JValue::Null => out.write_str("null"),
      JValue::Bool(bo) => write!(out, "{bo}"),
      JValue::BoolVar(bv, bit) => write!(out, "({bv}-{bit}: bool)"),
      JValue::Int(int) => write!(out, "{int}"),
      JValue::IntVar(iv) => write!(out, "({iv}: int)"),
      JValue::Float(fl) => write!(out, "{fl}"),
      JValue::FloatVar(fv) => write!(out, "({fv}: float)"),
      JValue::String(st) => out.write_str(&escape_string(st)?),
      JValue::StringVar(sv) => write!(out, "({sv}: string)"),
      JValue::Array(ar) => {
        iter_write(ar, out, depth.saturating_add(1))?;
        out.write_str(&format!("\n{}]", "  ".repeat(depth)))
      }
      JValue::ArrayVar(av) => write!(out, "({av}: array)"),
      JValue::Function { name: na, params: pa, ret: re } => {
        out.write_str(&format!("{na}("))?;
        iter_write(pa, out, depth.saturating_add(1))?;
        out.write_str(") -> ")?;
        (*re).clone().write_json(out, depth)
      }
      JValue::Object(obj) => {
        out.write_str("{\n")?;
        for (i, kv) in obj.iter().enumerate() {
          if i > 0 {
            out.write_str(",\n")?;
          }
          out.write_str(&"  ".repeat(depth.saturating_add(1)))?;
          out.write_str(&escape_string(&kv.0)?)?;
          write!(out, ": ")?;
          kv.1.value.write_json(out, depth.saturating_add(1))?;
        }
        out.write_str(&format!("\n{}}}", "  ".repeat(depth)))
      }
      JValue::ObjectVar(ov) => write!(out, "({ov}: object)"),
    }
  }
}
impl fmt::Display for JValue {
  /// Formats the `Json` object as a human-readable string.
  /// # Arguments
  /// * `f: fmt::Formatter`  - Used to write the formatted string.
  /// # Returns
  /// * `fmt::Result` - The result of the formatting operation, indicating success or failure.
  #[expect(clippy::min_ident_chars, reason = "default name is 'f'")]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    self.write_json(f, 0)
  }
}
/// Iterates over a list of `Json` objects and writes them to the formatter.
fn iter_write(list: &[Json], out: &mut fmt::Formatter, depth: usize) -> fmt::Result {
  for (i, item) in list.iter().enumerate() {
    if i > 0 {
      out.write_str(",\n")?;
    }
    out.write_str(&"  ".repeat(depth))?;
    item.value.write_json(out, depth)?;
  }
  Ok(())
}
