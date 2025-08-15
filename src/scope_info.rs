use crate::{
  Bind::Var,
  ErrOR,
  Inst::{self, *},
  Json, Label,
  OpQ::{Mq, Rq},
  Reg::{self, *},
  VarKind::{Local, Tmp},
  utility::align_up_32,
};
use core::iter::{DoubleEndedIterator as _, Iterator, once};
use core::mem::{replace, take};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::vec::IntoIter;
macro_rules! add {
  ($op1:expr, $op2:expr) => {
    $op1.checked_add($op2).ok_or("InternalError: Overflow occurred")
  };
}
macro_rules! sub {
  ($op1:expr, $op2:expr) => {
    $op1.checked_sub($op2).ok_or("InternalError: Underflow occurred")
  };
}
pub(crate) struct ScopeInfo {
  alloc_map: BTreeMap<u32, u32>,
  args_slots: u32,
  base_stack: u32,
  body: Vec<Inst>,
  locals: Vec<HashMap<String, Json>>,
  reg_used: BTreeSet<Reg>,
  stack_size: u32,
}
impl ScopeInfo {
  fn alloc(&mut self, size: u32) -> ErrOR<u32> {
    for (&start, &size2) in &self.alloc_map {
      let aligned_start = align_up_32(start, size)?;
      let padding = sub!(aligned_start, start)?;
      if size2 >= add!(padding, size)? {
        self.alloc_map.remove(&start);
        if padding > 0 {
          self.alloc_map.insert(start, padding);
        }
        let used_end = add!(aligned_start, size)?;
        let tail_size = sub!(add!(start, size2)?, used_end)?;
        if tail_size > 0 {
          self.alloc_map.insert(used_end, tail_size);
        }
        return Ok(used_end);
      }
    }
    let aligned_start = align_up_32(self.stack_size, size)?;
    if aligned_start > self.stack_size {
      let gap_size = sub!(aligned_start, self.stack_size)?;
      self.alloc_map.insert(self.stack_size, gap_size);
    }
    let new_end = add!(aligned_start, size)?;
    self.stack_size = new_end;
    Ok(new_end)
  }
  pub(crate) fn begin(&mut self) -> ErrOR<ScopeInfo> {
    let prev_align = self.base_stack;
    self.base_stack = add!(self.base_stack, align_up_32(self.stack_size, 16)?)?;
    self.locals.push(HashMap::new());
    Ok(ScopeInfo {
      body: take(&mut self.body),
      alloc_map: take(&mut self.alloc_map),
      stack_size: take(&mut self.stack_size),
      base_stack: prev_align,
      ..ScopeInfo::new()
    })
  }
  pub(crate) fn drop_json(&mut self, json: Json) -> ErrOR<()> {
    if let Some(Label { kind: Tmp { offset }, size }) = json.get_label() {
      return self.free(offset, size);
    }
    Ok(())
  }
  pub(crate) fn end(&mut self, tmp: ScopeInfo) -> ErrOR<()> {
    let align = align_up_32(self.stack_size, 16)?;
    let mut scope_body = replace(&mut self.body, tmp.body);
    if align != 0 {
      self.body.push(SubRId(Rsp, align));
    }
    self.body.append(&mut scope_body);
    if align != 0 {
      self.body.push(AddRId(Rsp, align));
    }
    self.stack_size = tmp.stack_size;
    self.base_stack = tmp.base_stack;
    self.alloc_map = tmp.alloc_map;
    self.locals.pop();
    Ok(())
  }
  pub(crate) fn extend(&mut self, insts: &[Inst]) {
    self.body.extend_from_slice(insts);
  }
  pub(crate) fn free(&mut self, abs_end: u32, mut size: u32) -> ErrOR<()> {
    let end = sub!(abs_end, self.base_stack)?;
    let mut start = sub!(end, size)?;
    if let Some((&prev_start, &prev_size)) = self.alloc_map.range(..start).next_back() {
      if add!(prev_start, prev_size)? == start {
        self.alloc_map.remove(&prev_start);
        start = prev_start;
        size = add!(size, prev_size)?;
      }
    }
    if let Some((&next_start, &next_size)) = self.alloc_map.range(start..).next() {
      if end == next_start {
        self.alloc_map.remove(&next_start);
        size = add!(size, next_size)?;
      }
    }
    self.alloc_map.insert(start, size);
    Ok(())
  }
  pub(crate) fn innermost_scope(&mut self) -> ErrOR<&mut HashMap<String, Json>> {
    Ok(self.locals.last_mut().ok_or("InternalError: Invalid scope.")?)
  }
  pub(crate) fn into_iter_code(self) -> IntoIter<Inst> {
    self.body.into_iter()
  }
  pub(crate) fn iter_all_scope<'a>(
    &'a self, globals: &'a HashMap<String, Json>,
  ) -> impl Iterator<Item = &'a HashMap<String, Json>> {
    self.locals.iter().rev().chain(once(globals))
  }
  pub(crate) fn local(&mut self, size: u32) -> ErrOR<Label> {
    Ok(Label { kind: Local { offset: add!(self.alloc(size)?, self.base_stack)? }, size })
  }
  pub(crate) fn mov_tmp(&mut self, reg: Reg) -> ErrOR<Label> {
    let return_value = self.tmp(8)?;
    self.body.push(MovQQ(Mq(return_value.kind), Rq(reg)));
    Ok(return_value)
  }
  pub(crate) fn mov_tmp_bool(&mut self, reg: Reg) -> ErrOR<Json> {
    let return_value = self.tmp(1)?;
    self.body.push(MovMbRb(return_value.kind, reg));
    Ok(Json::Bool(Var(return_value)))
  }
  pub(crate) fn new() -> Self {
    Self {
      alloc_map: BTreeMap::new(),
      args_slots: 0,
      body: vec![],
      locals: vec![HashMap::new()],
      reg_used: BTreeSet::new(),
      base_stack: 0,
      stack_size: 0,
    }
  }
  pub(crate) fn push(&mut self, inst: Inst) {
    self.body.push(inst);
  }
  pub(crate) fn reg_align(&mut self) -> ErrOR<u32> {
    Ok(u32::try_from((self.reg_used.len() & 1) << 3)?)
  }
  pub(crate) fn resolve_stack_size(&self, align: u32) -> ErrOR<u32> {
    let args_size = self.args_slots.checked_mul(8).ok_or("Overflow: args_slots * 8")?;
    let raw = add!(self.stack_size, args_size)?;
    let locals = align_up_32(raw, 16)?;
    let aligned = add!(locals, align)?;
    let shadow_space = 32;
    Ok(add!(aligned, shadow_space)?)
  }
  pub(crate) fn take_code(&mut self) -> Vec<Inst> {
    take(&mut self.body)
  }
  pub(crate) fn take_regs(&mut self) -> BTreeSet<Reg> {
    take(&mut self.reg_used)
  }
  pub(crate) fn tmp(&mut self, size: u32) -> ErrOR<Label> {
    Ok(Label { kind: Tmp { offset: add!(self.alloc(size)?, self.base_stack)? }, size })
  }
  #[expect(dead_code)]
  pub(crate) fn update_max(&mut self, size: u32) {
    self.args_slots = self.args_slots.max(size);
  }
  pub(crate) fn use_reg(&mut self, reg: Reg) {
    self.reg_used.insert(reg);
  }
}
