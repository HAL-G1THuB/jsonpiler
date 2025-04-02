use jsompiler::{Jsompiler, utility::error_exit};
use std::{env, fs, path::Path, process::Command};
fn main() -> ! {
  #[cfg(not(target_os = "windows"))]
  compile_error!("This program can only run on Windows.");
  let args: Vec<String> = env::args().collect();
  if args.len() != 2 {
    eprintln!("Usage: {} <input json file>", args[0]);
    std::process::exit(0)
  }
  let input_code = fs::read_to_string(&args[1])
    .unwrap_or_else(|e| error_exit(&format!("Failed to read file: {e}")));
  let mut jsompiler = Jsompiler::default();
  let parsed =
    jsompiler.parse(&input_code).unwrap_or_else(|e| error_exit(&format!("ParseError: {e}")));
  #[cfg(debug_assertions)]
  println!("{}", parsed);
  let file = Path::new(&args[1])
    .file_stem()
    .unwrap_or_else(|| error_exit(&format!("Invalid filename: {}", args[1])))
    .to_string_lossy()
    .to_string();
  let obj_file = format!("{file}.obj");
  let exe_file = format!("{file}.exe");
  let asm_file = format!("{file}.s");
  jsompiler
    .build(parsed, &args[1], &asm_file)
    .unwrap_or_else(|e| error_exit(&format!("CompileError: {e}")));
  if !Command::new("as")
    .args([&asm_file, "-o", &obj_file])
    .status()
    .unwrap_or_else(|e| error_exit(&format!("Failed to assemble: {e}")))
    .success()
  {
    error_exit("Failed to assemble")
  };
  if !Command::new("ld")
    .args([
      &obj_file,
      "-o",
      &exe_file,
      "-LC:/Windows/System32",
      "-luser32",
      "-lkernel32",
      "-lucrtbase",
    ])
    .status()
    .unwrap_or_else(|e| error_exit(&format!("Failed to link: {e}")))
    .success()
  {
    error_exit("Failed to link")
  };
  let mut path = env::current_dir()
    .unwrap_or_else(|e| error_exit(&format!("Failed to get current directory: {e}")));
  path.push(&exe_file);
  let exit_code = Command::new(path)
    .spawn()
    .unwrap_or_else(|e| error_exit(&format!("Failed to spawn child process: {e}")))
    .wait()
    .unwrap_or_else(|e| error_exit(&format!("Failed to wait for child process: {e}")))
    .code()
    .unwrap_or_else(|| error_exit("Failed to retrieve the exit code"));
  std::process::exit(exit_code)
}
