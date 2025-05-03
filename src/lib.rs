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
mod utility;
use core::{
  error::Error,
  fmt::{self, Display},
};
use std::{
  collections::{BTreeMap, HashMap, HashSet},
  env, fs,
  path::Path,
  process::{Command, ExitCode},
};
/// Generates an error.
#[macro_export]
macro_rules! err {
  ($self:ident, $pos:expr, $($arg:tt)*) => {Err($self.fmt_err(&format!($($arg)*), &$pos).into())};
  ($self:ident, $($arg:tt)*) => {Err($self.fmt_err(&format!($($arg)*), &$self.pos).into())};
}
/// Return `ExitCode`.
macro_rules! exit {($($arg: tt)*) =>{{eprintln!($($arg)*);return ExitCode::FAILURE;}}}
/// Macro to include assembly files only once.
#[macro_export]
macro_rules! include_once {
  ($self:ident, $name:literal) => {
    if !$self.include_flag.contains($name) {
      $self.include_flag.insert($name.into());
      $self.text.push(include_str!(concat!("asm/", $name, ".s")).into());
    }
  };
}
/// Arguments.
type Args = Vec<JsonWithPos>;
#[derive(Debug, Clone)]
/// Assembly boolean representation.
struct AsmBool {
  /// bit offset.
  bit: usize,
  /// Name of function.
  name: Name,
}
/// Assembly function representation.
#[derive(Debug, Clone)]
struct AsmFunc {
  /// Name of function.
  name: usize,
  /// Parameters of function.
  params: Vec<JsonWithPos>,
  /// Return type of function.
  ret: Box<Json>,
}
/// Binding.
#[derive(Debug, Clone)]
enum Bind<T> {
  /// Literal.
  Lit(T),
  /// Global variable.
  Var(Name),
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
/// Contain `T` or `Box<dyn Error>`.
type ErrOR<T> = Result<T, Box<dyn Error>>;
/// Information of Function.
#[derive(Debug, Clone, Default)]
struct FuncInfo {
  /// Size of arguments.
  args_slots: usize,
  /// Body of function.
  body: Vec<String>,
  /// Free memory list.
  free_map: BTreeMap<usize, usize>,
  /// Registers used.
  reg_used: HashSet<String>,
  /// Scope align.
  scope_align: usize,
  /// Stack size.
  stack_size: usize,
}
/// Type of global variable.
enum GVar {
  /// BSS variable.
  Bss,
  /// Global function.
  Fnc,
  /// Global integer.
  Int,
  /// Global string.
  Str,
}
/// Type of built-in function.
type JFunc = fn(&mut Jsonpiler, &JsonWithPos, Args, &mut FuncInfo) -> ErrOR<Json>;
/// Represents a JSON object with key-value pairs.
#[derive(Debug, Clone, Default)]
pub(crate) struct JObject {
  /// Stores key-value pairs in the order they were inserted.
  entries: Vec<(String, JsonWithPos)>,
}
/// Type and value information.
#[derive(Debug, Clone, Default)]
enum Json {
  /// Array.
  Array(Bind<Vec<JsonWithPos>>),
  /// Float.
  Float(Bind<f64>),
  /// Function.
  Function(AsmFunc),
  /// Integer.
  Int(Bind<i64>),
  /// Bool.
  LBool(bool),
  /// Null.
  #[default]
  Null,
  /// Object.
  Object(Bind<JObject>),
  /// String.
  String(Bind<String>),
  /// Bool variable.
  #[expect(dead_code, reason = "todo")]
  VBool(AsmBool),
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
  /// Buffer to store the contents of the bss section of the assembly.
  bss: Vec<String>,
  /// Built-in function table.
  builtin: HashMap<String, Builtin>,
  /// Buffer to store the contents of the data section of the assembly.
  data: Vec<String>,
  /// Seed to generate names.
  global_seed: usize,
  /// Flag to avoid including the same file twice.
  include_flag: HashSet<String>,
  /// Information to be used during parsing.
  pos: Position,
  /// Source code.
  source: Vec<u8>,
  /// Cache of the string.
  str_cache: HashMap<String, usize>,
  /// Buffer to store the contents of the text section of the assembly.
  text: Vec<String>,
  /// Variable table.
  vars: Vec<HashMap<String, Json>>,
}
/// Variable name.
#[derive(Debug, Clone)]
struct Name {
  /// Variable seed.
  seed: usize,
  /// Variable type.
  var: Var,
}
impl Display for Name {
  #[expect(clippy::min_ident_chars, reason = "default name is 'f'")]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self.var {
      Var::Global => write!(f, " ptr .L{:x}[rip]", self.seed),
      Var::Local | Var::Tmp => write!(f, " ptr -0x{:x}[rbp]", self.seed),
    }
  }
}
/// line and pos in source code.
#[derive(Debug, Clone, Default)]
struct Position {
  /// Line number of the part being parsed.
  line: usize,
  /// Byte offset of the part being parsed.
  offset: usize,
  /// Size of the part being parsed.
  size: usize,
}
/// Variable.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Var {
  /// Global variable.
  Global,
  /// Local variable.
  Local,
  /// Temporary local variable.
  Tmp,
}
/// Safe addition.
fn add(op1: usize, op2: usize) -> ErrOR<usize> {
  op1.checked_add(op2).ok_or("InternalError: Overflow".into())
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
  if let Err(err) = jsonpiler.build(&source, &asm) {
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
    [&obj, "-o", &exe, "-LC:/Windows/System32", "-luser32", "-lkernel32", "-lucrtbase", "-emain"],
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
  let exe_status = match Command::new(cwd).args(args.get(2..).unwrap_or(&[])).status() {
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
