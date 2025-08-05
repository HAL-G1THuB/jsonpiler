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
impl ScopeInfo {
  pub(crate) fn begin(&mut self) -> ErrOR<ScopeInfo> {
    let prev_align = self.scope_align;
    self.scope_align = add!(self.scope_align, align_up(self.stack_size, 16)?)?;
    Ok(ScopeInfo {
      body: take(&mut self.body),
      free_map: take(&mut self.free_map),
      stack_size: take(&mut self.stack_size),
      scope_align: prev_align,
      ..ScopeInfo::default()
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
    if let Some(Label { kind: Tmp, id, size }) = json.get_label() {
      return self.free(id, size);
    }
    Ok(())
  }
  pub(crate) fn end(&mut self, tmp: ScopeInfo) -> ErrOR<()> {
    let align = align_up(self.stack_size, 16)?;
    let mut scope_body = replace(&mut self.body, tmp.body);
    if align != 0 {
      self.body.push(mn!("sub", "rsp", format!("{align:#x}")));
    }
    self.body.append(&mut scope_body);
    if align != 0 {
      self.body.push(mn!("add", "rsp", format!("{align:#x}")));
    }
    self.stack_size = tmp.stack_size;
    self.scope_align = tmp.scope_align;
    self.free_map = tmp.free_map;
    Ok(())
  }
  fn free(&mut self, abs_end: usize, mut size: usize) -> ErrOR<()> {
    let end = sub!(abs_end, self.scope_align)?;
    let mut start = sub!(end, size)?;
    if let Some((&prev_start, &prev_size)) = self.free_map.range(..start).next_back() {
      if add!(prev_start, prev_size)? == start {
        self.free_map.remove(&prev_start);
        start = prev_start;
        size = add!(size, prev_size)?;
      }
    }
    if let Some((&next_start, &next_size)) = self.free_map.range(start..).next() {
      if end == next_start {
        self.free_map.remove(&next_start);
        size = add!(size, next_size)?;
      }
    }
    self.free_map.insert(start, size);
    Ok(())
  }
  pub fn free_if_tmp(&mut self, bind: &Label) -> ErrOR<()> {
    if bind.kind == Tmp { self.free(bind.id, bind.size) } else { Ok(()) }
  }
  pub fn get_local(&mut self, size: usize) -> ErrOR<Label> {
    Ok(Label { kind: Local, id: add!(self.push(size)?, self.scope_align)?, size })
  }
  pub fn get_tmp(&mut self, size: usize) -> ErrOR<Label> {
    Ok(Label { kind: Tmp, id: add!(self.push(size)?, self.scope_align)?, size })
  }
  pub fn mov_tmp(&mut self, reg: &str) -> ErrOR<Label> {
    let return_value = self.get_tmp(8)?;
    self.body.push(mn!("mov", return_value, reg));
    Ok(return_value)
  }
  pub fn mov_tmp_bool(&mut self, reg: &str) -> ErrOR<Json> {
    let return_value = self.get_tmp(1)?;
    self.body.push(mn!("mov", return_value, reg));
    Ok(Json::Bool(Var(return_value)))
  }
  fn push(&mut self, size: usize) -> ErrOR<usize> {
    for (&start, &size2) in &self.free_map {
      let aligned_start = align_up(start, size)?;
      let padding = sub!(aligned_start, start)?;
      if size2 >= add!(padding, size)? {
        self.free_map.remove(&start);
        if padding > 0 {
          self.free_map.insert(start, padding);
        }
        let used_end = add!(aligned_start, size)?;
        let tail_size = sub!(add!(start, size2)?, used_end)?;
        if tail_size > 0 {
          self.free_map.insert(used_end, tail_size);
        }
        return Ok(used_end);
      }
    }
    let aligned_start = align_up(self.stack_size, size)?;
    if aligned_start > self.stack_size {
      let gap_size = sub!(aligned_start, self.stack_size)?;
      self.free_map.insert(self.stack_size, gap_size);
    }
    let new_end = add!(aligned_start, size)?;
    self.stack_size = new_end;
    Ok(new_end)
  }
  #[expect(dead_code, reason = "todo")]
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
