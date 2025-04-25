//! Implementation for `FuncInfo`.
use {super::FuncInfo, core::cmp};
impl FuncInfo {
  /// Add local variable size (in bytes)
  #[expect(dead_code, reason = "todo")]
  pub fn add_local(&mut self, size: usize) -> Result<(), &'static str> {
    self.local_size = self.local_size.checked_add(size).ok_or("LocalSize Overflow")?;
    Ok(())
  }
  /// Calculate to allocate size.
  pub fn calc_alloc(&self, align: usize) -> Result<usize, &'static str> {
    let args_size = self.args_slots.checked_mul(8).ok_or("Overflow: args_slots * 8")?;
    let raw = self.local_size.checked_add(args_size).ok_or("Overflow: local_size + args")?;
    let locals = raw.checked_add(15).ok_or("Overflow before alignment")? & !15;
    let shadow_space = 32usize.checked_add(align).ok_or("ShadowSpace Overflow")?;
    locals.checked_add(shadow_space).ok_or("LocalSize Overflow")
  }
  /// Update `args_slots` (only if size is larger)
  #[expect(dead_code, reason = "todo")]
  pub fn update_max(&mut self, size: usize) {
    self.args_slots = cmp::max(self.args_slots, size);
  }
}
