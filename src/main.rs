use jsonpiler::Jsonpiler;
use std::{
  env, fs,
  path::Path,
  process::{Command, ExitCode},
};
fn main() -> ExitCode {
  macro_rules! exit {($($arg:tt)*) =>{{eprintln!($($arg)*);return ExitCode::FAILURE;}}}
  #[cfg(all(not(doc), not(target_os = "windows")))]
  compile_error!("This program is supported on Windows only.");
  let args: Vec<String> = env::args().collect();
  let Some(program_name) = args.first() else { exit!("Failed to get the program name.") };
  let Some(input_file) = args.get(1) else {
    exit!("Usage: {program_name} <input_json_file> [args for .exe]")
  };
  let metadata = match fs::metadata(input_file) {
    Ok(metadata) => metadata,
    Err(err) => exit!("Failed to get the file size of `{input_file}`: {err}"),
  };
  if metadata.len() > 1 << 30u8 {
    exit!("Input file size exceeds 1GB. Please provide a smaller file.");
  }
  let source = match fs::read(input_file) {
    Ok(content) => content,
    Err(err) => exit!("Failed to read `{input_file}`: {err}"),
  };
  let file = Path::new(input_file);
  let is_jspl = match file.extension() {
    Some(ext) if ext == "jspl" => true,
    Some(ext) if ext == "json" => false,
    _ => exit!("Input file must be a .json or .jspl file."),
  };
  let exe = file.with_extension("exe").to_string_lossy().to_string();
  let mut jsonpiler = Jsonpiler::setup(source);
  if let Err(err) = jsonpiler.run(&exe, is_jspl) {
    exit!("Compilation error: {err}");
  }
  let cwd = match env::current_dir() {
    Ok(dir) => dir,
    Err(err) => exit!("Failed to get current directory: {err}"),
  }
  .join(&exe);
  let exe_status = match Command::new(cwd).args(&args[2..]).status() {
    Ok(status) => status,
    Err(err) => exit!("Failed to execute compiled program: {err}"),
  };
  let Some(exit_code) = exe_status.code() else {
    exit!("Could not get the exit code of the compiled program.")
  };
  let Ok(code) = u8::try_from(exit_code.rem_euclid(256)) else {
    exit!("Internal error: Unexpected error in exit code conversion.")
  };
  ExitCode::from(code)
}
