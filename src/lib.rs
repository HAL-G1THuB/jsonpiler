//! (main.rs)
//! ```should_panic
//! fn main() -> ! {
//!  jsonpiler::run()
//!}
//! ```
mod impl_compiler;
mod impl_object;
mod impl_parser;
mod impl_value;
mod utility;
use core::error::Error;
use std::{
  collections::HashMap,
  env, fs,
  path::Path,
  process::{Command, exit},
};
use utility::error_exit;
/// Built-in function types.
#[derive(Debug, Clone)]
/// Built-in function.
pub(crate) struct BuiltinFunc<T> {
  /// Should arguments already be evaluated.
  pub evaluated: bool,
  /// Pointer of function.
  pub func: JFunc<T>,
}
type JFunc<T> = fn(&mut T, &Json, &[Json], &mut String) -> JFuncResult;
/// Contain `JValue` or `Box<dyn Error>`.
type JFuncResult = Result<JValue, Box<dyn Error>>;
/// Represents a JSON object with key-value pairs.
#[derive(Debug, Clone, Default)]
pub(crate) struct JObject {
  /// Stores the key-value pairs in insertion order.
  entries: Vec<(String, JValue)>,
  /// Maps keys to their index in the entries vector for quick lookup.
  idx: HashMap<String, usize>,
}
/// Contain `Json` or `Box<dyn Error>`.
type JResult = Result<Json, Box<dyn Error>>;
/// Type and value information.
#[derive(Debug, Clone, Default)]
pub(crate) enum JValue {
  /// Array.
  Array(Vec<Json>),
  /// Array variable.
  #[expect(dead_code, reason = "todo")]
  ArrayVar(String),
  /// Bool.
  Bool(bool),
  /// Bool variable.
  #[expect(dead_code, reason = "todo")]
  BoolVar(String, usize),
  /// Float.
  Float(f64),
  /// Float variable.
  #[expect(dead_code, reason = "todo")]
  FloatVar(String),
  /// Function.
  FuncVar {
    /// Name of function.
    name: String,
    /// Parameters of function.
    params: Vec<Json>,
    /// Return type of function.
    ret: Box<JValue>,
  },
  /// Integer.
  Int(i64),
  /// Integer variable.
  IntVar(String),
  /// Null.
  #[default]
  Null,
  /// Object.
  Object(HashMap<String, Json>),
  /// Object variable.
  #[expect(dead_code, reason = "todo")]
  ObjectVar(String),
  /// String.
  String(String),
  /// String variable.
  StringVar(String),
}
/// Json object.
#[derive(Debug, Clone, Default)]
pub(crate) struct Json {
  /// Line number of objects in the source code.
  line: usize,
  /// Location of objects in the source code.
  pos: usize,
  /// Type and value information.
  value: JValue,
}
/// Parser and compiler.
#[derive(Debug, Clone, Default)]
pub struct Jsonpiler {
  /// Global variables (now on Unused).
  _globals: HashMap<String, JValue>,
  /// Built-in function table.
  f_table: HashMap<String, BuiltinFunc<Self>>,
  /// Information to be used during parsing.
  info: ParseInfo,
  /// Section of the assembly.
  sect: Section,
  /// Seed to generate label names.
  seed: usize,
  /// Source code.
  source: String,
  /// Variable table.
  vars: HashMap<String, JValue>,
}
/// Information to be used during parsing.
#[derive(Debug, Clone, Default)]
pub(crate) struct ParseInfo {
  /// Line number of the part being parsed.
  line: usize,
  /// Location of the part being parsed.
  pos: usize,
}
/// Section of the assembly.
#[derive(Debug, Clone, Default)]
pub(crate) struct Section {
  /// Buffer to store the contents of the bss section of the assembly.
  bss: String,
  /// Buffer to store the contents of the data section of the assembly.
  data: String,
  /// Buffer to store the contents of the text section of the assembly.
  text: String,
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
  let Some(input_file) = args.get(1) else {
    error_exit(&format!("Usage: {program_name} <input json file> [arguments of .exe...]"))
  };
  let source = fs::read_to_string(input_file)
    .unwrap_or_else(|err| error_exit(&format!("Failed to read file ({input_file}): {err}")));
  let mut jsonpiler = Jsonpiler::default();
  let file = Path::new(input_file);
  let with_s = file.with_extension("s");
  let asm = &with_s.to_string_lossy().to_string();
  let with_obj = file.with_extension("obj");
  let obj = &with_obj.to_string_lossy().to_string();
  let with_exe = file.with_extension("exe");
  let exe = &with_exe.to_string_lossy().to_string();
  jsonpiler
    .build(source, input_file, asm)
    .unwrap_or_else(|err| error_exit(&format!("Error: {err}")));
  (!Command::new("as")
    .args([asm, "-o", obj])
    .status()
    .unwrap_or_else(|err| error_exit(&format!("Failed to assemble: {err}")))
    .success())
  .then(|| error_exit("Assembling process returned Bad status."));
  #[cfg(not(debug_assertions))]
  fs::remove_file(asm)
    .unwrap_or_else(|err| error_exit(&format!("Failed to remove '{asm}': {err}")));
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
  #[cfg(not(debug_assertions))]
  fs::remove_file(obj)
    .unwrap_or_else(|err| error_exit(&format!("Failed to remove '{obj}': {err}")));
  let mut path = env::current_dir()
    .unwrap_or_else(|err| error_exit(&format!("Failed to get current directory: {err}")));
  path.push(exe);
  let exit_code = Command::new(path)
    .args(args.get(2..).unwrap_or(&[]))
    .spawn()
    .unwrap_or_else(|err| error_exit(&format!("Failed to spawn child process: {err}")))
    .wait()
    .unwrap_or_else(|err| error_exit(&format!("Failed to wait for child process: {err}")))
    .code()
    .unwrap_or_else(|| error_exit("Failed to retrieve the exit code."));
  exit(exit_code)
}
