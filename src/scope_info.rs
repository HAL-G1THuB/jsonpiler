use crate::{
  Bind::Var,
  ErrOR, Json, Label, ScopeInfo,
  VarKind::{Local, Tmp},
  add, mn, sub,
};
use core::{
  cmp,
  mem::{replace, take},
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
impl ScopeInfo {
  pub(crate) fn begin(&mut self) -> ErrOR<ScopeInfo> {
    let prev_align = self.scope_align;
    self.scope_align = add!(self.scope_align, align_up(self.stack_size, 16)?)?;
    self.locals.push(HashMap::new());
    Ok(ScopeInfo {
      body: take(&mut self.body),
      alloc_map: take(&mut self.alloc_map),
      stack_size: take(&mut self.stack_size),
      scope_align: prev_align,
      ..ScopeInfo::new()
    })
  }
  pub fn calc_alloc(&self, align: usize) -> ErrOR<usize> {
    let args_size = self.args_slots.checked_mul(8).ok_or("Overflow: args_slots * 8")?;
    let raw = add!(self.stack_size, args_size)?;
    let locals = align_up(raw, 16)?;
    let aligned = add!(locals, align)?;
    let shadow_space = 32;
    Ok(add!(aligned, shadow_space)?)
  }
  pub fn drop_json(&mut self, json: Json) -> ErrOR<()> {
    if let Some(label) = json.get_label() {
      return self.free(label);
    }
    Ok(())
  }
  pub(crate) fn end(&mut self, tmp: ScopeInfo) -> ErrOR<()> {
    let align = align_up(self.stack_size, 16)?;
    let mut scope_body = replace(&mut self.body, tmp.body);
    if align != 0 {
      self.body.push(mn!("sub", "rsp", format!("{align:#X}")));
    }
    self.body.append(&mut scope_body);
    if align != 0 {
      self.body.push(mn!("add", "rsp", format!("{align:#X}")));
    }
    self.stack_size = tmp.stack_size;
    self.scope_align = tmp.scope_align;
    self.alloc_map = tmp.alloc_map;
    self.locals.pop();
    Ok(())
  }
  pub(crate) fn free(&mut self, mut label: Label) -> ErrOR<()> {
    let end = sub!(label.id, self.scope_align)?;
    let mut start = sub!(end, label.size)?;
    if let Some((&prev_start, &prev_size)) = self.alloc_map.range(..start).next_back() {
      if add!(prev_start, prev_size)? == start {
        self.alloc_map.remove(&prev_start);
        start = prev_start;
        label.size = add!(label.size, prev_size)?;
      }
    }
    if let Some((&next_start, &next_size)) = self.alloc_map.range(start..).next() {
      if end == next_start {
        self.alloc_map.remove(&next_start);
        label.size = add!(label.size, next_size)?;
      }
    }
    self.alloc_map.insert(start, label.size);
    Ok(())
  }
  pub fn local(&mut self, size: usize) -> ErrOR<Label> {
    Ok(Label { kind: Local, id: add!(self.push(size)?, self.scope_align)?, size })
  }
  pub fn mov_tmp(&mut self, reg: &str) -> ErrOR<Label> {
    let return_value = self.tmp(8)?;
    self.body.push(mn!("mov", return_value, reg));
    Ok(return_value)
  }
  pub fn mov_tmp_bool(&mut self, reg: &str) -> ErrOR<Json> {
    let return_value = self.tmp(1)?;
    self.body.push(mn!("mov", return_value, reg));
    Ok(Json::Bool(Var(return_value)))
  }
  pub(crate) fn new() -> Self {
    Self {
      alloc_map: BTreeMap::new(),
      args_slots: 0,
      body: vec![],
      locals: vec![HashMap::new()],
      reg_used: BTreeSet::new(),
      scope_align: 0,
      stack_size: 0,
    }
  }
  fn push(&mut self, size: usize) -> ErrOR<usize> {
    for (&start, &size2) in &self.alloc_map {
      let aligned_start = align_up(start, size)?;
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
    let aligned_start = align_up(self.stack_size, size)?;
    if aligned_start > self.stack_size {
      let gap_size = sub!(aligned_start, self.stack_size)?;
      self.alloc_map.insert(self.stack_size, gap_size);
    }
    let new_end = add!(aligned_start, size)?;
    self.stack_size = new_end;
    Ok(new_end)
  }
  pub fn tmp(&mut self, size: usize) -> ErrOR<Label> {
    Ok(Label { kind: Tmp, id: add!(self.push(size)?, self.scope_align)?, size })
  }
  #[expect(dead_code)]
  pub fn update_max(&mut self, size: usize) {
    self.args_slots = cmp::max(self.args_slots, size);
  }
  pub fn use_reg(&mut self, reg: &str) {
    self.reg_used.insert(reg.to_owned());
  }
}
fn align_up(num: usize, align: usize) -> ErrOR<usize> {
  let dec_align = sub!(align, 1)?;
  Ok(add!(num, dec_align)? & !dec_align)
}
