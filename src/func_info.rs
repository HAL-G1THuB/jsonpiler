//! Implementation for `FuncInfo`.
use {
  super::{Align, FuncInfo},
  core::cmp,
};
impl FuncInfo {
  /// Calculate to allocate size.
  pub fn calc_alloc(&self, align: usize) -> Result<usize, &'static str> {
    let args_size = self.args_slots.checked_mul(8).ok_or("Overflow: args_slots * 8")?;
    let raw = self.stack_size.checked_add(args_size).ok_or("Overflow: stack_size + args")?;
    let locals = raw.checked_add(15).ok_or("Overflow before alignment")? & !15;
    let shadow_space = align.checked_add(32).ok_or("ShadowSpace Overflow")?;
    locals.checked_add(shadow_space).ok_or("LocalSize Overflow")
  }
  /// Free from stack.
  pub fn free(&mut self, end: usize, mut size: usize) -> Result<(), &'static str> {
    let mut start = end.checked_sub(size).ok_or("StackCalc Overflow")?;
    if let Some((&prev_start, &prev_size)) = self.free_map.range(..start).next_back() {
      if prev_start.checked_add(prev_size).ok_or("StackCalc Overflow")? == start {
        self.free_map.remove(&prev_start);
        start = prev_start;
        size = size.checked_add(prev_size).ok_or("StackCalc Overflow")?;
      }
    }
    if let Some((&next_start, &next_size)) = self.free_map.range(start..).next() {
      if end == next_start {
        self.free_map.remove(&next_start);
        size = size.checked_add(next_size).ok_or("StackCalc Overflow")?;
      }
    }
    self.free_map.insert(start, size);
    Ok(())
  }
  /// get local variable name.
  pub fn get_local(&mut self, align: Align) -> Result<usize, &'static str> {
    self.push(align)?.checked_add(self.scope_align).ok_or("StackSize Overflow")
  }
  /// Push to stack.
  #[expect(clippy::as_conversions, reason = "Align enum safe conversion")]
  pub fn push(&mut self, ty: Align) -> Result<usize, &'static str> {
    let add = |op1: usize, op2: usize| -> Result<usize, &'static str> {
      op1.checked_add(op2).ok_or("InternalError: Error in FuncInfo::push")
    };
    let sub = |op1: usize, op2: usize| -> Result<usize, &'static str> {
      op1.checked_sub(op2).ok_or("InternalError: Error in FuncInfo::push")
    };
    let align = ty as usize;
    let dec_align = sub(align, 1)?;
    for (&start, &size) in &self.free_map {
      let aligned_start = add(start, dec_align)? & !dec_align;
      let padding = sub(aligned_start, start)?;
      if size >= add(padding, align)? {
        self.free_map.remove(&start);
        if padding > 0 {
          self.free_map.insert(start, padding);
        }
        let used_end = add(aligned_start, align)?;
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
    let new_end = add(aligned_start, align)?;
    self.stack_size = new_end;
    Ok(new_end)
  }
  /// Update `args_slots` (only if size is larger)
  #[expect(dead_code, reason = "todo")]
  pub fn update_max(&mut self, size: usize) {
    self.args_slots = cmp::max(self.args_slots, size);
  }
}
