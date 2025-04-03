//! (main.rs)
//! ```rust
//! fn main() -> ! {
//!  jsompiler::run()
//!}
//! ```
mod impl_compiler;
mod impl_json;
mod impl_parser;
pub mod utility;
use utility::{error_exit, format_err};
pub type JResult = Result<Json, Box<dyn Error>>;
pub type JFunc<T> = fn(&mut T, &[Json], &mut String) -> JResult;
use std::{collections::HashMap, env, error::Error, fmt, fs, path::Path, process::Command};
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
/// # Safety
///
/// This function uses external commands (`as` and `ld`) for assembly and linking. Ensure that these commands are available in the system's PATH.
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
  println!("{parsed}");
  let file = Path::new(&args[1])
    .file_stem()
    .unwrap_or_else(|| error_exit(&format!("Invalid filename: {}", args[1])))
    .to_string_lossy()
    .to_string();
  let obj_file = format!("{file}.obj");
  let exe_file = format!("{file}.exe");
  let asm_file = format!("{file}.s");
  jsompiler
    .build(&parsed, &args[1], &asm_file)
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
#[derive(Debug, Clone)]
pub struct Json {
  pub pos: usize,
  pub ln: usize,
  pub value: JValue,
}
#[derive(Debug, Clone)]
pub enum JValue {
  Null,
  Bool(bool),
  Int(i64),
  Float(f64),
  String(String),
  Array(Vec<Json>),
  Object(HashMap<String, Json>),
  FuncVar(String, Vec<Json>),
  BoolVar(String),
  IntVar(String),
  FloatVar(String),
  StringVar(String),
  ArrayVar(String),
  ObjectVar(String),
}
#[derive(Debug, Clone, Default)]
pub struct Jsompiler<'a> {
  input_code: &'a str,
  pos: usize,
  seed: usize,
  ln: usize,
  data: String,
  bss: String,
  text: String,
  f_table: HashMap<String, JFunc<Self>>,
  _globals: HashMap<String, JValue>,
  vars: HashMap<String, JValue>,
}
impl Jsompiler<'_> {
  fn obj_err(&self, text: &str, obj: &Json) -> JResult {
    format_err(text, obj.pos, obj.ln, self.input_code)
  }
  fn parse_err(&self, text: &str) -> JResult {
    format_err(text, self.pos, self.ln, self.input_code)
  }
}
#[derive(Debug)]
pub struct JError(pub String);

impl fmt::Display for JError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl Error for JError {}
