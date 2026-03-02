use crate::prelude::*;
#[derive(Debug, Clone, Copy)]
#[expect(clippy::arbitrary_source_item_ordering)]
pub(crate) struct SectHeader {
  name: [u8; 8],
  pub v_size: u32,
  pub v_addr: u32,
  pub r_size: u32,
  pub r_ptr: u32,
  characteristics: u32,
}
impl SectHeader {
  pub(crate) fn encode(&self) -> Vec<u8> {
    let mut out = Vec::with_capacity(40);
    extend!(
      out,
      self.name,
      self.v_size.to_le_bytes(),
      self.v_addr.to_le_bytes(),
      self.r_size.to_le_bytes(),
      self.r_ptr.to_le_bytes(),
      [0; 12],
      self.characteristics.to_le_bytes()
    );
    out
  }
  pub(crate) fn from(
    name: [u8; 8],
    v_size: u32,
    v_addr: u32,
    r_size: u32,
    r_ptr: u32,
    characteristics: u32,
  ) -> Self {
    SectHeader { name, v_size, v_addr, r_size, r_ptr, characteristics }
  }
  pub(crate) fn from_prev(
    name: [u8; 8],
    v_size: u32,
    prev: &SectHeader,
    characteristics: u32,
  ) -> ErrOR<Self> {
    Ok(SectHeader {
      name,
      v_size,
      v_addr: prev.next_v_addr()?,
      r_size: r_size(v_size)?,
      r_ptr: prev.next_r_ptr(),
      characteristics,
    })
  }
  pub(crate) fn next_r_ptr(&self) -> u32 {
    self.r_ptr + self.r_size
  }
  pub(crate) fn next_v_addr(&self) -> ErrOR<u32> {
    Ok(self.v_addr + align_up_32(self.v_size, SECTION_ALIGNMENT)?)
  }
}
