//! (main.rs)
//! ```should_panic
//! use jsonpiler::functions::run;
//! fn main() -> ! {
//!  run()
//!}
//! ```
pub mod functions;
mod impl_compiler;
mod impl_object;
mod impl_parser;
mod impl_value;
use core::error::Error;
use std::collections::HashMap;
/// Built-in function.
#[derive(Debug, Clone)]
pub(crate) struct BuiltinFunc {
  /// Should arguments already be evaluated.
  pub evaluated: bool,
  /// Pointer of function.
  pub func: JFunc,
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
/// Built-in function types.
type JFunc = fn(&mut Jsonpiler, &Json, &[Json], &mut String) -> JFuncResult;
/// Contain `JValue` or `Box<dyn Error>`.
type JFuncResult = ErrOR<JValue>;
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
  Function {
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
  Object(JObject),
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
  info: ErrorInfo,
  /// Type and value information.
  value: JValue,
}
/// Parser and compiler.
#[derive(Debug, Clone, Default)]
pub struct Jsonpiler {
  /// Global variables (now on Unused).
  _globals: HashMap<String, JValue>,
  /// Built-in function table.
  f_table: HashMap<String, BuiltinFunc>,
  /// Information to be used during parsing.
  info: ErrorInfo,
  /// Section of the assembly.
  sect: Section,
  /// Seed to generate label names.
  seed: usize,
  /// Source code.
  source: String,
  /// Variable table.
  vars: HashMap<String, JValue>,
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
