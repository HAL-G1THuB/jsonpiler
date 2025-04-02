use super::{JValue, Json};
use std::fmt;
impl fmt::Display for Json {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    self.write_json(f, 0)
  }
}
impl Json {
  fn write_json(&self, out: &mut fmt::Formatter, depth: usize) -> fmt::Result {
    match &self.value {
      JValue::Null => out.write_str("null"),
      JValue::Bool(b) => write!(out, "{}", b),
      JValue::BoolVar(bv) => write!(out, "({bv}: bool)"),
      JValue::Int(i) => write!(out, "{i}"),
      JValue::IntVar(v) => write!(out, "({v}: int)"),
      JValue::Float(f) => write!(out, "{f}"),
      JValue::FloatVar(v) => write!(out, "({v}: float)"),
      JValue::String(s) => write!(out, "\"{}\"", self.escape_string(s)),
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
          write!(out, "\"{}\": ", self.escape_string(k))?;
          v.write_json(out, depth + 1)?;
        }
        out.write_str("\n")?;
        out.write_str(&"  ".repeat(depth))?;
        out.write_str("}")
      }
      JValue::ObjectVar(v) => write!(out, "({v}: object)"),
    }
  }
  fn escape_string(&self, s: &str) -> String {
    let mut escaped = String::new();
    for c in s.chars() {
      match c {
        '"' => escaped.push_str("\\\""),
        '\\' => escaped.push_str("\\\\"),
        '\n' => escaped.push_str("\\n"),
        '\t' => escaped.push_str("\\t"),
        '\r' => escaped.push_str("\\r"),
        '\u{08}' => escaped.push_str("\\b"),
        '\u{0C}' => escaped.push_str("\\f"),
        c if c < '\u{20}' => escaped.push_str(&format!("\\u{:04x}", c as u32)),
        _ => escaped.push(c),
      }
    }
    escaped
  }
}
