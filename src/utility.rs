//! Utility functions.
use crate::{JValue, Json};
use core::{
  error::Error,
  fmt::{self, Write as _},
  time::Duration,
};
use std::{process::exit, thread};
/// Decoding base64 variants.
/// # Errors
/// `Box<dyn Error(String)>` - If an invalid encoded value is passed, return `Err`
#[expect(dead_code, reason = "todo")]
pub(crate) fn de64(encoded: &str) -> Result<Vec<u8>, Box<dyn Error>> {
  const ERR: &str = "Unreachable (de64)";
  let mut decoded = Vec::new();
  let mut buffer = 0u32;
  let mut buffer_len = 0u32;
  for ch in encoded.chars() {
    if !('0'..='o').contains(&ch) {
      return Err("Invalid character in input string.".into());
    }
    let val = u32::from(ch).checked_sub(48).ok_or(ERR)?;
    buffer = (buffer << 6u32) | val;
    buffer_len = buffer_len.checked_add(6).ok_or(ERR)?;
    while buffer_len >= 8 {
      let shift = buffer_len.checked_sub(8).ok_or(ERR)?;
      let byte = u8::try_from(buffer >> shift)?;
      decoded.push(byte);
      buffer_len = shift;
      buffer &= (1u32 << buffer_len).checked_sub(1).ok_or(ERR)?;
    }
  }
  Ok(decoded)
}
/// Encoding base64 variants.
/// # Errors
/// Unreachable.
#[expect(dead_code, reason = "todo")]
pub(crate) fn en64(input: &[u8]) -> Result<String, &str> {
  const ERR: &str = "Unreachable (en64)";
  let mut encoded = String::new();
  let chunks = input.chunks(3);
  for chunk in chunks {
    let b0 = chunk.first().unwrap_or(&0u8);
    let b1 = chunk.get(1).unwrap_or(&0u8);
    let b2 = chunk.get(2).unwrap_or(&0u8);
    let enc1 = (b0 >> 2u8) & 0x3F;
    let enc2 = ((b0 << 4u8) | (b1 >> 4u8)) & 0x3F;
    let enc3 = ((b1 << 2u8) | (b2 >> 6u8)) & 0x3F;
    let enc4 = b2 & 0x3F;
    encoded.push(char::from_u32(u32::from(enc1).checked_add(48).ok_or(ERR)?).ok_or(ERR)?);
    encoded.push(char::from_u32(u32::from(enc2).checked_add(48).ok_or(ERR)?).ok_or(ERR)?);
    if chunk.len() >= 2 {
      encoded.push(char::from_u32(u32::from(enc3).checked_add(48).ok_or(ERR)?).ok_or(ERR)?);
    }
    if chunk.len() == 3 {
      encoded.push(char::from_u32(u32::from(enc4).checked_add(48).ok_or(ERR)?).ok_or(ERR)?);
    }
  }
  Ok(encoded)
}
/// Exit the program with exit code 1.
pub(crate) fn error_exit(text: &str) -> ! {
  println!("{text}");
  thread::sleep(Duration::from_secs(1));
  exit(-1)
}
/// Escapes special characters in a string for proper JSON formatting.
/// This method ensures that characters like quotes (`"`) and backslashes (`\`)
/// are escaped in a way that conforms to the JSON specification.
/// It also escapes control characters and non-ASCII characters using Unicode escapes.
/// # Arguments
/// * `s` - The string to be escaped.
/// # Errors
/// * `fmt::Error` - ...
/// # Returns
/// * `String` - The escaped string.
pub(crate) fn escape_string(unescaped: &str) -> Result<String, fmt::Error> {
  let mut escaped = String::new();
  for ch in unescaped.chars() {
    match ch {
      '"' => write!(escaped, r#"\""#)?,
      '\\' => write!(escaped, r"\\")?,
      '\n' => write!(escaped, r"\n")?,
      '\t' => write!(escaped, r"\t")?,
      '\r' => write!(escaped, r"\r")?,
      '\u{08}' => write!(escaped, r"\b")?,
      '\u{0C}' => write!(escaped, r"\f")?,
      u_ch if u_ch < '\u{20}' => write!(escaped, r"\u{:04x}", u32::from(u_ch))?,
      _ => escaped.push(ch),
    }
  }
  Ok(escaped)
}
/// Format error.
/// # Errors
/// `Box<dyn Error>` - Err is always returned.
#[must_use]
pub(crate) fn format_err(text: &str, pos: usize, ln: usize, source: &str) -> String {
  const MSG1: &str = "\nError occurred on line: ";
  const MSG2: &str = "\nError position:\n";
  if source.is_empty() {
    return format!("{text}{MSG1}{ln}{MSG2}Error: Empty input");
  }
  let len = source.len();
  let idx = pos.min(len.saturating_sub(1));
  let start = if idx == 0 {
    0
  } else {
    match source[..idx].rfind('\n') {
      None => 0,
      Some(start_pos) => {
        let Some(res) = start_pos.checked_add(1) else {
          return format!("{text}{MSG1}{ln}{MSG2}Error: Overflow");
        };
        res
      }
    }
  };
  let end = match source[idx..].find('\n') {
    None => len,
    Some(end_pos) => {
      let Some(res) = idx.checked_add(end_pos) else {
        return format!("{text}{MSG1}{ln}{MSG2}Error: Overflow");
      };
      res
    }
  };
  let ws = " ".repeat(idx.saturating_sub(start));
  let result = &source[start..end];
  format!("{text}{MSG1}{ln}{MSG2}{result}\n{ws}^")
}
/// Change the value of another Json to create a new Json.
#[must_use]
pub(crate) const fn obj_json(val: JValue, obj: &Json) -> Json {
  Json { pos: obj.pos, line: obj.line, value: val }
}
