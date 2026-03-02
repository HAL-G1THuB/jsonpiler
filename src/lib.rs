mod assembler;
mod builtin;
mod consts;
mod intrinsic;
mod json;
mod macros;
mod other;
mod parser;
mod prelude;
mod scope;
mod utility;
use prelude::*;
use std::vec::IntoIter;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scale {
  #[expect(dead_code)]
  S1 = 0,
  #[expect(dead_code)]
  S2 = 1,
  S4 = 2,
  #[expect(dead_code)]
  S8 = 3,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(clippy::arbitrary_source_item_ordering)]
struct Sib {
  scale: Scale,
  index: Register,
  base: Register,
}
#[repr(u8)]
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Copy)]
enum Sect {
  Text,
  Data,
  Rdata,
  Pdata,
  Xdata,
  Bss,
  Idata,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
#[expect(clippy::min_ident_chars)]
enum ConditionCode {
  #[expect(dead_code)]
  O = 0,
  #[expect(dead_code)]
  No = 1,
  B = 2,
  #[expect(dead_code)]
  Ae = 3,
  E = 4,
  Ne = 5,
  #[expect(dead_code)]
  Be = 6,
  #[expect(dead_code)]
  A = 7,
  #[expect(dead_code)]
  S = 8,
  #[expect(dead_code)]
  Ns = 9,
  #[expect(dead_code)]
  P = 10,
  #[expect(dead_code)]
  Np = 11,
  L = 12,
  Ge = 13,
  Le = 14,
  G = 15,
}
type Dll = (&'static str, Vec<&'static str>);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum LogicOpcode {
  And = 0x22,
  Or = 0x0A,
  Xor = 0x32,
  Cmp = 0x3A,
  Test = 0x84,
}
#[derive(Clone, Debug)]
enum DataInst {
  BssAlloc(u32, u32, u32),
  Byte(u32, u8),
  #[expect(clippy::box_collection)]
  Bytes(u32, Box<String>),
  Quad(u32, u64),
  Seh(u32, u32, u32),
  #[expect(clippy::box_collection)]
  WChars(u32, Box<String>),
}
#[must_use]
#[derive(Copy, Clone, Debug)]
enum Inst {
  AddRId(Register, u32),
  AddRR(Register, Register),
  AddSd(Register, Register),
  CMovCc(ConditionCode, Register, Register),
  Call(u32),
  CallApi((u32, u32)),
  CallApiNull((u32, u32)),
  Clear(Register),
  CmpRIb(Register, i8),
  Custom(&'static [u8]),
  CvtSi2Sd(Register, Register),
  CvtTSd2Si(Register, Register),
  DecMd(Address),
  DecR(Register),
  DivSd(Register, Register),
  IDivR(Register),
  IMulRR(Register, Register),
  IncMd(Address),
  IncR(Register),
  JCc(ConditionCode, u32),
  Jmp(u32),
  Lbl(u32),
  LeaRM(Register, Address),
  LogicRR(LogicOpcode, Register, Register),
  LogicRbRb(LogicOpcode, Register, Register),
  MovBB((Operand<u8>, Operand<u8>)),
  MovDD((Operand<u32>, Operand<u32>)),
  MovQQ((Operand<u64>, Operand<u64>)),
  MovSdMX(Address, Register),
  MovSdRefX(Register, Register),
  MovSdXM(Register, Address),
  MovSdXRef(Register, Register),
  MovSxDRMd(Register, Address),
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
  SqrtSd(Register, Register),
  SubRId(Register, u32),
  SubRR(Register, Register),
  SubSd(Register, Register),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operand<T> {
  Args(i32),
  Imm(T),
  Mem(Address),
  Ref(Register),
  Reg(Register),
  Sib(Sib, Disp),
}
#[derive(Debug, Copy, Clone)]
enum Arity {
  Any,
  AtLeast(usize),
  #[expect(dead_code)]
  AtMost(usize),
  Exactly(usize),
  #[expect(dead_code)]
  Range(usize, usize),
  Zero,
}
#[derive(Debug, Clone)]
struct AsmFunc {
  id: u32,
  params: Vec<Json>,
  ret: Json,
}
#[derive(Debug, Clone)]
enum Bind<T> {
  Lit(T),
  Var(Label),
}
type BuiltinPtr = fn(&mut Jsonpiler, &mut Function, &mut Scope) -> ErrOR<Json>;
#[derive(Debug, Clone)]
struct BuiltinFunc {
  arity: Arity,
  builtin_ptr: BuiltinPtr,
  scoped: bool,
  skip_eval: bool,
}
type ErrOR<T> = Result<T, JsonpilerErr>;
#[derive(Debug, Clone)]
struct Function {
  args: IntoIter<WithPos<Json>>,
  free_vec: Vec<(i32, LabelSize)>,
  len: usize,
  name: String,
  nth: usize,
  pos: Position,
}
type KeyVal = (WithPos<String>, WithPos<Json>);
#[derive(Debug, Clone, Default)]
enum Json {
  Array(Bind<Vec<WithPos<Json>>>),
  Bool(Bind<bool>),
  Float(Bind<f64>),
  Int(Bind<i64>),
  #[default]
  Null,
  Object(Bind<Vec<KeyVal>>),
  Str(Bind<String>),
}
#[doc(hidden)]
#[derive(Default)]
pub struct Jsonpiler {
  builtin: HashMap<String, BuiltinFunc>,
  data_insts: Vec<DataInst>,
  dlls: Vec<Dll>,
  files: Vec<HashMap<String, WithPos<AsmFunc>>>,
  globals: HashMap<String, Json>,
  id_seed: u32,
  insts: Vec<Inst>,
  parser: Vec<Parser>,
  release: bool,
  startup: Vec<Inst>,
  str_cache: HashMap<String, u32>,
  symbols: HashMap<&'static str, u32>,
  user_defined: HashMap<String, WithPos<AsmFunc>>,
}
#[derive(Debug, Clone, Copy, Default)]
struct Label(Address, LabelSize);
#[derive(Eq, PartialEq, Debug, Clone, Copy, Default)]
enum LabelSize {
  #[default]
  Heap,
  Size(i32),
}
#[derive(Debug, Copy, Clone, Default)]
struct Position {
  file: usize,
  line: usize,
  offset: usize,
  size: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Address {
  Global(u32),
  Local(Lifetime, i32),
}
//struct LAddr(Lifetime, i32);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lifetime {
  Long,
  Tmp,
}
#[derive(Debug, Clone, Default)]
struct WithPos<T> {
  pos: Position,
  val: T,
}
