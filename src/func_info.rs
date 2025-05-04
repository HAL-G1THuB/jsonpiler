//! Implementation for `FuncInfo`.
use crate::{
  ErrOR, FuncInfo, Name,
  VarKind::{Local, Tmp},
  add,
};
use core::cmp;
impl FuncInfo {
  /// Calculate to allocate size.
  pub fn calc_alloc(&self, align: usize) -> ErrOR<usize> {
    let args_size = self.args_slots.checked_mul(8).ok_or("Overflow: args_slots * 8")?;
    let raw = add(self.stack_size, args_size)?;
    let locals = add(raw, 15)? & !15;
    let shadow_space = add(align, 32)?;
    add(locals, shadow_space)
  }
  /// Free from stack.
  pub fn free(&mut self, end: usize, mut size: usize) -> ErrOR<()> {
    let mut start = end.checked_sub(size).ok_or("StackCalc Overflow")?;
    if let Some((&prev_start, &prev_size)) = self.free_map.range(..start).next_back() {
      if add(prev_start, prev_size)? == start {
        self.free_map.remove(&prev_start);
        start = prev_start;
        size = add(size, prev_size)?;
      }
    }
    if let Some((&next_start, &next_size)) = self.free_map.range(start..).next() {
      if end == next_start {
        self.free_map.remove(&next_start);
        size = add(size, next_size)?;
      }
    }
    self.free_map.insert(start, size);
    Ok(())
  }
  /// get local variable name.
  pub fn get_local(&mut self, byte: usize) -> ErrOR<Name> {
    Ok(Name { var: Local, seed: add(self.push(byte)?, self.scope_align)? })
  }
  /// get temporary variable name.
  pub fn get_tmp(&mut self, byte: usize) -> ErrOR<Name> {
    Ok(Name { var: Tmp, seed: add(self.push(byte)?, self.scope_align)? })
  }
  /// Push to stack.
  fn push(&mut self, byte: usize) -> ErrOR<usize> {
    let sub = |op1: usize, op2: usize| -> Result<usize, &'static str> {
      op1.checked_sub(op2).ok_or("InternalError: Error in FuncInfo::push")
    };
    let dec_align = sub(byte, 1)?;
    for (&start, &size) in &self.free_map {
      let aligned_start = add(start, dec_align)? & !dec_align;
      let padding = sub(aligned_start, start)?;
      if size >= add(padding, byte)? {
        self.free_map.remove(&start);
        if padding > 0 {
          self.free_map.insert(start, padding);
        }
        let used_end = add(aligned_start, byte)?;
        let tail_size = sub(add(start, size)?, used_end)?;
        if tail_size > 0 {
          self.free_map.insert(used_end, tail_size);
        }
        return Ok(used_end);
      }
    }
    let aligned_start = add(self.stack_size, dec_align)? & !dec_align;
    if aligned_start > self.stack_size {
      let gap_size = sub(aligned_start, self.stack_size)?;
      self.free_map.insert(self.stack_size, gap_size);
    }
    let new_end = add(aligned_start, byte)?;
    self.stack_size = new_end;
    Ok(new_end)
  }
  /// Update `args_slots` (only if size is larger)
  #[expect(dead_code, reason = "todo")]
  pub fn update_max(&mut self, size: usize) {
    self.args_slots = cmp::max(self.args_slots, size);
  }
}
