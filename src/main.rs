use jsonpiler::Jsonpiler;
use std::{
  env, fs,
  path::Path,
  process::{Command, ExitCode},
};
fn main() -> ExitCode {
  macro_rules! exit {($($arg:tt)*) =>{{eprintln!($($arg)*);return ExitCode::FAILURE;}}}
  macro_rules! invoke {
    ($cmd:literal, $list:expr, $name:literal) => {
      match Command::new($cmd).args($list).status() {
        Ok(status) if status.success() => (),
        Ok(_) => exit!("{} returned a non-zero exit status.", $name),
        Err(err) => exit!("Failed to invoke {}: {err}", $name),
      };
    };
  }
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
  let with_ext = |ext: &str| -> String { file.with_extension(ext).to_string_lossy().to_string() };
  let asm = with_ext("s");
  let obj = with_ext("obj");
  let exe = with_ext("exe");
  let Ok(mut jsonpiler) = Jsonpiler::setup(source, &asm) else {
    exit!("Can't create File `{asm}`");
  };
  if let Err(err) = jsonpiler.run() {
    exit!("Compilation error: {err}");
  }
  invoke!("as", &[&asm, "-o", &obj], "assembler");
  #[cfg(not(debug_assertions))]
  if let Err(err) = fs::remove_file(&asm) {
    exit!("Failed to delete `{asm}`: {err}")
  }
  invoke!(
    "ld",
    [&obj, "-o", &exe, "-LC:/Windows/System32", "-luser32", "-lkernel32", "-emain"],
    "linker"
  );
  if let Err(err) = fs::remove_file(&obj) {
    exit!("Failed to delete `{obj}`: {err}")
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
