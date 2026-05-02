use super::utility::r_size;
use crate::prelude::*;
#[repr(u8)]
#[derive(Eq, PartialEq, Hash, Clone, Copy)]
pub(crate) enum Section {
  Text,
  Data,
  RData,
  PData,
  XData,
  Bss,
  IData,
}
#[derive(Debug, Clone, Copy)]
#[expect(clippy::arbitrary_source_item_ordering)]
pub(crate) struct SectionHeader {
  name: [u8; 8],
  pub v_size: u32,
  pub v_addr: u32,
  pub r_size: u32,
  pub r_ptr: u32,
  characteristics: u32,
}
impl SectionHeader {
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
  pub(crate) fn from(sect: Section, v_size: u32, v_addr: u32, r_size: u32, r_ptr: u32) -> Self {
    SectionHeader {
      name: sect.name(),
      v_size,
      v_addr,
      r_size,
      r_ptr,
      characteristics: sect.characteristics(),
    }
  }
  pub(crate) fn next(&self, sect: Section, v_size: u32) -> ErrOR<Self> {
    Ok(SectionHeader {
      name: sect.name(),
      v_size,
      v_addr: self.next_v_addr()?,
      r_size: r_size(v_size)?,
      r_ptr: self.next_r_ptr(),
      characteristics: sect.characteristics(),
    })
  }
  pub(crate) fn next_r_ptr(&self) -> u32 {
    self.r_ptr + self.r_size
  }
  pub(crate) fn next_v_addr(&self) -> ErrOR<u32> {
    Ok(self.v_addr + align_up_u32(self.v_size, SECTION_ALIGNMENT)?)
  }
}
impl Section {
  pub(crate) fn characteristics(self) -> u32 {
    match self {
      Text => 0x6000_0020,
      Data | IData => 0xC000_0040,
      Bss => 0xC000_0080,
      RData | PData | XData => 0x4000_0040,
    }
  }
  pub(crate) fn name(self) -> [u8; 8] {
    match self {
      Text => *b".text\0\0\0",
      Data => *b".data\0\0\0",
      RData => *b".rdata\0\0",
      PData => *b".pdata\0\0",
      XData => *b".xdata\0\0",
      Bss => *b".bss\0\0\0\0",
      IData => *b".idata\0\0",
    }
  }
}
