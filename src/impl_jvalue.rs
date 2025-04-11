use super::{JValue, utility::escape_string};
use core::fmt;
impl JValue {
  /// Recursively writes the `Json` value to the formatter, with indentation based on depth.
  ///
  /// # Arguments
  ///
  /// * `out` - A mutable reference to the `fmt::Formatter`, where the formatted output is
  ///   written.
  /// * `depth` - The current depth of the nested structure, used to control the indentation.
  ///
  /// # Returns
  ///
  /// * `fmt::Result` - The result of the formatting operation, indicating success or failure.
  fn write_json(&self, out: &mut fmt::Formatter, depth: usize) -> fmt::Result {
    match &self {
      JValue::Null => out.write_str("null"),
      JValue::Bool(bo) => write!(out, "{bo}"),
      JValue::BoolVar(bv, bit) => write!(out, "({bv}-{bit}: bool)"),
      JValue::Int(int) => write!(out, "{int}"),
      JValue::IntVar(iv) => write!(out, "({iv}: int)"),
      JValue::Float(fl) => write!(out, "{fl}"),
      JValue::FloatVar(fv) => write!(out, "({fv}: float)"),
      JValue::String(st) => write!(out, "\"{}\"", escape_string(st)?),
      JValue::StringVar(sv) => write!(out, "({sv}: string)"),
      JValue::Array(ar) => {
        out.write_str("[\n")?;
        for (i, item) in ar.iter().enumerate() {
          if i > 0 {
            out.write_str(",\n")?;
          }
          out.write_str(&"  ".repeat(depth + 1))?;
          item.value.write_json(out, depth + 1)?;
        }
        out.write_str("\n")?;
        out.write_str(&"  ".repeat(depth))?;
        out.write_str("]")
      }
      JValue::ArrayVar(av) => write!(out, "({av}: array)"),
      JValue::FuncVar { name: na, params: pa, ret: re } => {
        out.write_str(&format!("{na}("))?;
        for (i, item) in pa.iter().enumerate() {
          if i > 0 {
            out.write_str(", ")?;
          }
          item.value.write_json(out, depth)?;
        }
        out.write_str(") -> ")?;
        (**re).clone().write_json(out, depth)
      }
      JValue::Object(obj) => {
        out.write_str("{\n")?;
        for (i, (key, value)) in obj.iter().enumerate() {
          if i > 0 {
            out.write_str(",\n")?;
          }
          out.write_str(&"  ".repeat(depth + 1))?;
          write!(out, "\"{}\": ", escape_string(key)?)?;
          value.value.write_json(out, depth + 1)?;
        }
        out.write_str("\n")?;
        out.write_str(&"  ".repeat(depth))?;
        out.write_str("}")
      }
      JValue::ObjectVar(ov) => write!(out, "({ov}: object)"),
    }
  }
}
impl fmt::Display for JValue {
  /// Formats the `Json` object as a human-readable string.
  ///
  /// # Arguments
  ///
  /// * `f` - A mutable reference to the `fmt::Formatter`, which is used to write the formatted
  ///   string.
  ///
  /// # Returns
  ///
  /// * `fmt::Result` - The result of the formatting operation, indicating success or failure.impl `fmt::Display` for Json {
  #[expect(clippy::min_ident_chars, reason = "default name is 'f'")]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    self.write_json(f, 0)
  }
}
