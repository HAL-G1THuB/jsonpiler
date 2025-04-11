//! Utility functions.
use crate::{JValue, Json};
use core::{
  error::Error,
  fmt::{self, Write as _},
};
use std::{io, process::exit};
/// Format error.
///
/// # Examples
///
/// ```rust
/// use jsompiler::{JError, utility::format_err};
/// use std::any::Any;
/// use std::error::Error;
/// assert_eq!(
///   *format_err("", 0, 0, "").err().unwrap().downcast_ref::<JError>().unwrap().0,
///   String::from("\nError occurred on line: 0\nError position:\nError: Empty input")
/// );
/// assert_eq!(
///   *format_err("Error!", 8, 3, "ok!\nok!\nError!!!").err().unwrap().downcast_ref::<JError>().unwrap().0,
///   String::from("Error!\nError occurred on line: 3\nError position:\nError!!!\n^")
/// );
/// ```
/// # Errors
/// `Box(JError)` - Err is always returned.
#[must_use]
pub fn format_err(text: &str, index: usize, ln: usize, input_code: &str) -> String {
  if input_code.is_empty() {
    return format!("{text}\nError occurred on line: {ln}\nError position:\nError: Empty input");
  }
  let len = input_code.len();
  let idx = index.min(len.saturating_sub(1));
  let start = if idx > 0 { input_code[..idx].rfind('\n').map_or(0, |pos| pos + 1) } else { 0 };
  let end = input_code[idx..].find('\n').map_or(len, |pos| idx + pos);
  let ws = " ".repeat(idx.saturating_sub(start));
  let result = &input_code[start..end];
  format!("{text}\nError occurred on line: {ln}\nError position:\n{result}\n{ws}^")
}
/// Exit the program with exit code 1.
///
/// # Examples
///
/// ```should_panic
/// use std::process::{Command, Stdio};
/// use std::thread::spawn;
/// use jsompiler::utility::error_exit;
/// let mut child = Command::new("echo")
///   .arg("")
///   .stdout(Stdio::piped())
///   .spawn();
/// #[should_panic]
///   error_exit("Error!")
/// ```
pub fn error_exit(text: &str) -> ! {
  let mut nu = String::new();
  eprint!("{text}\nPress Enter to exit:");
  let _ = io::stdin().read_line(&mut nu);
  exit(1)
}
/// Encoding base64 variants.
///
/// # Examples
///
/// ```rust
/// use jsompiler::utility::en64;
/// assert_eq!(en64(b"0"), String::from("<0"))
/// ```
#[must_use]
#[expect(dead_code, reason = "todo")]
pub fn en64(input: &[u8]) -> String {
  let mut encoded = String::new();
  let chunks = input.chunks(3);
  for chunk in chunks {
    let (b0, b1, b2) = match chunk.len() {
      3 => (chunk[0], chunk[1], chunk[2]),
      2 => (chunk[0], chunk[1], 0),
      1 => (chunk[0], 0, 0),
      _ => unreachable!(),
    };
    let enc1 = (b0 >> 2) & 0x3F;
    let enc2 = ((b0 << 4) | (b1 >> 4)) & 0x3F;
    let enc3 = ((b1 << 2) | (b2 >> 6)) & 0x3F;
    let enc4 = b2 & 0x3F;
    encoded.push((enc1 + 48) as char);
    encoded.push((enc2 + 48) as char);
    if chunk.len() > 1 {
      encoded.push((enc3 + 48) as char);
    }
    if chunk.len() > 2 {
      encoded.push((enc4 + 48) as char);
    }
  }
  encoded
}
/// Decoding base64 variants.
///
/// # Examples
///
/// ```rust
/// use jsompiler::utility::de64;
/// assert_eq!(de64("<0").unwrap(), b"0")
/// ```
///
/// # Errors
///
/// `Box<dyn Error(String)>` - If an invalid encoded value is passed, return `Err`
#[expect(dead_code, reason = "todo")]
pub fn de64(encoded: &str) -> Result<Vec<u8>, Box<dyn Error>> {
  let mut decoded = Vec::new();
  let mut buffer = 0u32;
  let mut buffer_length = 0;
  for ch in encoded.chars() {
    let val = (ch as u8).wrapping_sub(48);
    if val > 63 {
      return Err::<Vec<u8>, Box<dyn Error>>("Invalid character in input string".into());
    }
    buffer = (buffer << 6) | u32::from(val);
    buffer_length += 6;
    while buffer_length >= 8 {
      let byte = u8::try_from(buffer >> (buffer_length - 8))?;
      decoded.push(byte);
      buffer_length -= 8;
    }
  }
  Ok(decoded)
}
/// Escapes special characters in a string for proper JSON formatting.
///
/// This method ensures that characters like quotes (`"`) and backslashes (`\`) are escaped
/// in a way that conforms to the JSON specification. It also escapes control characters and
/// non-ASCII characters using Unicode escapes.
///
/// # Arguments
///
/// * `s` - The string to be escaped.
///
/// # Errors
///
/// * `fmt::Error` - ...
/// # Returns
///
/// * `String` - The escaped string.
pub fn escape_string(unescaped: &str) -> Result<String, fmt::Error> {
  let mut escaped = String::new();
  for ch in unescaped.chars() {
    match ch {
      '"' => write!(escaped, "\\\"")?,
      '\\' => write!(escaped, r"\\")?,
      '\n' => write!(escaped, r"\n")?,
      '\t' => write!(escaped, r"\t")?,
      '\r' => write!(escaped, r"\r")?,
      '\u{08}' => write!(escaped, r"\b")?,
      '\u{0C}' => write!(escaped, r"\f")?,
      u_ch if u_ch < '\u{20}' => write!(escaped, "\\u{:04x}", u_ch as u32)?,
      _ => escaped.push(ch),
    }
  }
  Ok(escaped)
}
#[must_use]
pub const fn obj_json(val: JValue, obj: &Json) -> Json {
  Json { pos: obj.pos, line: obj.line, value: val }
}
