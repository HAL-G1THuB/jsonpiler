//! Utility functions.
use {
  crate::{ErrOR, ErrorInfo, JValue, Json, Jsonpiler},
  core::fmt::{self, Write as _},
  std::{
    env, fs,
    path::Path,
    process::{Command, ExitCode},
  },
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
      buffer_len = buffer_len.checked_sub(8).ok_or(ERR)?;
      decoded.push(u8::try_from(buffer >> buffer_len)?);
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
/// Escapes special characters in a string for proper JSON formatting.
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
pub(crate) const fn gen_json(val: JValue, e_info: ErrorInfo) -> Json {
  Json { info: e_info, value: val }
}
/// Compiles and executes a JSON-based program using the Jsonpiler.
///
/// This function performs the following steps:
/// 1. Parses the first CLI argument as the input JSON file path.
/// 2. Reads the file content into a string.
/// 3. Parses the string into a `Json` structure.
/// 4. Compiles the structure into assembly code.
/// 5. Assembles it into an `.obj` file.
/// 6. Links it into an `.exe`.
/// 7. Executes the resulting binary.
/// 8. Returns its exit code.
///
/// # Panics
/// This function will panic if:
/// - The platform is not Windows.
/// - CLI arguments are invalid.
/// - File reading, parsing, compilation, assembling, linking, or execution fails.
/// - The working directory or executable filename is invalid.
///
/// # Requirements
/// - `as` and `ld` must be available in the system PATH.
/// - On failure, exits with code 1 using `error_exit`.
///
/// # Example
/// ```sh
/// ./jsonpiler test.json
/// ```
///
/// # Platform
/// Windows only.
#[inline]
#[must_use]
#[expect(clippy::print_stderr, reason = "User-facing diagnostics")]
pub fn run() -> ExitCode {
  #[cfg(all(not(doc), not(target_os = "windows")))]
  compile_error!("This program is supported on Windows only.");
  let args: Vec<String> = env::args().collect();
  let Some(program_name) = args.first() else {
    eprintln!("Failed to get the program name.");
    return ExitCode::FAILURE;
  };
  let Some(input_file) = args.get(1) else {
    eprintln!("Usage: {program_name} <input_json_file> [args for .exe]");
    return ExitCode::FAILURE;
  };
  let source = match fs::read_to_string(input_file) {
    Ok(content) => content,
    Err(err) => {
      eprintln!("Failed to read '{input_file}': {err}");
      return ExitCode::FAILURE;
    }
  };
  let mut jsonpiler = Jsonpiler::default();
  let file = Path::new(input_file);
  let asm = file.with_extension("s").to_string_lossy().to_string();
  let obj = file.with_extension("obj").to_string_lossy().to_string();
  let exe = file.with_extension("exe").to_string_lossy().to_string();
  if let Err(err) = jsonpiler.build(source, input_file, &asm) {
    eprintln!("Compilation error: {err}");
    return ExitCode::FAILURE;
  }
  match Command::new("as").args([&asm, "-o", &obj]).status() {
    Ok(status) if status.success() => status,
    Ok(_) => {
      eprintln!("Assembler returned a non-zero exit status.");
      return ExitCode::FAILURE;
    }
    Err(err) => {
      eprintln!("Failed to invoke assembler: {err}");
      return ExitCode::FAILURE;
    }
  };
  #[cfg(not(debug_assertions))]
  if let Err(err) = fs::remove_file(&asm) {
    eprintln!("Failed to delete '{asm}': {err}");
    return ExitCode::FAILURE;
  }
  match Command::new("ld")
    .args([
      &obj,
      "-o",
      &exe,
      "-LC:/Windows/System32",
      "-luser32",
      "-lkernel32",
      "-lucrtbase",
      "--gc-sections",
      "-e_start",
    ])
    .status()
  {
    Ok(status) if status.success() => status,
    Ok(_) => {
      eprintln!("Linker returned a non-zero exit status.");
      return ExitCode::FAILURE;
    }
    Err(err) => {
      eprintln!("Failed to invoke linker: {err}");
      return ExitCode::FAILURE;
    }
  };
  if let Err(err) = fs::remove_file(&obj) {
    eprintln!("Failed to delete '{obj}': {err}");
    return ExitCode::FAILURE;
  }
  let cwd = match env::current_dir() {
    Ok(dir) => dir,
    Err(err) => {
      eprintln!("Failed to get current directory: {err}");
      return ExitCode::FAILURE;
    }
  };
  let exe_status = match Command::new(cwd.join(&exe)).args(args.get(2..).unwrap_or(&[])).status() {
    Ok(status) => status,
    Err(err) => {
      eprintln!("Failed to execute compiled program: {err}");
      return ExitCode::FAILURE;
    }
  };
  let Some(exit_code) = exe_status.code() else {
    eprintln!("Could not retrieve the child process's exit code.");
    return ExitCode::FAILURE;
  };
  if let Ok(code) = u8::try_from(exit_code.rem_euclid(256)) {
    ExitCode::from(code)
  } else {
    eprintln!("Internal error: Unexpected failure in exit code conversion.");
    ExitCode::FAILURE
  }
}
