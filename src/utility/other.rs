use crate::prelude::*;
use std::{ops::Add, vec::IntoIter};
pub(crate) type BuiltInPtr = fn(&mut Jsonpiler, &mut Pos<BuiltIn>, &mut Scope) -> ErrOR<Json>;
pub(crate) type Dll = (String, Vec<String>);
pub(crate) type FileIdx = u32;
pub(crate) type LabelId = u32;
pub(crate) type Seh = Vec<(LabelId, LabelId, i32)>;
#[derive(Debug, Clone)]
pub(crate) struct BuiltIn {
  pub args: IntoIter<Pos<Json>>,
  pub free_list: BTreeSet<Memory>,
  pub len: u32,
  pub name: String,
  pub nth: u32,
}
#[derive(Debug, Clone)]
pub(crate) struct UserDefinedInfo {
  pub dep: Dependency,
  pub refs: Vec<Position>,
  pub sig: Signature,
}
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct Signature {
  pub params: Vec<(String, JsonType)>,
  pub ret_type: JsonType,
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct BuiltInInfo {
  pub arity: Arity,
  pub builtin_ptr: BuiltInPtr,
  pub scoped: bool,
  pub skip_eval: bool,
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
