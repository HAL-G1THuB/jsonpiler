use crate::definition::{JResult, JValue, Json};
use std::error::Error;
use std::io;
pub fn format_err(text: &str, index: usize, ln: usize, input_code: &str) -> JResult {
  if input_code.is_empty() {
    return Err("Error: Empty input".into());
  }
  let len = input_code.len();
  let idx = index.min(len.saturating_sub(1));
  let start = if idx > 0 {
    input_code[..idx].rfind('\n').map_or(0, |pos| pos + 1)
  } else {
    0
  };
  let end = input_code[idx..].find('\n').map_or(len, |pos| idx + pos);
  let ws = " ".repeat(idx.saturating_sub(start));
  let result = &input_code[start..end];
  Err(format!("{text}\nError occurred on line: {ln}\nError position:\n{result}\n{ws}^").into())
}

pub fn error_exit(text: &str) -> ! {
  let mut nu = String::new();
  eprint!("{text}\nPress Enter to exit:");
  let _ = io::stdin().read_line(&mut nu);
  std::process::exit(1)
}
pub fn dummy() -> JResult {
  Ok(Json {
    pos: 0,
    ln: 0,
    value: JValue::Null,
  })
}
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
pub fn de64(encoded: &str) -> Result<Vec<u8>, Box<dyn Error>> {
  let mut decoded = Vec::new();
  let mut buffer = 0u32;
  let mut buffer_length = 0;
  for ch in encoded.chars() {
    let val = (ch as u8).wrapping_sub(48);
    if val > 63 {
      panic!("Invalid character in input string");
    }
    buffer = (buffer << 6) | val as u32;
    buffer_length += 6;
    while buffer_length >= 8 {
      let byte = (buffer >> (buffer_length - 8)) as u8;
      decoded.push(byte);
      buffer_length -= 8;
    }
  }
  Ok(decoded)
}
