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
mod compile_context;
mod func_info;
mod json;
mod label;
mod macros;
mod parser;
mod scope_info;
mod utility;
use compile_context::CompileContext;
use core::error::Error;
use parser::Parser;
use std::{
  collections::{BTreeMap, BTreeSet, HashMap},
  fs::File,
  io::BufWriter,
  vec::IntoIter,
};
#[derive(Debug, Copy, Clone)]
enum Arity {
  Any,
  AtLeast(usize),
  #[expect(dead_code)]
  AtMost(usize),
  Exactly(usize),
  #[expect(dead_code)]
  NoArgs,
  #[expect(dead_code)]
  Range(usize, usize),
}
#[derive(Debug, Clone)]
struct AsmFunc {
  label: Label,
  params: Vec<Json>,
  ret: Box<Json>,
}
#[derive(Debug, Clone)]
enum Bind<T> {
  Lit(T),
  Var(Label),
}
#[derive(Debug)]
struct Builtin {
  arg_len: Arity,
  func: JFunc,
  scoped: bool,
  skip_eval: bool,
}
type ErrOR<T> = Result<T, Box<dyn Error>>;
#[derive(Debug)]
struct FuncInfo {
  args: IntoIter<WithPos<Json>>,
  free_list: Vec<Label>,
  len: usize,
  name: String,
  pos: Position,
}
type JFunc = fn(&mut Jsonpiler, &mut FuncInfo, &mut ScopeInfo) -> ErrOR<Json>;
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
#[derive(Debug)]
#[doc(hidden)]
pub struct Jsonpiler {
  bss: Vec<(usize, usize)>,
  builtin: HashMap<String, Builtin>,
  ctx: CompileContext,
  data: BufWriter<File>,
  globals: HashMap<String, Json>,
  parser: Parser,
  text: Vec<String>,
}
#[derive(Debug, Clone, Copy)]
struct Label {
  id: usize,
  kind: VarKind,
  size: usize,
}
#[derive(Debug, Copy, Clone, Default)]
struct Position {
  line: usize,
  offset: usize,
  size: usize,
}
#[derive(Debug)]
struct ScopeInfo {
  alloc_map: BTreeMap<usize, usize>,
  args_slots: usize,
  body: Vec<String>,
  locals: Vec<HashMap<String, Json>>,
  reg_used: BTreeSet<String>,
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
