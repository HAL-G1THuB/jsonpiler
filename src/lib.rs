//! A JSON-based programming language compiler.
//! (main.rs)
//! ```
//! use jsonpiler::run;
//! use std::process::ExitCode;
//! fn main() -> ExitCode {
//!   run()
//! }
//! ```
mod bind;
mod builtin;
mod compiler;
mod json;
mod name;
mod object;
mod parser;
mod scope_info;
mod utility;
use core::error::Error;
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
  ($self:ident, $dest:expr, $name:literal) => {
    if !$self.include_flag.contains($name) {
      $self.include_flag.insert($name.into());
      $dest.push(include_str!(concat!("asm/", $name, ".s")).into());
    }
  };
}
/// Macro to generate mnemonic.
#[macro_export]
macro_rules! mn {
  ($mne:expr) => {format!("  {}\n", $mne)};
  ($mne:expr, $($arg:expr),+ $(,)?) => {format!("  {} {}\n", $mne, vec![$($arg),+].join(", "))};
}
/// Macro to generate mnemonic.
#[macro_export]
macro_rules! mn_write {
  ($dest:expr, $mne:expr) => {writeln!($dest, "  {}", $mne)};
  ($dest:expr, $mne:expr, $($arg:expr),+ $(,)?) => {
    writeln!($dest, "  {} {}", $mne, vec![$($arg),+].join(", "))
  };
}
/// Assembly boolean representation.
#[derive(Clone)]
struct AsmBool {
  /// bit offset.
  bit: u8,
  /// Name of the variable holding the boolean value.
  name: Name,
}
/// Assembly function representation.
#[derive(Clone)]
struct AsmFunc {
  /// Unique identifier for the function (used to generate a label).
  name: Name,
  /// Parameters of function.
  params: Vec<JsonWithPos>,
  /// Return type of function.
  ret: Box<Json>,
}
/// Represents a value that can be either a literal or a variable.
#[derive(Clone)]
enum Bind<T> {
  /// Literal.
  Lit(T),
  /// Variable binding.
  Var(Name),
}
/// Built-in function.
#[derive(Clone)]
struct Builtin {
  /// Pointer of function.
  func: JFunc,
  /// Whether the function introduces a new scope.
  scoped: bool,
  /// Whether to skip evaluation of arguments before calling the function.
  skip_eval: bool,
}
/// A type alias for `Result<T, Box<dyn Error>>`.
type ErrOR<T> = Result<T, Box<dyn Error>>;
/// Information of arguments.
#[derive(Clone)]
struct FuncInfo {
  /// Arguments.
  args: Vec<JsonWithPos>,
  /// Function Name.
  name: String,
  /// Position of function call.
  pos: Position,
}
/// A type alias for a built-in function pointer.
type JFunc = fn(&mut Jsonpiler, FuncInfo, &mut ScopeInfo) -> ErrOR<Json>;
/// Represents a JSON object with key-value pairs.
#[derive(Clone, Default)]
pub(crate) struct JObject {
  /// Stores key-value pairs in the order they were inserted.
  entries: Vec<(String, JsonWithPos)>,
}
/// Represents a JSON value.
#[derive(Clone, Default)]
enum Json {
  /// Array.
  Array(Bind<Vec<JsonWithPos>>),
  /// Float.
  Float(Bind<f64>),
  /// Function.
  Function(AsmFunc),
  /// Integer.
  Int(Bind<i64>),
  /// Literal boolean.
  LBool(bool),
  /// Null.
  #[default]
  Null,
  /// Object.
  Object(Bind<JObject>),
  /// String.
  String(Bind<String>),
  /// Variable boolean.
  VBool(AsmBool),
}
/// Json object.
#[derive(Clone, Default)]
struct JsonWithPos {
  /// Line number of objects in the source code.
  pos: Position,
  /// Type and value information.
  value: Json,
}
/// Parser and compiler.
#[derive(Clone, Default)]
pub struct Jsonpiler {
  /// Buffer to store the contents of the bss section of the assembly.
  bss: Vec<String>,
  /// Built-in function table.
  builtin: HashMap<String, Builtin>,
  /// Buffer to store the contents of the data section of the assembly.
  data: Vec<String>,
  /// Bit-level allocation for bools.
  global_bool_map: BTreeMap<usize, u8>,
  /// Flag to avoid including the same file twice.
  include_flag: HashSet<String>,
  /// Internal unique ID.
  label_id: usize,
  /// Information to be used during parsing.
  pos: Position,
  /// Source code.
  source: Vec<u8>,
  /// Cache of the string.
  str_cache: HashMap<String, usize>,
  /// Buffer to store the contents of the text section of the assembly.
  text: Vec<String>,
  /// Global variable table.
  vars_global: HashMap<String, Json>,
  /// Local variable table.
  vars_local: Vec<HashMap<String, Json>>,
}
/// Variable name.
#[derive(Clone)]
struct Name {
  /// Variable label.
  id: usize,
  /// Variable type.
  var: VarKind,
}
/// line and pos in source code.
#[derive(Clone, Default)]
struct Position {
  /// Line number of the part being parsed.
  line: usize,
  /// Byte offset of the part being parsed.
  offset: usize,
  /// Size of the part being parsed.
  size: usize,
}
/// Information of Scope.
#[derive(Clone, Default)]
struct ScopeInfo {
  /// Size of arguments.
  args_slots: usize,
  /// Body of function.
  body: Vec<String>,
  /// Bit-level allocation for bools.
  bool_map: BTreeMap<usize, u8>,
  /// Free memory list.
  free_map: BTreeMap<usize, usize>,
  /// Registers used.
  reg_used: HashSet<String>,
  /// Scope align.
  scope_align: usize,
  /// Stack size.
  stack_size: usize,
}
/// Variable.
#[derive(Clone, Copy, PartialEq)]
enum VarKind {
  /// Global variable.
  Global,
  /// Local variable.
  Local,
  /// Temporary local variable.
  Tmp,
}
/// Safe addition.
fn add(op1: usize, op2: usize) -> ErrOR<usize> {
  op1.checked_add(op2).ok_or("InternalError: Overflow occurred".into())
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
  if let Err(err) = jsonpiler.build(source, &asm) {
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
/// Safe subtraction.
fn sub(op1: usize, op2: usize) -> ErrOR<usize> {
  op1.checked_sub(op2).ok_or("InternalError: Underflow occurred".into())
}
