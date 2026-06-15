use super::mach_o::*;
use crate::prelude::*;
#[expect(clippy::arbitrary_source_item_ordering)]
#[derive(Debug, Clone)]
pub(crate) struct A64LcSegment {
  name: [u8; 16],
  pub vm_addr: u64,
  pub vm_size: u64,
  pub file_off: u64,
  pub file_size: u64,
  max_prot: Protection,
  init_prot: Protection,
  flags: u32,
  sections: Vec<A64LcSection>,
}
#[expect(clippy::arbitrary_source_item_ordering)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct A64LcSection {
  name: [u8; 16],
  addr: u64,
  size: u64,
  offset: u32,
  align: u32,
  flags: u32,
  reserved: [u32; 3],
}
impl A64LcSegment {
  pub(crate) fn dummy(n_sects: usize) -> Self {
    const ___: Protection = Protection { r: false, w: false, e: false };
    Self {
      name: [0u8; 16],
      vm_addr: 0,
      vm_size: 0,
      file_off: 0,
      file_size: 0,
      max_prot: ___,
      init_prot: ___,
      flags: 0,
      sections: vec![
        A64LcSection {
          name: [0u8; 16],
          addr: 0,
          size: 0,
          offset: 0,
          align: 0,
          flags: 0,
          reserved: [0; 3],
        };
        n_sects
      ],
    }
  }
  pub(crate) fn lc_segment(&self) -> ErrOR<Vec<u8>> {
    const LC_SEGMENT_64: u32 = 0x19;
    let total = self.sizeof_cmd()?;
    let mut bytes = Vec::with_capacity(total as usize);
    extend!(
      bytes,
      LC_SEGMENT_64.to_le_bytes(),
      total.to_le_bytes(),
      self.name,
      self.vm_addr.to_le_bytes(),
      self.sizeof_vm()?.to_le_bytes(),
      self.file_off.to_le_bytes(),
      self.sizeof_file()?.to_le_bytes(),
      self.max_prot.to_le_bytes(),
      self.init_prot.to_le_bytes(),
      len_u32(&self.sections)?.to_le_bytes(),
      self.flags.to_le_bytes(),
    );
    for sect in &self.sections {
      extend!(
        bytes,
        sect.name,
        self.name,
        sect.addr.to_le_bytes(),
        sect.size.to_le_bytes(),
        sect.offset.to_le_bytes(),
        sect.align.to_le_bytes(),
        [0; 8],
        sect.flags.to_le_bytes(),
        sect.reserved[0].to_le_bytes(),
        sect.reserved[1].to_le_bytes(),
        sect.reserved[2].to_le_bytes()
      );
    }
    Ok(bytes)
  }
}
impl A64LcSegment {
  pub(crate) fn add(
    &mut self,
    name: &[u8],
    size: u64,
    align: u32,
    flags: u32,
    reserved: [u32; 3],
    bss: bool,
  ) -> ErrOR<()> {
    let offset = align_up_u64(self.file_off + self.file_size, 1 << align)?;
    let addr = align_up_u64(self.vm_addr + self.vm_size, 1 << align)?;
    self.sections.push(A64LcSection {
      name: seg_sect_name(name),
      addr,
      size,
      offset: u32::try_from(offset)?,
      align,
      flags,
      reserved,
    });
    if !bss {
      self.file_size += size;
    }
    self.vm_size += size;
    Ok(())
  }
}
impl A64LcSegment {
  pub(crate) fn next(
    &self,
    name: &[u8],
    size: Option<u64>,
    prot: Protection,
    flags: u32,
  ) -> ErrOR<Self> {
    let file_off = self.file_off + self.sizeof_file()?;
    let vm_addr = self.vm_addr + self.sizeof_vm()?;
    Ok(Self {
      name: seg_sect_name(name),
      vm_addr,
      vm_size: size.unwrap_or(0),
      file_off,
      file_size: size.unwrap_or(0),
      max_prot: prot,
      init_prot: prot,
      flags,
      sections: vec![],
    })
  }
  pub(crate) fn page_zero() -> Self {
    Self {
      name: seg_sect_name(b"PAGEZERO"),
      vm_addr: 0,
      vm_size: A64_TEXT_ADDR,
      file_off: 0,
      file_size: 0,
      max_prot: Protection::default(),
      init_prot: Protection::default(),
      flags: 0,
      sections: vec![],
    }
  }
  pub(crate) fn sizeof_cmd(&self) -> ErrOR<u32> {
    Ok(0x48 + len_u32(&self.sections)? * 0x50)
  }
  pub(crate) fn sizeof_file(&self) -> ErrOR<u64> {
    align_up_u64(self.file_size, u64::from(A64_SEG_ALIGN))
  }
  pub(crate) fn sizeof_vm(&self) -> ErrOR<u64> {
    align_up_u64(self.vm_size, u64::from(A64_SEG_ALIGN))
  }
}
pub(crate) fn seg_sect_name(name: &[u8]) -> [u8; 16] {
  let mut buf = [0u8; 16];
  let len = name.len().min(14);
  buf[0..2].copy_from_slice(b"__");
  buf[2..len + 2].copy_from_slice(&name[..len]);
  buf
}
