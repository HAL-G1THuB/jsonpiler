use crate::prelude::*;
use std::vec::IntoIter;
pub(crate) type BuiltInPtr = fn(&mut Jsonpiler, &mut Pos<BuiltIn>, &mut Scope) -> ErrOR<Json>;
pub(crate) type FileIdx = usize;
pub(crate) type LabelId = u32;
pub(crate) type Seh = Vec<(LabelId, LabelId, i32)>;
#[derive(Debug, Clone)]
pub(crate) struct JsonpilerFlags {
  pub a64: bool,
  pub build_only: bool,
  pub debug: bool,
}
#[derive(Debug, Clone)]
pub(crate) enum AorX {
  A64(Vec<Vec<A64Inst>>),
  X64(Vec<Vec<X64Inst>>),
}
impl Default for AorX {
  fn default() -> Self {
    X64(vec![])
  }
}
impl AorX {
  pub(crate) fn a64(self) -> ErrOR<Vec<Vec<A64Inst>>> {
    if let A64(insts) = self { Ok(insts) } else { Err(MismatchInstGen.into()) }
  }
  pub(crate) fn a64_mut(&mut self) -> ErrOR<&mut Vec<Vec<A64Inst>>> {
    if let A64(insts) = self { Ok(insts) } else { Err(MismatchInstGen.into()) }
  }
  pub(crate) fn is_a64(&self) -> bool {
    match self {
      A64(_) => true,
      X64(_) => false,
    }
  }
  pub(crate) fn new(a64: bool) -> Self {
    if a64 { A64(vec![]) } else { X64(vec![]) }
  }
  pub(crate) fn x64(self) -> ErrOR<Vec<Vec<X64Inst>>> {
    if let X64(insts) = self { Ok(insts) } else { Err(MismatchInstGen.into()) }
  }
  pub(crate) fn x64_mut(&mut self) -> ErrOR<&mut Vec<Vec<X64Inst>>> {
    if let X64(insts) = self { Ok(insts) } else { Err(MismatchInstGen.into()) }
  }
}
#[derive(Debug, Clone)]
pub(crate) struct BuiltIn {
  pub args: IntoIter<Pos<Json>>,
  pub len: u32,
  pub name: String,
  pub nth: u32,
  pub tmp_list: BTreeSet<Memory>,
}
#[derive(Debug, Clone)]
pub(crate) struct UserDefinedInfo {
  pub dep: Dependency,
  pub refs: Vec<Position>,
  pub sig: Signature,
}
impl UserDefinedInfo {
  pub(crate) fn new(id: LabelId, params: Vec<(String, JsonType)>, ret_type: JsonType) -> Self {
    Self { dep: Dependency::new(id), refs: vec![], sig: Signature { params, ret_type } }
  }
}
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct Signature {
  pub params: Vec<(String, JsonType)>,
  pub ret_type: JsonType,
}
#[derive(Debug, Clone)]
pub(crate) struct BuiltInInfo {
  pub arity: Arity,
  pub builtin_a64: Option<BuiltInPtr>,
  pub builtin_x64: BuiltInPtr,
  pub refs: Vec<Position>,
  pub scoped: bool,
  pub skip_eval: bool,
}
impl Pos<BuiltIn> {
  pub(crate) fn arg(&mut self) -> ErrOR<Pos<Json>> {
    self.val.nth += 1;
    self.val.args.next().ok_or_else(|| ArgNotFound(self.val.name.clone(), self.val.nth).into())
  }
  pub(crate) fn push_free_tmp(&mut self, mem_opt: Option<Memory>) {
    if let Some(mem @ Memory(Local(Tmp, _), _)) = mem_opt {
      self.val.tmp_list.insert(mem);
    }
  }
}
