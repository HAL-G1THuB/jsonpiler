//! Implementation for `FuncInfo`.
use {
  super::{Align, FuncInfo},
  core::{cmp, iter},
};
impl FuncInfo {
  /// Calculate to allocate size.
  pub fn calc_alloc(&self, align: usize) -> Result<usize, &'static str> {
    let args_size = self.args_slots.checked_mul(8).ok_or("Overflow: args_slots * 8")?;
    let raw = self.layout.len().checked_add(args_size).ok_or("Overflow: layout + args")?;
    let locals = raw.checked_add(15).ok_or("Overflow before alignment")? & !15;
    let shadow_space = align.checked_add(32).ok_or("ShadowSpace Overflow")?;
    locals.checked_add(shadow_space).ok_or("LocalSize Overflow")
  }
  /// get local variable name.
  pub fn get_local(&mut self, align: Align) -> Result<String, &'static str> {
    Ok(format!(
      "qword ptr -{}[rbp]",
      self.push(align)?.checked_add(self.scope_align).ok_or("StackCalc Overflow")?
    ))
  }
  /// Push to stack.
  #[expect(clippy::as_conversions, reason = "")]
  fn push(&mut self, ty: Align) -> Result<usize, &'static str> {
    let align = ty as usize;
    if ty == Align::U8 {
      let mut free_count: usize = 0;
      for (i, flag) in self.layout.iter().enumerate() {
        if *flag {
          free_count = 0;
        } else {
          // Saturating addition to prevent overflow
          free_count = free_count.checked_add(1).ok_or("StackCalc Overflow")?;
        }
        if free_count == align {
          for j in (i.checked_sub(free_count).ok_or("StackCalc Overflow")?)..=i {
            if let Some(slot) = self.layout.get_mut(j) {
              *slot = true;
            }
          }
          return i.checked_add(1).ok_or("StackCalc Overflow");
        }
      }
    }
    let pad = self.layout.len().rem_euclid(align);
    self.layout.extend(iter::repeat_n(false, pad));
    let start = self.layout.len();
    self.layout.extend(iter::repeat_n(true, align));
    start.checked_add(align).ok_or("StackCalc Overflow")
  }
  /// Update `args_slots` (only if size is larger)
  #[expect(dead_code, reason = "todo")]
  pub fn update_max(&mut self, size: usize) {
    self.args_slots = cmp::max(self.args_slots, size);
  }
}
