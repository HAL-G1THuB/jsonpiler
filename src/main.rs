use jsompiler::definition::JParser;
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
fn error_exit(text: String) -> ! {
  let mut nu = String::new();
  eprint!("{text}\nPress Enter to exit:");
  let _ = io::stdin().read_line(&mut nu);
  std::process::exit(1)
}
fn main() -> ! {
  let args: Vec<String> = env::args().collect();
  if args.len() != 2 {
    eprintln!("Usage: {} <input json file>", args[0]);
    std::process::exit(0)
  }
  let input_code = fs::read_to_string(&args[1])
    .unwrap_or_else(|e| error_exit(format!("Failed to read file: {e}")));
  let mut parser = JParser::default();
  let parsed = parser
    .parse(&input_code)
    .unwrap_or_else(|e| error_exit(format!("ParseError: {e}")));
  #[cfg(debug_assertions)]
  {
    parsed
      .print_json()
      .unwrap_or_else(|e| error_exit(format!("Couldn't print json: {e}")));
  }
  let json_file = Path::new(&args[1])
    .file_stem()
    .unwrap_or_else(|| error_exit(format!("Invalid filename: {}", args[1])))
    .to_string_lossy();
  let asm_file = format!("{json_file}.s");
  let exe_file = format!("{json_file}.exe");
  parser
    .build(parsed, &asm_file)
    .unwrap_or_else(|e| error_exit(format!("CompileError: {e}")));
  if !Command::new("gcc")
    .args([
      &asm_file,
      "-o",
      &exe_file,
      "-nostartfiles",
      "-luser32",
      "-lkernel32",
    ])
    .status()
    .unwrap_or_else(|e| error_exit(format!("Failed to assemble or link: {e}")))
    .success()
  {
    error_exit(String::from("Failed to assemble or link"))
  };
  let mut path = env::current_dir()
    .unwrap_or_else(|e| error_exit(format!("Failed to get current directory: {e}")));
  path.push(&exe_file);
  let exit_code = Command::new(path)
    .spawn()
    .unwrap_or_else(|e| error_exit(format!("Failed to spawn child process: {e}")))
    .wait()
    .unwrap_or_else(|e| error_exit(format!("Failed to wait for child process: {e}")))
    .code()
    .unwrap_or_else(|| error_exit(String::from("Failed to retrieve the exit code")));
  std::process::exit(exit_code)
}
