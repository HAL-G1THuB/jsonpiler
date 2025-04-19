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
/// Runs the Jsonpiler, compiling and executing a JSON-based program.
/// This is the main function of the Jsonpiler.
/// It runs the full compilation process, step by step:
/// 1. **Argument Parsing:** first command-line argument is the path to the input JSON file.
/// 2. **File Reading:** It reads the content of the specified JSON file into a string.
/// 3. **Parsing:** Converts the JSON text into an internal `Json` data structure.
/// 4. **Compilation:** It compiles the parsed `Json` into assembly code.
/// 5. **Assembly:** It assembles the generated `.asm` code into an `.obj` file.
/// 6. **Linking:** It links the `.obj` file with necessary libraries to create an `.exe` file.
/// 7. **Execution:** It executes the generated `.exe` file.
/// 8. **Exit Code Handling:** It exits with the exit code of the executed program.
/// # Panics
/// This function uses external commands (`as` and `ld`) for assembly and linking.
/// Ensure that these commands are available in the system's PATH.
/// This function will panic if:
/// *   The program is not run on Windows.
/// *   The number of command-line arguments is not exactly two.
/// *   The input file cannot be read.
/// *   The JSON input cannot be parsed.
/// *   The compilation process fails.
/// *   The assembly process fails.
/// *   The linking process fails.
/// *   The generated executable cannot be spawned.
/// *   The program fails to wait for the child process.
/// *   The program fails to retrieve the exit code.
/// *   The current directory cannot be retrieved.
/// *   The filename is invalid.
/// # Errors
/// This function does not return a `Result` type,
/// but instead uses `error_exit` to terminate the program with an error message.
/// # Examples
/// ```sh
/// # Assuming you have a JSON file named "test.json"
/// ./jsonpiler test.json
/// ```
/// # Platform Specific
/// This function is designed to work exclusively on Windows operating systems.
/// # Exits
/// This function will exit the program with the exit code of the executed program.
/// If any error occurs during the process, it will exit with code 1.
#[inline]
pub fn run() -> ! {
  #[cfg(not(target_os = "windows"))]
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
