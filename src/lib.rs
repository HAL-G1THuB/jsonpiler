//! (main.rs)
//! ```rust
//! fn main() -> ! {
//!  jsompiler::run()
//!}
//! ```
mod impl_compiler;
mod impl_jvalue;
mod impl_parser;
mod utility;
use core::error::Error;
use std::{
  collections::HashMap,
  env, fs,
  path::Path,
  process::{Command, exit},
};
use utility::error_exit;
type JFunc<T> = fn(&mut T, &[Json], &mut String) -> Result<JValue, Box<dyn Error>>;
type JFuncResult = Result<JValue, Box<dyn Error>>;
type JResult = Result<Json, Box<dyn Error>>;
// Type and value information.
#[derive(Debug, Clone, Default)]
pub enum JValue {
  Array(Vec<Json>),
  ArrayVar(String),
  Bool(bool),
  BoolVar(String, usize),
  Float(f64),
  FloatVar(String),
  FuncVar {
    name: String,
    params: Vec<Json>,
    ret: Box<JValue>,
  },
  Int(i64),
  IntVar(String),
  #[default]
  Null,
  Object(HashMap<String, Json>),
  ObjectVar(String),
  String(String),
  StringVar(String),
}
#[derive(Debug, Clone, Default)]
pub(crate) struct ParserContext {
  /// Location of the part being parsed.
  pos: usize,
  /// Line number of the part being parsed.
  line: usize,
}
/// Section of the assembly.
#[derive(Debug, Clone, Default)]
pub(crate) struct Section {
  /// Buffer to store the contents of the data section of the assembly.
  data: String,
  /// Buffer to store the contents of the bss section of the assembly.
  bss: String,
  /// Buffer to store the contents of the text section of the assembly.
  text: String,
}
#[derive(Debug, Clone, Default)]
pub struct Jsompiler<'a> {
  input_code: &'a str,
  pctx: ParserContext,
  seed: usize,
  /// Buffer to store the contents of the data section of the assembly.
  sect: Section,
  f_table: HashMap<String, JFunc<Self>>,
  _globals: HashMap<String, JValue>,
  vars: HashMap<String, JValue>,
}
/// Json object.
#[derive(Debug, Clone, Default)]
pub struct Json {
  /// Line number of objects in the source code.
  pub line: usize,
  /// Location of objects in the source code.
  pub pos: usize,
  /// Type and value information.
  pub value: JValue,
}
/// Runs the Jsompiler, compiling and executing a JSON-based program.
///
/// This function serves as the main entry point for the Jsompiler. It performs the following steps:
///
/// 1.  **Argument Parsing:** It expects a single command-line argument, which is the path to the input JSON file.
/// 2.  **File Reading:** It reads the content of the specified JSON file into a string.
/// 3.  **Parsing:** It parses the JSON string into an internal `Json` representation using the `Jsompiler`.
/// 4.  **Compilation:** It compiles the parsed `Json` into assembly code.
/// 5.  **Assembly:** It assembles the generated assembly code into an object file (`.obj`).
/// 6.  **Linking:** It links the object file with necessary libraries to create an executable file (`.exe`).
/// 7.  **Execution:** It executes the generated `.exe` file.
/// 8.  **Exit Code Handling:** It retrieves the exit code from the executed program and exits with the same code.
///
/// # Panics
///
/// This function uses external commands (`as` and `ld`) for assembly and linking. Ensure that these commands are available in the system's PATH.
///
/// This function will panic if:
///
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
///
/// # Errors
///
/// This function does not return a `Result` type, but instead uses `error_exit` to terminate the program with an error message.
///
///
/// # Examples
///
/// ```bash
/// # Assuming you have a JSON file named "test.json"
/// ./jsompiler test.json
/// ```
///
/// # Platform Specific
///
/// This function is designed to work exclusively on Windows operating systems.
///
/// # Exits
///
/// This function will exit the program with the exit code of the executed program.
/// If any error occurs during the process, it will exit with code 1.
pub fn run() -> ! {
  #[cfg(not(target_os = "windows"))]
  compile_error!("This program can only run on Windows.");
  let args: Vec<String> = env::args().collect();
  if args.len() <= 1 {
    eprintln!("Usage: {} <input json file> [arguments of .exe...]", args[0]);
    exit(0)
  }
  let input_code = fs::read_to_string(&args[1])
    .unwrap_or_else(|err| error_exit(&format!("Failed to read file {}: {err}", args[1])));
  let mut jsompiler = Jsompiler::default();
  let parsed =
    jsompiler.parse(&input_code).unwrap_or_else(|e| error_exit(&format!("ParseError: {e}")));
  #[cfg(debug_assertions)]
  println!("{}", parsed.value);
  let file = Path::new(&args[1]).with_extension("").to_string_lossy().to_string();
  let obj_file = format!("{file}.obj");
  let exe_file = format!("{file}.exe");
  let asm_file = format!("{file}.s");
  jsompiler
    .build(&parsed, &args[1], &asm_file)
    .unwrap_or_else(|err| error_exit(&format!("CompileError: {err}")));
  if !Command::new("as")
    .args([&asm_file, "-o", &obj_file])
    .status()
    .unwrap_or_else(|err| error_exit(&format!("Failed to assemble: {err}")))
    .success()
  {
    error_exit("Failed to assemble")
  }
  #[cfg(not(debug_assertions))]
  fs::remove_file(asm_file)
    .unwrap_or_else(|err| error_exit(&format!("Failed to remove '.asm': {err}")));
  if !Command::new("ld")
    .args([
      &obj_file,
      "-o",
      &exe_file,
      "-LC:/Windows/System32",
      "-luser32",
      "-lkernel32",
      "-lucrtbase",
      "--gc-sections",
      "-e_start",
    ])
    .status()
    .unwrap_or_else(|err| error_exit(&format!("Failed to link: {err}")))
    .success()
  {
    error_exit("Failed to link")
  }
  #[cfg(not(debug_assertions))]
  fs::remove_file(obj_file)
    .unwrap_or_else(|err| error_exit(&format!("Failed to remove '.obj': {err}")));
  let mut path = env::current_dir()
    .unwrap_or_else(|err| error_exit(&format!("Failed to get current directory: {err}")));
  path.push(&exe_file);
  let exit_code = Command::new(path)
    .args(&args[2..])
    .spawn()
    .unwrap_or_else(|err| error_exit(&format!("Failed to spawn child process: {err}")))
    .wait()
    .unwrap_or_else(|err| error_exit(&format!("Failed to wait for child process: {err}")))
    .code()
    .unwrap_or_else(|| error_exit("Failed to retrieve the exit code"));
  exit(exit_code)
}
