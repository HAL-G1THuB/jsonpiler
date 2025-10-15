use crate::{
  Bind::Var,
  ErrOR,
  Inst::{self, *},
  InternalErrKind::*,
  Json,
  JsonpilerErr::*,
  Label,
  Memory::{Local, Tmp},
  Register,
  utility::{align_up_i32, mov_b, mov_q},
};
use core::iter::{DoubleEndedIterator as _, Iterator as _};
use core::mem::{replace, take};
use std::collections::{BTreeMap, HashMap};
use std::vec::IntoIter;
macro_rules! add {
  ($op1:expr, $op2:expr) => {
    $op1.checked_add($op2).ok_or(InternalError(Overflow))
  };
}
macro_rules! sub {
  ($op1:expr, $op2:expr) => {{ $op1.checked_sub($op2).ok_or(InternalError(Underflow)) }};
}
pub(crate) struct ScopeInfo {
  alloc_map: BTreeMap<i32, i32>,
  body: Vec<Inst>,
  epilogue: Option<(u32, Json)>,
  locals: Vec<HashMap<String, Json>>,
  loop_labels: Vec<(u32, u32)>,
  stack_args: i32,
  stack_size: i32,
}
impl ScopeInfo {
  pub(crate) fn alloc(&mut self, size: i32, align: i32) -> ErrOR<i32> {
    for (&start, &size2) in &self.alloc_map {
      let aligned_start = align_up_i32(start, align)?;
      let padding = sub!(aligned_start, start)?;
      if size2 >= add!(padding, size)? {
        self.alloc_map.remove(&start);
        if padding > 0i32 {
          self.alloc_map.insert(start, padding);
        }
        let used_end = add!(aligned_start, size)?;
        let tail_size = sub!(add!(start, size2)?, used_end)?;
        if tail_size > 0i32 {
          self.alloc_map.insert(used_end, tail_size);
        }
        return Ok(used_end);
      }
    }
    let aligned_start = align_up_i32(self.stack_size, align)?;
    if aligned_start > self.stack_size {
      let gap_size = sub!(aligned_start, self.stack_size)?;
      self.alloc_map.insert(self.stack_size, gap_size);
    }
    let new_end = add!(aligned_start, size)?;
    self.stack_size = new_end;
    Ok(new_end)
  }
  pub(crate) fn begin(&mut self) {
    self.locals.push(HashMap::new());
  }
  #[expect(clippy::needless_pass_by_value)]
  pub(crate) fn drop_json(&mut self, json: Json) -> ErrOR<()> {
    if let Some(Label { mem: Tmp { offset }, size }) = json.get_label() {
      return self.free(offset, size);
    }
    Ok(())
  }
  pub(crate) fn end(&mut self) -> ErrOR<()> {
    if let Some(locals) = self.locals.pop() {
      #[expect(clippy::iter_over_hash_type)]
      for (_, json) in locals {
        if let Some(Label { mem: Tmp { offset } | Local { offset }, size }) = json.get_label() {
          self.free(offset, size)?;
        }
      }
    }
    Ok(())
  }
  pub(crate) fn extend(&mut self, insts: &[Inst]) {
    self.body.extend_from_slice(insts);
  }
  pub(crate) fn free(&mut self, end: i32, mut size: i32) -> ErrOR<()> {
    let mut start = sub!(end, size)?;
    if let Some((&prev_start, &prev_size)) = self.alloc_map.range(..start).next_back()
      && add!(prev_start, prev_size)? == start
    {
      self.alloc_map.remove(&prev_start);
      start = prev_start;
      size = add!(size, prev_size)?;
    }
    if let Some((&next_start, &next_size)) = self.alloc_map.range(start..).next()
      && end == next_start
    {
      self.alloc_map.remove(&next_start);
      size = add!(size, next_size)?;
    }
    self.alloc_map.insert(start, size);
    Ok(())
  }
  pub(crate) fn get_epilogue(&self) -> Option<&(u32, Json)> {
    self.epilogue.as_ref()
  }
  pub(crate) fn get_var_local(&self, var_name: &str) -> Option<Json> {
    for table in &self.locals {
      if let Some(val) = table.get(var_name) {
        return Some(val.clone());
      }
    }
    None
  }
  pub(crate) fn innermost_scope(&mut self) -> ErrOR<&mut HashMap<String, Json>> {
    self.locals.last_mut().ok_or(InternalError(InvalidScope))
  }
  pub(crate) fn into_iter_code(self) -> IntoIter<Inst> {
    self.body.into_iter()
  }
  pub(crate) fn local(&mut self, size: i32, align: i32) -> ErrOR<Label> {
    Ok(Label { mem: Local { offset: self.alloc(size, align)? }, size })
  }
  pub(crate) fn loop_enter(&mut self, start_label: u32, end_label: u32) {
    self.loop_labels.push((start_label, end_label));
  }
  pub(crate) fn loop_exit(&mut self) {
    self.loop_labels.pop();
  }
  pub(crate) fn loop_label(&mut self) -> Option<&(u32, u32)> {
    self.loop_labels.last()
  }
  pub(crate) fn mov_tmp(&mut self, reg: Register) -> ErrOR<Label> {
    let return_value = self.tmp(8, 8)?;
    self.body.push(mov_q(return_value.mem, reg));
    Ok(return_value)
  }
  pub(crate) fn mov_tmp_bool(&mut self, reg: Register) -> ErrOR<Json> {
    let return_value = self.tmp(1, 8)?;
    self.body.push(mov_b(return_value.mem, reg));
    Ok(Json::Bool(Var(return_value)))
  }
  pub(crate) fn mov_tmp_xmm(&mut self, reg: Register) -> ErrOR<Json> {
    let return_value = self.tmp(8, 8)?;
    self.body.push(MovSdMX(return_value.mem, reg));
    Ok(Json::Float(Var(return_value)))
  }
  pub(crate) fn new() -> Self {
    Self {
      epilogue: None,
      alloc_map: BTreeMap::new(),
      loop_labels: vec![],
      stack_args: 0,
      body: vec![],
      locals: vec![HashMap::new()],
      stack_size: 0,
    }
  }
  pub(crate) fn push(&mut self, inst: Inst) {
    self.body.push(inst);
  }
  pub(crate) fn replace_locals(
    &mut self, locals: Vec<HashMap<String, Json>>,
  ) -> Vec<HashMap<String, Json>> {
    replace(&mut self.locals, locals)
  }
  pub(crate) fn resolve_stack_size(&self) -> ErrOR<u32> {
    const SHADOW_SPACE: u32 = 32;
    // let ret_addr = 8; let rbp = 8; (ret_addr + rbp) % 16 == 0
    let args_size = self.stack_args.checked_mul(8).ok_or(InternalError(Overflow))?;
    let raw = add!(self.stack_size, args_size)?;
    let locals = align_up_i32(raw, 16)?;
    let aligned = locals;
    Ok(add!(u32::try_from(aligned)?, SHADOW_SPACE)?)
  }
  pub(crate) fn set_epilogue(&mut self, epilogue: Option<(u32, Json)>) {
    self.epilogue = epilogue;
  }
  pub(crate) fn take_code(&mut self) -> Vec<Inst> {
    take(&mut self.body)
  }
  pub(crate) fn tmp(&mut self, size: i32, align: i32) -> ErrOR<Label> {
    Ok(Label { mem: Tmp { offset: self.alloc(size, align)? }, size })
  }
  pub(crate) fn update_stack_args(&mut self, size: i32) {
    self.stack_args = self.stack_args.max(size);
  }
}
