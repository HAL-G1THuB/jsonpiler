//! Implementation for `ScopeInfo`.
use crate::{
  AsmBool, ErrOR, Name, ScopeInfo,
  VarKind::{Local, Tmp},
  add,
};
use core::cmp;
impl ScopeInfo {
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
    let mut start = end
      .checked_sub(size)
      .ok_or("InternalError: `free` failed")?
      .checked_sub(self.scope_align)
      .ok_or("InternalError: `free` failed")?;
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
  /// Free a specific bit from bool map.
  #[expect(dead_code, reason = "todo")]
  pub fn free_bool(&mut self, asm_bool: &AsmBool) -> ErrOR<()> {
    let seed = asm_bool.name.seed;
    let bit = asm_bool.bit;
    if let Some(bits) = self.bool_map.get_mut(&seed) {
      *bits &= !(1 << bit);
      if *bits == 0 {
        self.bool_map.remove(&seed);
        self.free(seed, 1)
      } else {
        Ok(())
      }
    } else {
      Err("InternalError: Address not found in bool_map.".into())
    }
  }
  /// get temporary variable name.
  #[expect(dead_code, reason = "todo")]
  pub fn get_bool_local(&mut self) -> ErrOR<AsmBool> {
    let (end, bit) = self.push_bool()?;
    Ok(AsmBool { name: Name { var: Local, seed: add(end, self.scope_align)? }, bit })
  }
  /// get temporary variable name.
  #[expect(dead_code, reason = "todo")]
  pub fn get_bool_tmp(&mut self) -> ErrOR<AsmBool> {
    let (end, bit) = self.push_bool()?;
    Ok(AsmBool { name: Name { var: Tmp, seed: add(end, self.scope_align)? }, bit })
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
  /// Push bool to stack, using the next available bit.
  fn push_bool(&mut self) -> ErrOR<(usize, u8)> {
    for (&addr, bits) in &mut self.bool_map {
      for i in 0..8 {
        if *bits & (1 << i) == 0 {
          *bits |= 1 << i;
          return Ok((addr, i));
        }
      }
    }
    let addr = self.push(1)?;
    self.bool_map.insert(addr, 0b0000_0001);
    Ok((addr, 0))
  }
  /// Update `args_slots` (only if size is larger)
  #[expect(dead_code, reason = "todo")]
  pub fn update_max(&mut self, size: usize) {
    self.args_slots = cmp::max(self.args_slots, size);
  }
}
