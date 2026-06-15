pub(crate) mod encode;
pub(crate) mod inst;
pub(crate) mod lc_segment;
mod link_edit;
pub(crate) mod load_cmd;
pub(crate) mod mach_o;
pub(crate) mod ops;
pub(crate) mod register;
pub(crate) mod section;
mod sha256;
use crate::assembler::a64::mach_o::A64Assembler;
use crate::prelude::*;
impl A64Assembler {
  pub(crate) fn insert_lbl(&mut self, id: LabelId, sect: A64Sect, offset: u32) -> ErrOR<()> {
    if self.labels.insert(id, (sect, offset)).is_some() {
      return Err(DuplicateLabel.into());
    }
    Ok(())
  }
}
pub(crate) fn len_u64<T>(slice: &[T]) -> u64 {
  slice.len() as u64
}
