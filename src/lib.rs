/*!A JSON-based programming language compiler.
Compiles and executes a JSON-based program using the Jsonpiler.
This program performs the following steps:
1. Parses the first CLI argument as the input JSON file path.
2. Reads the file content into a string.
3. Parses the string into a `Json` structure.
4. Compiles the structure into assembly code.
5. Assembles it into an `.obj` file.
6. Links it into an `.exe`.
7. Executes the resulting binary.
8. Returns its exit code.
# Panics
This function will panic if:
- The platform is not Windows.
- CLI arguments are invalid.
- File reading, parsing, compilation, assembling, linking, or execution fails.
- The working directory or executable filename is invalid.
# Requirements
- `as` and `ld` must be available in the system PATH.
- On failure, exits with code 1 using `error_exit`.
# Example
```sh
jsonpiler test.json
```
# Platform
Windows only.
*/
mod bind;
mod builtin;
mod err_msg;
mod func_info;
mod json;
mod label;
mod macros;
mod parser;
mod scope_info;
mod utility;
use core::error::Error;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
#[derive(Debug, Copy, Clone)]
enum ArgLen {
  Any,
  AtLeast(usize),
  #[expect(dead_code, reason = "")]
  AtMost(usize),
  Exactly(usize),
  #[expect(dead_code, reason = "")]
  NoArgs,
  #[expect(dead_code, reason = "")]
  Range(usize, usize),
  SomeArg,
}
#[derive(Debug, Clone)]
struct AsmFunc {
  label: Label,
  params: Vec<WithPos<Json>>,
  ret: Box<Json>,
}
#[derive(Debug, Clone)]
enum Bind<T> {
  Lit(T),
  Var(Label),
}
#[derive(Debug, Clone)]
struct Builtin {
  arg_len: ArgLen,
  func: JFunc,
  scoped: bool,
  skip_eval: bool,
}
type ErrOR<T> = Result<T, Box<dyn Error>>;
#[derive(Debug, Clone)]
struct FuncInfo {
  args: VecDeque<WithPos<Json>>,
  len: usize,
  name: String,
  pos: Position,
}
type JFunc = fn(&mut Jsonpiler, FuncInfo, &mut ScopeInfo) -> ErrOR<Json>;
#[derive(Debug, Clone, Default)]
enum Json {
  Array(Bind<Vec<WithPos<Json>>>),
  Bool(Bind<bool>),
  Float(Bind<f64>),
  Function(AsmFunc),
  Int(Bind<i64>),
  #[default]
  Null,
  Object(Bind<Vec<(WithPos<String>, WithPos<Json>)>>),
  String(Bind<String>),
}
#[derive(Debug, Clone, Default)]
#[doc(hidden)]
pub struct Jsonpiler {
  bss: Vec<String>,
  builtin: HashMap<String, Builtin>,
  data: Vec<String>,
  include_flag: HashSet<String>,
  label_id: usize,
  pos: Position,
  source: Vec<u8>,
  str_cache: HashMap<String, usize>,
  text: Vec<String>,
  vars_global: HashMap<String, Json>,
  vars_local: Vec<HashMap<String, Json>>,
}
#[derive(Debug, Clone)]
struct Label {
  id: usize,
  kind: VarKind,
  size: usize,
}
#[derive(Debug, Clone, Default)]
struct Position {
  line: usize,
  offset: usize,
  size: usize,
}
#[derive(Debug, Clone, Default)]
struct ScopeInfo {
  args_slots: usize,
  body: Vec<String>,
  free_map: BTreeMap<usize, usize>,
  reg_used: HashSet<String>,
  scope_align: usize,
  stack_size: usize,
}
#[derive(Debug, Clone, Copy, PartialEq)]
enum VarKind {
  Global,
  Local,
  Tmp,
}
#[derive(Debug, Clone, Default)]
struct WithPos<T> {
  pos: Position,
  value: T,
}
