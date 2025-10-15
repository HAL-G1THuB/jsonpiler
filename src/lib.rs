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
This program will panic if:
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
mod dll {
  pub(crate) const GDI32: &str = "gdi32.dll";
  pub(crate) const KERNEL32: &str = "kernel32.dll";
  pub(crate) const USER32: &str = "user32.dll";
}
use parser::Parser;
use scope_info::ScopeInfo;
use std::{collections::HashMap, io, vec::IntoIter};
type BuiltinPtr = fn(&mut Jsonpiler, &mut FuncInfo, &mut ScopeInfo) -> ErrOR<Json>;
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Disp {
  Byte(i8),
  Dword(i32),
  Zero,
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[expect(dead_code, clippy::arbitrary_source_item_ordering)]
enum Register {
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
enum RM {
  Base(Register, Disp),
  Reg(Register),
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
  base: Register,
  disp: Disp,
  index: Register,
  scale: Scale,
}
#[repr(u8)]
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash)]
enum Sect {
  Text,
  Data,
  Rdata,
  Pdata,
  Xdata,
  Bss,
  Idata,
}
struct Assembler {
  addr_sect: HashMap<u32, Sect>,
  dlls: Dlls,
  rva: HashMap<Sect, u32>,
  sym_addr: HashMap<u32, u32>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
#[expect(clippy::min_ident_chars, dead_code)]
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
type Dlls = Vec<(&'static str, Vec<&'static str>)>;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum LogicByteOpcode {
  And = 0x22,
  Or = 0x0A,
  Xor = 0x32,
  Cmp = 0x3A,
  Test = 0x84,
}
#[derive(Clone, Debug)]
enum DataInst {
  Bss(u32, u32, u32),
  Byte(u32, u8),
  #[expect(clippy::box_collection)]
  Bytes(u32, Box<String>),
  Quad(u32, u64),
  RDAlign(usize),
  Seh(u32, u32, u32),
}
#[derive(Clone, Debug)]
enum Inst {
  #[expect(dead_code)]
  AddRId(Register, u32),
  AddRR(Register, Register),
  AddSd(Register, Register),
  CMovCc(ConditionCode, Register, Register),
  Call(u32),
  CallApi((u32, u32)),
  Clear(Register),
  CmpRIb(Register, i8),
  Custom(&'static &'static [u8]),
  CvtSi2Sd(Register, Register),
  CvtTSd2Si(Register, Register),
  DecR(Register),
  DivSd(Register, Register),
  IDivR(Register),
  IMulRR(Register, Register),
  IncR(Register),
  JCc(ConditionCode, u32),
  Jmp(u32),
  Lbl(u32),
  LeaRM(Register, Memory),
  LogicRR(LogicByteOpcode, Register, Register),
  LogicRbRb(LogicByteOpcode, Register, Register),
  MovBB(Box<(Operand<u8>, Operand<u8>)>),
  MovDD(Box<(Operand<u32>, Operand<u32>)>),
  MovQQ(Box<(Operand<u64>, Operand<u64>)>),
  MovSdMX(Memory, Register),
  MovSdXM(Register, Memory),
  MulSd(Register, Register),
  NegR(Register),
  NegRb(Register),
  NotR(Register),
  NotRb(Register),
  Pop(Register),
  Push(Register),
  #[expect(dead_code)]
  SarRIb(Register, u8),
  SetCc(ConditionCode, Register),
  Shl1R(Register),
  ShlRIb(Register, u8),
  ShrRIb(Register, u8),
  SubRId(Register, u32),
  SubRR(Register, Register),
  SubSd(Register, Register),
  TestRdRd(Register, Register),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operand<T> {
  Args(usize),
  Imm(T),
  Mem(Memory),
  Ref(Register),
  Reg(Register),
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
  id: u32,
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
type ErrOR<T> = Result<T, JsonpilerErr>;
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
  data_insts: Vec<DataInst>,
  files: Vec<HashMap<String, AsmFunc>>,
  globals: HashMap<String, Json>,
  import_table: Dlls,
  insts: Vec<Inst>,
  label_id: u32,
  parser: Vec<Parser>,
  startup: Vec<Inst>,
  str_cache: HashMap<String, (u32, usize)>,
  sym_table: HashMap<&'static str, u32>,
  user_defined: HashMap<String, AsmFunc>,
}
#[derive(Debug, Clone, Copy)]
struct Label {
  mem: Memory,
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
enum Memory {
  Global { id: u32, disp: i32 },
  Local { offset: i32 },
  Tmp { offset: i32 },
}
#[derive(Debug, Clone, Default)]
struct WithPos<T> {
  pos: Position,
  value: T,
}
enum JsonpilerErr {
  CompilationError { kind: CompilationErrKind, pos: Position },
  InternalError(InternalErrKind),
}
enum CompilationErrKind {
  ArityError { name: String, expected: Arity, supplied: usize },
  ExistentBuiltin(String),
  ExistentUserDefined(String),
  ExistentVar(String),
  ExpectedTokenError(TokenKind),
  IOError(io::Error),
  IncludeFuncNotFound(Vec<String>),
  IntegerOutOfRange,
  InvalidChar,
  InvalidEsc(char),
  InvalidIdentifier,
  InvalidUnicodeEsc,
  OutSideError { kind: &'static str, place: &'static str },
  ParentDirNotFound,
  ParseError(&'static str),
  RecursiveInclude(String),
  StartsWithZero,
  TooLargeFile,
  TypeError { name: String, expected: String, typ: String },
  UndefinedFn(String),
  UndefinedVar(String),
  UnexpectedLiteral,
  UnexpectedTokenError(TokenKind),
  UnknownType(String),
  UnsupportedExtension,
  UnsupportedType(String),
  UnterminatedLiteral,
  ZeroDivisionError,
}
enum TokenKind {
  Char(char),
  Eof,
  NewLineOrSemiColon,
}
enum InternalErrKind {
  CastError,
  InternalIOError(io::Error),
  InvalidInst(String),
  InvalidScope,
  MismatchReassignment,
  NonExistentArg,
  Overflow,
  TooLargeSection,
  Underflow,
  UnknownLabel,
}
