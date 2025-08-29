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
mod assembler;
mod builtin;
mod json;
mod macros;
mod other;
mod parser;
mod portable_executable;
mod scope_info;
mod utility;
use core::error::Error;
use parser::Parser;
use scope_info::ScopeInfo;
use std::{collections::HashMap, vec::IntoIter};
type BuiltinPtr = fn(&mut Jsonpiler, &mut FuncInfo, &mut ScopeInfo) -> ErrOR<Json>;
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[expect(clippy::allow_attributes)]
enum Disp {
  #[allow(dead_code)]
  Byte(i8),
  Dword(i32),
  Zero,
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[expect(dead_code, clippy::arbitrary_source_item_ordering)]
enum Reg {
  Rax = 0,
  Rcx = 1,
  Rdx = 2,
  Rbx = 3,
  Rsp = 4,
  Rbp = 5,
  Rsi = 6,
  Rdi = 7,
  R8 = 8,
  R9 = 9,
  R10 = 10,
  R11 = 11,
  R12 = 12,
  R13 = 13,
  R14 = 14,
  R15 = 15,
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[expect(clippy::allow_attributes)]
enum Memory {
  #[allow(dead_code)]
  Base(Reg, Disp),
  Reg(Reg),
  RipRel(i32),
  #[allow(dead_code)]
  Sib(Sib),
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[expect(dead_code)]
enum Scale {
  S1 = 0,
  S2 = 1,
  S4 = 2,
  S8 = 3,
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Sib {
  base: Reg,
  disp: Disp,
  index: Reg,
  scale: Scale,
}
#[repr(u8)]
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash)]
enum Sect {
  Bss,
  Data,
  Idata,
  Text,
}
struct Assembler {
  addr_sect: HashMap<usize, Sect>,
  dlls: Dlls,
  rva: HashMap<Sect, u32>,
  sym_addr: HashMap<usize, u32>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
#[expect(clippy::arbitrary_source_item_ordering, clippy::min_ident_chars, dead_code)]
enum ConditionCode {
  O = 0,
  No = 1,
  B = 2,
  Ae = 3,
  E = 4,
  Ne = 5,
  Be = 6,
  A = 7,
  S = 8,
  Ns = 9,
  P = 10,
  Np = 11,
  L = 12,
  Ge = 13,
  Le = 14,
  G = 15,
}
type Dlls = Vec<(&'static str, Vec<(u16, &'static str)>)>;
#[derive(Clone, Debug)]
enum Inst {
  AddRId(Reg, u32),
  AddRR(Reg, Reg),
  AddSd(Reg, Reg),
  AndRbRb(Reg, Reg),
  Bss(usize, u32),
  Byte(usize, u8),
  Call(usize),
  CallApi((usize, usize)),
  Clear(Reg),
  CmpRIb(Reg, i8),
  CmpRR(Reg, Reg),
  Custom(Vec<u8>),
  CvtTSd2Si(Reg, Reg),
  #[expect(dead_code)]
  DecR(Reg),
  DivSd(Reg, Reg),
  IDivR(Reg),
  IMulRR(Reg, Reg),
  Jcc(ConditionCode, usize),
  Jmp(usize),
  JmpSh(usize),
  Lbl(usize),
  LeaRM(Reg, VarKind),
  #[expect(dead_code)]
  MovMId(VarKind, u32),
  MovMbIb(VarKind, u8),
  MovMbRb(VarKind, Reg),
  MovQQ(OpQ, OpQ),
  MovRId(Reg, u32),
  MovRbIb(Reg, u8),
  MovRbMb(Reg, VarKind),
  MovSdMX(VarKind, Reg),
  MovSdXM(Reg, VarKind),
  MulSd(Reg, Reg),
  NegR(Reg),
  NotRb(Reg),
  OrRbRb(Reg, Reg),
  Pop(Reg),
  Push(Reg),
  Quad(usize, u64),
  Ret,
  Shl1R(Reg),
  ShlRIb(Reg, u8),
  ShrRIb(Reg, u8),
  StringZ(usize, String),
  SubRId(Reg, u32),
  SubRR(Reg, Reg),
  SubSd(Reg, Reg),
  TestRR(Reg, Reg),
  TestRbRb(Reg, Reg),
  TestRdRd(Reg, Reg),
  XorRR(Reg, Reg),
  XorRbRb(Reg, Reg),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpQ {
  Args(usize),
  Iq(u64),
  Mq(VarKind),
  Ref(Reg),
  Rq(Reg),
}
#[derive(Copy, Clone)]
enum Arity {
  Any,
  AtLeast(usize),
  #[expect(dead_code)]
  AtMost(usize),
  Exactly(usize),
  NoArgs,
  #[expect(dead_code)]
  Range(usize, usize),
}
#[derive(Debug, Clone)]
struct AsmFunc {
  file: usize,
  id: usize,
  params: Vec<Json>,
  ret: Json,
}
#[derive(Debug, Clone)]
enum Bind<T> {
  Lit(T),
  Var(Label),
}
struct Builtin {
  arg_len: Arity,
  ptr: BuiltinPtr,
  scoped: bool,
  skip_eval: bool,
}
type ErrOR<T> = Result<T, Box<dyn Error>>;
struct FuncInfo {
  args: IntoIter<WithPos<Json>>,
  free_list: Vec<(i32, i32)>,
  len: usize,
  name: String,
  nth: usize,
  pos: Position,
}
#[derive(Debug, Clone, Default)]
enum Json {
  Array(Bind<Vec<WithPos<Json>>>),
  Bool(Bind<bool>),
  Float(Bind<f64>),
  Int(Bind<i64>),
  #[default]
  Null,
  Object(Bind<Vec<(WithPos<String>, WithPos<Json>)>>),
  String(Bind<String>),
}
#[doc(hidden)]
pub struct Jsonpiler {
  builtin: HashMap<String, Builtin>,
  files: Vec<HashMap<String, AsmFunc>>,
  globals: HashMap<String, Json>,
  import_table: Dlls,
  insts: Vec<Inst>,
  label_id: usize,
  parser: Vec<Parser>,
  str_cache: HashMap<String, usize>,
  sym_table: HashMap<&'static str, usize>,
  user_defined: HashMap<String, AsmFunc>,
}
#[derive(Debug, Clone, Copy)]
struct Label {
  kind: VarKind,
  size: i32,
}
#[derive(Debug, Copy, Clone, Default)]
struct Position {
  file: usize,
  line: usize,
  offset: usize,
  size: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VarKind {
  Global { id: usize, disp: i32 },
  Local { offset: i32 },
  Tmp { offset: i32 },
}
#[derive(Debug, Clone, Default)]
struct WithPos<T> {
  pos: Position,
  value: T,
}
