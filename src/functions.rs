//! Utility functions.
use crate::{ErrOR, ErrorInfo, JValue, Json, Jsonpiler};
use core::fmt::{self, Display, Write as _};
use std::{
  env, fs,
  path::Path,
  process::{Command, exit},
};
/// Decoding base64 variants.
/// # Errors
/// `Box<dyn Error(String)>` - If an invalid encoded value is passed, return `Err`
#[expect(dead_code, reason = "todo")]
pub(crate) fn de64(encoded: &str) -> ErrOR<Vec<u8>> {
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
  let mut helper = |enc: u8| {
    encoded.push(char::from_u32(u32::from(enc).checked_add(48).ok_or(ERR)?).ok_or(ERR)?);
    Ok(())
  };
  let chunks = input.chunks(3);
  for chunk in chunks {
    let b0 = chunk.first().unwrap_or(&0u8);
    let b1 = chunk.get(1).unwrap_or(&0u8);
    helper((b0 >> 2u8) & 0x3F)?;
    helper(((b0 << 4u8) | (b1 >> 4u8)) & 0x3F)?;
    if chunk.len() >= 2 {
      let b2 = chunk.get(2).unwrap_or(&0u8);
      helper(((b1 << 2u8) | (b2 >> 6u8)) & 0x3F)?;
      if chunk.len() == 3 {
        helper(b2 & 0x3F)?;
      }
    }
  }
  Ok(encoded)
}
/// Exit the program with exit code 1.
#[expect(clippy::print_stderr, reason = "")]
pub(crate) fn error_exit(text: &str) -> ! {
  eprintln!("{text}");
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
/// Change the value of another Json to create a new Json.
#[must_use]
pub(crate) const fn obj_json(val: JValue, e_info: ErrorInfo) -> Json {
  Json { info: e_info, value: val }
}
/// Compiles and runs a JSON-based program using the Jsonpiler.
/// This function performs the following steps:
/// 1. Parses the first CLI argument as the input JSON file path.
/// 2. Reads the file content into a string.
/// 3. Parses it into a `Json` structure.
/// 4. Compiles it into assembly.
/// 5. Assembles to `.obj`.
/// 6. Links to `.exe`.
/// 7. Executes the `.exe`.
/// 8. Exits with its exit code.
/// # Panics
/// Panics if:
/// - Not on Windows
/// - Incorrect CLI arguments
/// - File read, parse, compile, assemble, link, execute, or wait fails
/// - Invalid filename or working directory
/// # Notes
/// Requires `as` and `ld` in PATH.
/// Terminates with `error_exit` on failure (exit code 1).
/// # Example
/// ```sh
/// ./jsonpiler test.json
/// ```
/// # Platform
/// Windows only.
#[inline]
pub fn run() -> ! {
  #[cfg(all(not(doc), not(target_os = "windows")))]
  compile_error!("This program can only run on Windows.");
  let args: Vec<String> = env::args().collect();
  let Some(program_name) = args.first() else { error_exit("Failed to get name of the program") };
  let input_file = unwrap_or_exit(
    args.get(1).ok_or_else(|| format!("{program_name} <input json file> [arguments of .exe...]")),
    "Usage",
  );
  let source =
    unwrap_or_exit(fs::read_to_string(input_file), &format!("Failed to read file '{input_file}'"));
  let mut jsonpiler = Jsonpiler::default();
  let file = Path::new(input_file);
  let asm = &file.with_extension("s").to_string_lossy().to_string();
  let obj = &file.with_extension("obj").to_string_lossy().to_string();
  let exe = &file.with_extension("exe").to_string_lossy().to_string();
  unwrap_or_exit(jsonpiler.build(source, input_file, asm), "Error");
  (!Command::new("as")
    .args([asm, "-o", obj])
    .status()
    .unwrap_or_else(|err| error_exit(&format!("Failed to assemble: {err}")))
    .success())
  .then(|| error_exit("Assembling process returned Bad status."));
  #[cfg(not(debug_assertions))]
  {
    unwrap_or_exit(fs::remove_file(asm), &format!("Failed to remove '{asm}'"))
  }
  (!Command::new("ld")
    .args([
      obj,
      "-o",
      exe,
      "-LC:/Windows/System32",
      "-luser32",
      "-lkernel32",
      "-lucrtbase",
      "--gc-sections",
      "-e_start",
    ])
    .status()
    .unwrap_or_else(|err| error_exit(&format!("Failed to link: {err}")))
    .success())
  .then(|| error_exit("Linking process returned Bad status."));
  unwrap_or_exit(fs::remove_file(obj), &format!("Failed to remove '{obj}'"));
  let exit_code =
    Command::new(unwrap_or_exit(env::current_dir(), "Failed to get current directory").join(exe))
      .args(args.get(2..).unwrap_or(&[]))
      .spawn()
      .unwrap_or_else(|err| error_exit(&format!("Failed to spawn child process: {err}")))
      .wait()
      .unwrap_or_else(|err| error_exit(&format!("Failed to wait for child process: {err}")))
      .code()
      .unwrap_or_else(|| error_exit("Failed to retrieve the exit code."));
  exit(exit_code)
}
/// Unwraps the result. Exits the program on error.
pub(crate) fn unwrap_or_exit<T, U: Display>(result: Result<T, U>, text: &str) -> T {
  result.unwrap_or_else(|err| error_exit(&format!("{text}: {err}")))
}
