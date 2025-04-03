use super::{JValue, Json, utility::escape_string};
use std::fmt;
impl fmt::Display for Json {
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
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    self.write_json(f, 0)
  }
}
impl Json {
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
    match &self.value {
      JValue::Null => out.write_str("null"),
      JValue::Bool(b) => write!(out, "{b}"),
      JValue::BoolVar(bv) => write!(out, "({bv}: bool)"),
      JValue::Int(i) => write!(out, "{i}"),
      JValue::IntVar(v) => write!(out, "({v}: int)"),
      JValue::Float(f) => write!(out, "{f}"),
      JValue::FloatVar(v) => write!(out, "({v}: float)"),
      JValue::String(s) => write!(out, "\"{}\"", escape_string(s)),
      JValue::StringVar(v) => write!(out, "({v}: string)"),
      JValue::Array(a) => {
        out.write_str("[\n")?;
        for (i, item) in a.iter().enumerate() {
          if i > 0 {
            out.write_str(",\n")?;
          }
          out.write_str(&"  ".repeat(depth + 1))?;
          item.write_json(out, depth + 1)?;
        }
        out.write_str("\n")?;
        out.write_str(&"  ".repeat(depth))?;
        out.write_str("]")
      }
      JValue::ArrayVar(v) => write!(out, "({v}: array)"),
      JValue::FuncVar(name, params) => {
        out.write_str(&format!("{name}(",))?;
        for (i, item) in params.iter().enumerate() {
          if i > 0 {
            out.write_str(", ")?;
          }
          item.write_json(out, depth)?;
        }
        out.write_str(")")
      }
      JValue::Object(o) => {
        out.write_str("{\n")?;
        for (i, (k, v)) in o.iter().enumerate() {
          if i > 0 {
            out.write_str(",\n")?;
          }
          out.write_str(&"  ".repeat(depth + 1))?;
          write!(out, "\"{}\": ", escape_string(k))?;
          v.write_json(out, depth + 1)?;
        }
        out.write_str("\n")?;
        out.write_str(&"  ".repeat(depth))?;
        out.write_str("}")
      }
      JValue::ObjectVar(v) => write!(out, "({v}: object)"),
    }
  }
}
