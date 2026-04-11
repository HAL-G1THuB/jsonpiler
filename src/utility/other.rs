use crate::prelude::*;
use core::ops::Add;
use std::vec::IntoIter;
pub(crate) type BuiltinPtr = fn(&mut Jsonpiler, &mut BuiltIn, &mut Scope) -> ErrOR<Json>;
pub(crate) type Dll = (String, Vec<String>);
pub(crate) type FileId = u32;
pub(crate) type LabelId = u32;
#[derive(Debug, Clone)]
pub(crate) enum Bind<T> {
  Lit(T),
  Var(Memory),
}
impl<T: Copy> Copy for Bind<T> {}
#[derive(Debug, Clone)]
pub(crate) struct BuiltIn {
  pub args: IntoIter<WithPos<Json>>,
  pub free_vec: Vec<(i32, MemoryType)>,
  pub len: u32,
  pub name: String,
  pub nth: u32,
  pub pos: Position,
}
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Memory(pub Address, pub MemoryType);
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum MemoryType {
  // Some(size) -> size < 8byte | None -> ptr
  Heap(Option<i32>),
  Size(i32),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Address {
  Global(LabelId),
  Local(Lifetime, i32),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lifetime {
  Long,
  Tmp,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct WithPos<T> {
  pub pos: Position,
  pub val: T,
}
impl<T: Copy> Copy for WithPos<T> {}
#[derive(Debug, Clone)]
pub(crate) struct UserDefinedInfo {
  pub id: LabelId,
  pub params: Vec<JsonType>,
  pub ret_type: JsonType,
  pub uses: Vec<LabelId>,
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct BuiltInInfo {
  pub arity: Arity,
  pub builtin_ptr: BuiltinPtr,
  pub scoped: bool,
  pub skip_eval: bool,
}
#[derive(Debug, Clone)]
pub(crate) struct CompiledFunc {
  //  pub id: LabelId,
  pub insts: Vec<Inst>,
  pub seh: Option<(LabelId, i32)>,
  pub uses: Vec<LabelId>,
}
// pub trait Uses {
//   fn id(&self) -> LabelId;
//   fn uses(&self) -> &[LabelId];
// }
// impl Uses for CompiledFunc {
//   fn id(&self) -> LabelId {
//     self.id
//   }
//   fn uses(&self) -> &[LabelId] {
//     &self.uses
//   }
// }
// impl Uses for UserDefinedInfo {
//   fn id(&self) -> LabelId {
//     self.id
//   }
//   fn uses(&self) -> &[LabelId] {
//     &self.uses
//   }
// }
// // impl Uses for Variable {
// // }
impl Default for Address {
  fn default() -> Self {
    Local(Tmp, 0)
  }
}
impl Default for MemoryType {
  fn default() -> Self {
    Size(8)
  }
}
impl<T> Default for Bind<T> {
  fn default() -> Self {
    Var(Memory::default())
  }
}
impl BuiltIn {
  pub(crate) fn arg(&mut self) -> ErrOR<WithPos<Json>> {
    self.nth += 1;
    self.args.next().ok_or_else(|| Internal(ArgNotFound(self.name.clone(), self.nth)))
  }
  pub(crate) fn push_free_tmp(&mut self, memory: Memory) {
    if let Memory(Local(Tmp, offset), size) = memory {
      self.free_vec.push((offset, size));
    }
  }
}
impl<T> WithPos<T> {
  pub(crate) fn map<F: Fn(T) -> V, V>(self, map_f: F) -> WithPos<V> {
    self.pos.with(map_f(self.val))
  }
  pub(crate) fn map_ref<F: Fn(&T) -> V, V>(&self, map_f: F) -> WithPos<V> {
    self.pos.with(map_f(&self.val))
  }
}
impl MemoryType {
  pub(crate) fn size(self) -> i32 {
    match self {
      Heap(_) => 8,
      Size(size) => size,
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
