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
mod object;
mod parser;
mod value;
use {
  core::error::Error,
  std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::Path,
    process::{Command, ExitCode},
  },
};
#[derive(Debug, Clone)]
/// Assembly function representation.
pub(crate) struct AsmFunc {
  /// Name of function.
  pub name: String,
  /// Parameters of function.
  pub params: Vec<Json>,
  /// Return type of function.
  pub ret: Box<JValue>,
}
/// Built-in function.
#[derive(Debug, Clone)]
pub(crate) struct BuiltinFunc {
  /// Should arguments already be evaluated.
  pub do_not_eval: bool,
  /// Pointer of function.
  pub func: JFunc,
  /// If it is true, function introduces a new scope.
  pub scoped: bool,
}
type ErrOR<T> = Result<T, Box<dyn Error>>;
/// line and pos in source code.
#[derive(Debug, Clone, Default)]

pub(crate) struct ErrorInfo {
  /// Line number of the part being parsed.
  line: usize,
  /// Location of the part being parsed.
  pos: usize,
}
/// Contain `JValue` or `Box<dyn Error>`.
type FResult = ErrOR<JValue>;
/// Information of Function.
#[derive(Debug, Clone, Default)]
pub(crate) struct FuncInfo {
  /// Body of function.
  pub body: String,
  /// Registers used.
  pub using_reg: HashSet<String>,
}
/// Type of built-in function.
type JFunc = fn(&mut Jsonpiler, &Json, &[Json], &mut FuncInfo) -> FResult;
/// Represents a JSON object with key-value pairs.
#[derive(Debug, Clone, Default)]
pub(crate) struct JObject {
  /// Stores the key-value pairs in insertion order.
  entries: Vec<(String, Json)>,
  /// Maps keys to their index in the entries vector for quick lookup.
  idx: HashMap<String, usize>,
}
/// Contain `Json` or `Box<dyn Error>`.
type JResult = ErrOR<Json>;
/// Type and value information.
#[derive(Debug, Clone, Default)]
pub(crate) enum JValue {
  /// Function.
  Function(AsmFunc),
  /// Array.
  LArray(Vec<Json>),
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
pub(crate) struct Json {
  /// Line number of objects in the source code.
  info: ErrorInfo,
  /// Type and value information.
  value: JValue,
}
/// Parser and compiler.
#[derive(Debug, Clone, Default)]
pub struct Jsonpiler {
  /// Built-in function table.
  f_table: HashMap<String, BuiltinFunc>,
  /// Flag to avoid including the same file twice.
  include_flag: HashSet<String>,
  /// Information to be used during parsing.
  info: ErrorInfo,
  /// Section of the assembly.
  sect: Section,
  /// Source code.
  source: String,
  /// Seed to generate names.
  symbol_seeds: HashMap<String, usize>,
  /// Variable table.
  vars: Vec<HashMap<String, JValue>>,
}
impl Jsonpiler {
  /// Format error.
  #[must_use]
  pub(crate) fn fmt_err(&self, err: &str, info: &ErrorInfo) -> String {
    const MSG1: &str = "\nError occurred on line: ";
    const MSG2: &str = "\nError position:\n";
    if self.source.is_empty() {
      return format!("{err}{MSG1}{}{MSG2}Error: Empty input", info.line);
    }
    let len = self.source.len();
    let idx = info.pos.min(len.saturating_sub(1));
    let start = if idx == 0 {
      0
    } else {
      let Some(left) = self.source.get(..idx) else {
        return format!("{err}{MSG1}{}{MSG2}Error: Failed to get substring", info.line);
      };
      match left.rfind('\n') {
        None => 0,
        Some(start_pos) => {
          let Some(res) = start_pos.checked_add(1) else {
            return format!("{err}{MSG1}{}{MSG2}Error: Overflow", info.line);
          };
          res
        }
      }
    };
    let Some(right) = self.source.get(idx..) else {
      return format!("{err}{MSG1}{}{MSG2}Error: Failed to get substring", info.line);
    };
    let end = match right.find('\n') {
      None => len,
      Some(end_pos) => {
        let Some(res) = idx.checked_add(end_pos) else {
          return format!("{err}{MSG1}{}{MSG2}Error: Overflow", info.line);
        };
        res
      }
    };
    let ws = " ".repeat(idx.saturating_sub(start));
    let Some(result) = self.source.get(start..end) else {
      return format!("{err}{MSG1}{}{MSG2}Error: Failed to get substring", info.line);
    };
    format!("{err}{MSG1}{}{MSG2}{result}\n{ws}^", info.line)
  }
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
/// Compiles and executes a JSON-based program using the Jsonpiler.
///
/// This function performs the following steps:
/// 1. Parses the first CLI argument as the input JSON file path.
/// 2. Reads the file content into a string.
/// 3. Parses the string into a `Json` structure.
/// 4. Compiles the structure into assembly code.
/// 5. Assembles it into an `.obj` file.
/// 6. Links it into an `.exe`.
/// 7. Executes the resulting binary.
/// 8. Returns its exit code.
///
/// # Panics
/// This function will panic if:
/// - The platform is not Windows.
/// - CLI arguments are invalid.
/// - File reading, parsing, compilation, assembling, linking, or execution fails.
/// - The working directory or executable filename is invalid.
///
/// # Requirements
/// - `as` and `ld` must be available in the system PATH.
/// - On failure, exits with code 1 using `error_exit`.
///
/// # Example
/// ```sh
/// ./jsonpiler test.json
/// ```
///
/// # Platform
/// Windows only.
#[inline]
#[must_use]
#[expect(clippy::print_stderr, reason = "User-facing diagnostics")]
pub fn run() -> ExitCode {
  #[cfg(all(not(doc), not(target_os = "windows")))]
  compile_error!("This program is supported on Windows only.");
  let args: Vec<String> = env::args().collect();
  let Some(program_name) = args.first() else {
    eprintln!("Failed to get the program name.");
    return ExitCode::FAILURE;
  };
  let Some(input_file) = args.get(1) else {
    eprintln!("Usage: {program_name} <input_json_file> [args for .exe]");
    return ExitCode::FAILURE;
  };
  let source = match fs::read_to_string(input_file) {
    Ok(content) => content,
    Err(err) => {
      eprintln!("Failed to read '{input_file}': {err}");
      return ExitCode::FAILURE;
    }
  };
  let mut jsonpiler = Jsonpiler::default();
  let file = Path::new(input_file);
  let asm = file.with_extension("s").to_string_lossy().to_string();
  let obj = file.with_extension("obj").to_string_lossy().to_string();
  let exe = file.with_extension("exe").to_string_lossy().to_string();
  if let Err(err) = jsonpiler.build(source, input_file, &asm) {
    eprintln!("Compilation error: {err}");
    return ExitCode::FAILURE;
  }
  match Command::new("as").args([&asm, "-o", &obj]).status() {
    Ok(status) if status.success() => status,
    Ok(_) => {
      eprintln!("Assembler returned a non-zero exit status.");
      return ExitCode::FAILURE;
    }
    Err(err) => {
      eprintln!("Failed to invoke assembler: {err}");
      return ExitCode::FAILURE;
    }
  };
  #[cfg(not(debug_assertions))]
  if let Err(err) = fs::remove_file(&asm) {
    eprintln!("Failed to delete '{asm}': {err}");
    return ExitCode::FAILURE;
  }
  match Command::new("ld")
    .args([
      &obj,
      "-o",
      &exe,
      "-LC:/Windows/System32",
      "-luser32",
      "-lkernel32",
      "-lucrtbase",
      "--gc-sections",
      "-e_start",
    ])
    .status()
  {
    Ok(status) if status.success() => status,
    Ok(_) => {
      eprintln!("Linker returned a non-zero exit status.");
      return ExitCode::FAILURE;
    }
    Err(err) => {
      eprintln!("Failed to invoke linker: {err}");
      return ExitCode::FAILURE;
    }
  };
  if let Err(err) = fs::remove_file(&obj) {
    eprintln!("Failed to delete '{obj}': {err}");
    return ExitCode::FAILURE;
  }
  let cwd = match env::current_dir() {
    Ok(dir) => dir,
    Err(err) => {
      eprintln!("Failed to get current directory: {err}");
      return ExitCode::FAILURE;
    }
  };
  let exe_status = match Command::new(cwd.join(&exe)).args(args.get(2..).unwrap_or(&[])).status() {
    Ok(status) => status,
    Err(err) => {
      eprintln!("Failed to execute compiled program: {err}");
      return ExitCode::FAILURE;
    }
  };
  let Some(exit_code) = exe_status.code() else {
    eprintln!("Could not retrieve the child process's exit code.");
    return ExitCode::FAILURE;
  };
  if let Ok(code) = u8::try_from(exit_code.rem_euclid(256)) {
    ExitCode::from(code)
  } else {
    eprintln!("Internal error: Unexpected failure in exit code conversion.");
    ExitCode::FAILURE
  }
}
