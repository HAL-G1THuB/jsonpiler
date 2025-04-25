//! (main.rs)
//! ```
//! use jsonpiler::run;
//! use std::process::ExitCode;
//! fn main() -> ExitCode {
//!   run()
//! }
//! ```
mod builtin;
mod compiler;
mod func_info;
mod json;
mod object;
mod parser;
use core::error::Error;
use std::{
  collections::{HashMap, HashSet},
  env, fs,
  path::Path,
  process::{Command, ExitCode},
};
/// Generates an error.
#[macro_export]
macro_rules! err {
  ($self:ident, $pos:expr, $($arg:tt)*) => {
    Err($self.fmt_err(&format!($($arg)*), &$pos).into())
  };
  ($self:ident, $($arg:tt)*) => {
    Err($self.fmt_err(&format!($($arg)*), &$self.pos).into())
  };
}
/// Return `ExitCode`.
macro_rules! exit {($($arg: tt)*) =>{{eprintln!($($arg)*);return ExitCode::FAILURE;}}}
/// Arguments.
type Args = [JsonWithPos];
#[derive(Debug, Clone)]
/// Assembly function representation.
struct AsmFunc {
  /// Name of function.
  name: String,
  /// Parameters of function.
  params: Vec<JsonWithPos>,
  /// Return type of function.
  ret: Box<Json>,
}
/// Built-in function.
#[derive(Debug, Clone)]
struct Builtin {
  /// Pointer of function.
  func: JFunc,
  /// If it is true, function introduces a new scope.
  scoped: bool,
  /// Should arguments already be evaluated.
  skip_eval: bool,
}
type ErrOR<T> = Result<T, Box<dyn Error>>;
/// Contain `JValue` or `Box<dyn Error>`.
type FResult = ErrOR<Json>;
/// Information of Function.
#[derive(Debug, Clone, Default)]
struct FuncInfo {
  /// Size of arguments.
  args_slots: usize,
  /// Body of function.
  body: Vec<String>,
  /// Size of local variable.
  local_size: usize,
  /// Registers used.
  reg_used: HashSet<String>,
}
/// Type of built-in function.
type JFunc = fn(&mut Jsonpiler, &JsonWithPos, &Args, &mut FuncInfo) -> FResult;
/// Represents a JSON object with key-value pairs.
#[derive(Debug, Clone, Default)]
struct JObject {
  /// Stores the key-value pairs in insertion order.
  entries: Vec<(String, JsonWithPos)>,
  /// Maps keys to their index in the entries vector for quick lookup.
  index: HashMap<String, usize>,
}
/// Contain `Json` or `Box<dyn Error>`.
type JResult = ErrOR<JsonWithPos>;
/// Type and value information.
#[derive(Debug, Clone, Default)]
enum Json {
  /// Function.
  Function(AsmFunc),
  /// Array.
  LArray(Vec<JsonWithPos>),
  /// Bool.
  LBool(bool),
  /// Float.
  LFloat(f64),
  /// Integer.
  LInt(i64),
  /// Object.
  LObject(JObject),
  /// String.
  LString(String),
  /// Null.
  #[default]
  Null,
  /// Array variable.
  #[expect(dead_code, reason = "todo")]
  VArray(String),
  /// Bool variable.
  #[expect(dead_code, reason = "todo")]
  VBool(String, usize),
  /// Float variable.
  #[expect(dead_code, reason = "todo")]
  VFloat(String),
  /// Integer variable.
  VInt(String),
  /// Object variable.
  #[expect(dead_code, reason = "todo")]
  VObject(String),
  /// String variable.
  VString(String),
}
/// Json object.
#[derive(Debug, Clone, Default)]
struct JsonWithPos {
  /// Line number of objects in the source code.
  pos: Position,
  /// Type and value information.
  value: Json,
}
/// Parser and compiler.
#[derive(Debug, Clone, Default)]
pub struct Jsonpiler {
  /// Built-in function table.
  builtin: HashMap<String, Builtin>,
  /// Flag to avoid including the same file twice.
  include_flag: HashSet<String>,
  /// Information to be used during parsing.
  pos: Position,
  /// Section of the assembly.
  sect: Section,
  /// Source code.
  source: String,
  /// Cache of the string.
  str_cache: HashMap<String, usize>,
  /// Seed to generate names.
  symbol_seeds: HashMap<String, usize>,
  /// Variable table.
  vars: Vec<HashMap<String, Json>>,
}
impl Jsonpiler {
  /// Format error.
  #[must_use]
  pub(crate) fn fmt_err(&self, err: &str, pos: &Position) -> String {
    let gen_err = |msg: &str| -> String {
      format!("{err}\nError occurred on line: {}\nError position:\n{msg}", pos.line)
    };
    if self.source.is_empty() {
      return gen_err("\n^");
    }
    let len = self.source.len();
    let idx = pos.offset.min(len.saturating_sub(1));
    let start = if idx == 0 {
      0
    } else {
      let Some(left) = self.source.get(..idx) else {
        return gen_err("Error: Failed to get substring");
      };
      match left.rfind('\n') {
        None => 0,
        Some(start_offset) => {
          let Some(res) = start_offset.checked_add(1) else {
            return gen_err("Error: Overflow");
          };
          res
        }
      }
    };
    let Some(right) = self.source.get(idx..) else {
      return gen_err("Error: Failed to get substring");
    };
    let end = match right.find('\n') {
      None => len,
      Some(end_offset) => {
        let Some(res) = idx.checked_add(end_offset) else {
          return gen_err("Error: Overflow");
        };
        res
      }
    };
    let ws = " ".repeat(idx.saturating_sub(start));
    let Some(result) = self.source.get(start..end) else {
      return gen_err("Error: Failed to get substring");
    };
    gen_err(&format!("{result}\n{ws}^"))
  }
}
/// line and pos in source code.
#[derive(Debug, Clone, Default)]
struct Position {
  /// Line number of the part being parsed.
  line: usize,
  /// Byte offset of the part being parsed.
  offset: usize,
}
/// Section of the assembly.
#[derive(Debug, Clone, Default)]
pub(crate) struct Section {
  /// Buffer to store the contents of the bss section of the assembly.
  bss: Vec<String>,
  /// Buffer to store the contents of the data section of the assembly.
  data: Vec<String>,
  /// Buffer to store the contents of the text section of the assembly.
  text: Vec<String>,
}
/// Compiles and executes a JSON-based program using the Jsonpiler.
/// This function performs the following steps:
/// 1. Parses the first CLI argument as the input JSON file path.
/// 2. Reads the file content into a string.
/// 3. Parses the string into a `Json` structure.
/// 4. Compiles the structure into assembly code.
/// 5. Assembles it into an `.obj` file.
/// 6. Links it into an `.exe`.
/// 7. Executes the resulting binary.
/// 8. Returns its exit code.
/// # Panics
/// This function will panic if:
/// - The platform is not Windows.
/// - CLI arguments are invalid.
/// - File reading, parsing, compilation, assembling, linking, or execution fails.
/// - The working directory or executable filename is invalid.
/// # Requirements
/// - `as` and `ld` must be available in the system PATH.
/// - On failure, exits with code 1 using `error_exit`.
/// # Example
/// ```sh
/// ./jsonpiler test.json
/// ```
/// # Platform
/// Windows only.
#[inline]
#[must_use]
pub fn run() -> ExitCode {
  #[cfg(all(not(doc), not(target_os = "windows")))]
  compile_error!("This program is supported on Windows only.");
  let args: Vec<String> = env::args().collect();
  let Some(program_name) = args.first() else { exit!("Failed to get the program name.") };
  let Some(input_file) = args.get(1) else {
    exit!("Usage: {program_name} <input_json_file> [args for .exe]")
  };
  let source = match fs::read_to_string(input_file) {
    Ok(content) => content,
    Err(err) => exit!("Failed to read `{input_file}`: {err}"),
  };
  let mut jsonpiler = Jsonpiler::default();
  let file = Path::new(input_file);
  let with_ext = |ext: &str| -> String { file.with_extension(ext).to_string_lossy().to_string() };
  let asm = with_ext("s");
  let obj = with_ext("obj");
  let exe = with_ext("exe");
  if let Err(err) = jsonpiler.build(source, input_file, &asm) {
    exit!("Compilation error: {err}");
  }
  macro_rules! invoke {
    ($cmd:literal, $list:expr,$name:literal) => {
      match Command::new($cmd).args($list).status() {
        Ok(status) if status.success() => (),
        Ok(_) => exit!("{} returned a non-zero exit status.", $name),
        Err(err) => exit!("Failed to invoke {}: {err}", $name),
      };
    };
  }
  invoke!("as", &[&asm, "-o", &obj], "assembler");
  #[cfg(not(debug_assertions))]
  if let Err(err) = fs::remove_file(&asm) {
    exit!("Failed to delete `{asm}`: {err}")
  }
  invoke!(
    "ld",
    [&obj, "-o", &exe, "-LC:/Windows/System32", "-luser32", "-lkernel32", "-lucrtbase", "-e_start"],
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
  let compiled_program_status = match Command::new(cwd).args(args.get(2..).unwrap_or(&[])).status()
  {
    Ok(status) => status,
    Err(err) => exit!("Failed to execute compiled program: {err}"),
  };
  let Some(exit_code) = compiled_program_status.code() else {
    exit!("Could not get the exit code of the compiled program.")
  };
  let Ok(code) = u8::try_from(exit_code.rem_euclid(256)) else {
    exit!("Internal error: Unexpected error in exit code conversion.")
  };
  ExitCode::from(code)
}
