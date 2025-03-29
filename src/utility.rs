use std::io;
use crate::definition::JResult;
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
