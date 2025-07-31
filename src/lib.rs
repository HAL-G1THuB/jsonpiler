//! A JSON-based programming language compiler.
//! Compiles and executes a JSON-based program using the Jsonpiler.
//! This program performs the following steps:
//! 1. Parses the first CLI argument as the input JSON file path.
//! 2. Reads the file content into a string.
//! 3. Parses the string into a `Json` structure.
//! 4. Compiles the structure into assembly code.
//! 5. Assembles it into an `.obj` file.
//! 6. Links it into an `.exe`.
//! 7. Executes the resulting binary.
//! 8. Returns its exit code.
//! # Panics
//! This function will panic if:
//! - The platform is not Windows.
//! - CLI arguments are invalid.
//! - File reading, parsing, compilation, assembling, linking, or execution fails.
//! - The working directory or executable filename is invalid.
//! # Requirements
//! - `as` and `ld` must be available in the system PATH.
//! - On failure, exits with code 1 using `error_exit`.
//! # Example
//! ```sh
//! ./jsonpiler test.json
//! ```
//! # Platform
//! Windows only.
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
mod object;
mod parser;
mod scope_info;
mod utility;
mod variable;
use core::error::Error;
use std::{
  collections::{BTreeMap, HashMap, HashSet},
  env, fs,
  path::Path,
  process::{Command, ExitCode},
};
#[macro_export]
#[doc(hidden)]
macro_rules! add {
  ($op1: expr, $op2: expr) => {
    $op1.checked_add($op2).ok_or("InternalError: Overflow occurred")
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! err {
  ($self:ident, $pos:expr, $($arg:tt)*) => {Err($self.fmt_err(&format!($($arg)*), &$pos).into())};
  ($self:ident, $($arg:tt)*) => {Err($self.fmt_err(&format!($($arg)*), &$self.pos).into())};
}
macro_rules! exit {($($arg: tt)*) =>{{eprintln!($($arg)*);return ExitCode::FAILURE;}}}
#[macro_export]
#[doc(hidden)]
macro_rules! include_once {
  ($self:ident, $dest:expr, $name:literal) => {
    if !$self.include_flag.contains($name) {
      $self.include_flag.insert($name.into());
      $dest.push(include_str!(concat!("asm/", $name, ".s")).into());
    }
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! mn {
  ($mne:expr) => {format!("\t{}\n", $mne)};
  ($mne:expr, $($arg:expr),+ $(,)?) => {format!("\t{}\t{}\n", $mne, vec![$(format!("{}", $arg)),+].join(",\t"))};
}
#[macro_export]
#[doc(hidden)]
macro_rules! mn_write {
  ($dest:expr, $mne:expr) => {writeln!($dest, "\t{}", $mne)};
  ($dest:expr, $mne:expr, $($arg:expr),+ $(,)?) => {
    writeln!($dest, "\t{}\t{}", $mne, vec![$(format!("{}", $arg)),+].join(",\t"))
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! sub {
  ($op1: expr, $op2: expr) => {
    $op1.checked_sub($op2).ok_or("InternalError: Underflow occurred")
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! warn {
  ($self:ident, $pos:expr, $($arg:tt)*) => {println!("Warning: {}", $self.fmt_err(&format!($($arg)*), &$pos))};
  ($self:ident, $($arg:tt)*) => {println!("Warning: {}", $self.fmt_err(&format!($($arg)*), &$self.pos))};
}
#[derive(Debug, Clone)]
struct AsmBool {
  bit: u8,
  name: Variable,
}
#[derive(Debug, Clone)]
struct AsmFunc {
  name: Variable,
  params: Vec<JsonWithPos>,
  ret: Box<Json>,
}
#[derive(Debug, Clone)]
enum Bind<T> {
  Lit(T),
  Var(Variable),
}
#[derive(Debug, Clone)]
struct Builtin {
  func: JFunc,
  scoped: bool,
  skip_eval: bool,
}
type ErrOR<T> = Result<T, Box<dyn Error>>;
#[derive(Debug, Clone)]
struct FuncInfo {
  args: Vec<JsonWithPos>,
  name: String,
  pos: Position,
}
type JFunc = fn(&mut Jsonpiler, FuncInfo, &mut ScopeInfo) -> ErrOR<Json>;
#[derive(Debug, Clone, Default)]
pub(crate) struct JObject {
  entries: Vec<(String, JsonWithPos)>,
}
#[derive(Debug, Clone, Default)]
enum Json {
  Array(Bind<Vec<JsonWithPos>>),
  Float(Bind<f64>),
  Function(AsmFunc),
  Int(Bind<i64>),
  LBool(bool),
  #[default]
  Null,
  Object(Bind<JObject>),
  String(Bind<String>),
  VBool(AsmBool),
}
#[derive(Debug, Clone, Default)]
struct JsonWithPos {
  pos: Position,
  value: Json,
}
/// Parser and compiler.
#[derive(Debug, Clone, Default)]
#[doc(hidden)]
pub struct Jsonpiler {
  bss: Vec<String>,
  builtin: HashMap<String, Builtin>,
  data: Vec<String>,
  global_bool_map: BTreeMap<isize, u8>,
  include_flag: HashSet<String>,
  label_id: isize,
  pos: Position,
  source: Vec<u8>,
  str_cache: HashMap<String, isize>,
  text: Vec<String>,
  vars_global: HashMap<String, Json>,
  vars_local: Vec<HashMap<String, Json>>,
}
#[derive(Debug, Clone, Default)]
struct Position {
  line: usize,
  offset: usize,
  size: usize,
}
#[derive(Debug, Clone, Default)]
struct ScopeInfo {
  args_slots: isize,
  body: Vec<String>,
  bool_map: BTreeMap<isize, u8>,
  free_map: BTreeMap<isize, isize>,
  reg_used: HashSet<String>,
  scope_align: isize,
  stack_size: isize,
}
#[derive(Debug, Clone, Copy, PartialEq)]
enum VarKind {
  Global,
  Local,
  Tmp,
}
#[derive(Debug, Clone)]
struct Variable {
  byte: isize,
  id: isize,
  kind: VarKind,
}
#[inline]
#[must_use]
#[doc(hidden)]
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
