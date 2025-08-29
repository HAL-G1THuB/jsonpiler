use jsonpiler::Jsonpiler;
use std::{
  env, fs, is_x86_feature_detected,
  path::Path,
  process::{Command, exit},
};
fn main() {
  macro_rules! exit {($($arg:tt)*) =>{{eprintln!($($arg)*);exit(1i32)}}}
  macro_rules! unwrap_exit {
  ($result:expr, $($arg:tt)*) => {
    match $result {
      Ok(value) => value,
      Err(err) => exit!("{err}: {}", format!($($arg)*)),
    }
  };
  }
  if !is_x86_feature_detected!("sse2") {
    exit!("Error: SSE2 not supported on this CPU. CPU may not be x86_64.");
  }
  #[cfg(all(not(doc), not(all(target_os = "windows", target_arch = "x86_64"))))]
  compile_error!("This program is supported on Windows x64 only.");
  let mut args = env::args();
  let Some(program_name) = args.next() else { exit!("Failed to get the program name.") };
  let Some(input_file) = args.next() else {
    exit!("Usage: {program_name} <input.jspl | input.json> [args for .exe]")
  };
  let metadata = unwrap_exit!(fs::metadata(&input_file), "Failed to access for `{input_file}`");
  if metadata.len() > 1 << 30u8 {
    exit!("Input file size exceeds 1GB. Please provide a smaller file.");
  }
  let source = unwrap_exit!(fs::read(&input_file), "Failed to read `{input_file}`");
  let file = Path::new(&input_file);
  let is_jspl = match file.extension() {
    Some(ext) if ext == "jspl" => true,
    Some(ext) if ext == "json" => false,
    _ => exit!("Input file must be a .json or .jspl file."),
  };
  let exe = file.with_extension("exe").to_string_lossy().to_string();
  let mut jsonpiler = Jsonpiler::setup(source, input_file);
  if let Err(err) = jsonpiler.run(&exe, is_jspl) {
    exit!("Compilation error: {err}");
  }
  let cwd = unwrap_exit!(env::current_dir(), "Failed to get current directory").join(&exe);
  let exe_status =
    unwrap_exit!(Command::new(cwd).args(args).status(), "Failed to execute compiled program");
  let Some(exit_code) = exe_status.code() else {
    exit!("Could not get the exit code of the compiled program.")
  };
  exit(exit_code);
}
