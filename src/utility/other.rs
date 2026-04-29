use crate::prelude::*;
use std::ops::Add;
use std::vec::IntoIter;
pub(crate) type BuiltInPtr = fn(&mut Jsonpiler, &mut Pos<BuiltIn>, &mut Scope) -> ErrOR<Json>;
pub(crate) type Dll = (String, Vec<String>);
pub(crate) type FileId = u32;
pub(crate) type LabelId = u32;
pub(crate) type Seh = Vec<(LabelId, LabelId, i32)>;
#[derive(Debug, Clone)]
pub(crate) enum Bind<T> {
  Lit(T),
  Var(Memory),
}
impl<T: Copy> Copy for Bind<T> {}
#[derive(Debug, Clone)]
pub(crate) struct BuiltIn {
  pub args: IntoIter<Pos<Json>>,
  pub free_list: BTreeSet<Memory>,
  pub len: u32,
  pub name: String,
  pub nth: u32,
}
#[derive(Debug, Clone, Copy, Default, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct Memory(pub Address, pub MemoryType);
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct MemoryType {
  pub heap: Storage,
  pub size: MemorySize,
}
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Storage {
  HeapPtr,
  #[default]
  Value,
}
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) enum MemorySize {
  Dynamic,
  Known(i32),
  Small(RegSize),
}
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) enum RegSize {
  RB = 1,
  RD = 4,
  RQ = 8,
}
#[derive(Debug, Clone, Copy, Ord, PartialOrd, PartialEq, Eq)]
pub(crate) enum Address {
  Global(LabelId),
  Local(Lifetime, i32),
}
#[derive(Debug, Clone, Copy, Ord, PartialOrd, PartialEq, Eq)]
pub(crate) enum Lifetime {
  Long,
  Tmp,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct Pos<T> {
  pub pos: Position,
  pub val: T,
}
impl<T: Copy> Copy for Pos<T> {}
#[derive(Debug, Clone)]
pub(crate) struct UserDefinedInfo {
  pub dep: Dependency,
  pub params: Vec<(String, JsonType)>,
  pub refs: Vec<Position>,
  pub ret_type: JsonType,
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct BuiltInInfo {
  pub arity: Arity,
  pub builtin_ptr: BuiltInPtr,
  pub scoped: bool,
  pub skip_eval: bool,
}
#[derive(Debug, Clone)]
pub(crate) struct CompiledFunc {
  pub dep: Dependency,
  pub insts: Vec<Inst>,
  pub seh: Option<(LabelId, i32)>,
}
impl Default for Address {
  fn default() -> Self {
    Local(Tmp, 0)
  }
}
impl Default for MemorySize {
  fn default() -> Self {
    Small(RQ)
  }
}
impl<T> Default for Bind<T> {
  fn default() -> Self {
    Var(Memory::default())
  }
}
impl Pos<BuiltIn> {
  pub(crate) fn arg(&mut self) -> ErrOR<Pos<Json>> {
    self.val.nth += 1;
    self.val.args.next().ok_or_else(|| Internal(ArgNotFound(self.val.name.clone(), self.val.nth)))
  }
  pub(crate) fn push_free_tmp(&mut self, memory_opt: Option<Memory>) {
    if let Some(memory @ Memory(Local(Tmp, _), _)) = memory_opt {
      self.val.free_list.insert(memory);
    }
  }
}
impl<T> Pos<T> {
  pub(crate) fn map<F: Fn(T) -> V, V>(self, map_f: F) -> Pos<V> {
    self.pos.with(map_f(self.val))
  }
  pub(crate) fn map_ref<F: Fn(&T) -> V, V>(&self, map_f: F) -> Pos<V> {
    self.pos.with(map_f(&self.val))
  }
}
impl MemoryType {
  pub(crate) fn size(self) -> i32 {
    if self.heap == HeapPtr {
      8
    } else {
      match self.size {
        Known(size) => size,
        Small(size) => size as i32,
        Dynamic => 8,
      }
    }
  }
}
impl Address {
  pub(crate) fn modrm_sib_disp(self) -> u32 {
    match self {
      Global(_) => 5,
      Local(_, offset) => 1 + Disp::from(offset).sizeof(Rbp as u8),
    }
  }
}
impl<T> From<T> for Operand<T>
where
  T: Copy + Add<Output = T>,
{
  fn from(src: T) -> Operand<T> {
    Imm(src)
  }
}
impl<T> From<Register> for Operand<T> {
  fn from(src: Register) -> Operand<T> {
    Reg(src)
  }
}
impl<T> From<Address> for Operand<T> {
  fn from(src: Address) -> Operand<T> {
    Mem(src)
  }
}
