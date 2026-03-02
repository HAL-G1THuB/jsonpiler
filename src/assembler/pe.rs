use super::sizeof_entry;
use crate::prelude::*;
use std::{
  fs,
  io::{self, Seek as _, SeekFrom::Start, Write as _},
  path::Path,
};
impl Assembler {
  pub(crate) fn build_idata(&self) -> ErrOR<Vec<u8>> {
    let idt_size = self.sizeof_idt()?;
    let i_la_t_size = self.sizeof_iat()?;
    let mut cur_rva = self.rva[Idata as usize] + idt_size;
    let hint_name_start = cur_rva + i_la_t_size * 2;
    let estimated_hint_name = usize::try_from(i_la_t_size * 3)?;
    let mut hint_name = Vec::with_capacity(estimated_hint_name);
    let mut idt = Vec::with_capacity(usize::try_from(idt_size)?);
    let mut ilt_iat = Vec::with_capacity(usize::try_from(i_la_t_size * 2)?);
    for dll in &self.dlls {
      let entry_size = sizeof_entry(dll)?;
      let mut i_la_t = Vec::with_capacity(usize::try_from(entry_size)?);
      for func in &dll.1 {
        let hint_name_offset = u64::from(hint_name_start) + u64::try_from(hint_name.len())?;
        i_la_t.extend_from_slice(&hint_name_offset.to_le_bytes());
        extend!(hint_name, [0; 2], func.as_bytes(), [0]);
        hint_name.resize(align_up(hint_name.len(), 2)?, 0);
      }
      i_la_t.extend_from_slice(&[0; 8]);
      let hint_name_offset = (hint_name_start + u32::try_from(hint_name.len())?).to_le_bytes();
      let iat_offset = (cur_rva + entry_size).to_le_bytes();
      extend!(idt, cur_rva.to_le_bytes(), [0; 8], hint_name_offset, iat_offset);
      extend!(ilt_iat, i_la_t, i_la_t);
      extend!(hint_name, dll.0.as_bytes(), [0]);
      cur_rva += entry_size * 2;
    }
    idt.extend_from_slice(&[0; 20]);
    hint_name.resize(align_up(hint_name.len(), 2)?, 0);
    let mut idata = Vec::with_capacity(usize::try_from(hint_name_start)? + hint_name.len());
    extend!(idata, idt, ilt_iat, hint_name);
    Ok(idata)
  }
  pub(crate) fn build_pdata(&self, seh: &mut Vec<(u32, u32, u32)>) -> ErrOR<(Vec<u8>, Vec<u32>)> {
    let mut pdata = vec![];
    let mut stack_sizes = vec![];
    seh.sort_by(|lhs, rhs| self.labels[&lhs.0].1.cmp(&self.labels[&rhs.0].1));
    for (prologue, epilogue, size) in seh {
      extend!(
        pdata,
        self.get_rva(*prologue)?.to_le_bytes(),
        self.get_rva(*epilogue)?.to_le_bytes(),
        [0; 4]
      );
      stack_sizes.push(*size);
    }
    Ok((pdata, stack_sizes))
  }
  pub(crate) fn build_xdata(
    &self,
    xdata_v_addr: u32,
    pdata: &mut [u8],
    stack_sizes: &[u32],
    seh_handler: u32,
  ) -> ErrOR<Vec<u8>> {
    let mut xdata = vec![];
    #[expect(clippy::cast_possible_truncation)]
    for (idx, &size) in stack_sizes.iter().enumerate() {
      xdata.resize(align_up(xdata.len(), 4)?, 0);
      let unwind_info = idx * 12 + 8;
      let xdata_offset = xdata_v_addr + v_size(&xdata)?;
      pdata[unwind_info..unwind_info + 4].copy_from_slice(&xdata_offset.to_le_bytes());
      let push_rbp = self.sizeof_inst(&Push(Rbp), 0)? as u8;
      let mov_rbp_rsp = push_rbp + self.sizeof_inst(&mov_q(Rbp, Rsp), u32::from(push_rbp))? as u8;
      let sub_rsp_size =
        mov_rbp_rsp + self.sizeof_inst(&SubRId(Rsp, size), u32::from(mov_rbp_rsp))? as u8;
      extend!(
        xdata,
        [0o11, sub_rsp_size, 4, Rbp as u8, sub_rsp_size, 1],
        ((align_up_32(size, 8)? >> 3) as u16).to_le_bytes(),
        [mov_rbp_rsp, 3, push_rbp, (Rbp as u8) << 4u8],
        self.get_rva(seh_handler)?.to_le_bytes()
      );
    }
    Ok(xdata)
  }
  pub(crate) fn link(self, mut sect: [(Vec<u8>, SectHeader); 7], input: &str) -> ErrOR<()> {
    const PE32PLUS: u16 = 0x020B;
    const MACHINE_X64: u16 = 0x8664;
    const COFF_CHARACTERISTICS: u16 = 0x0222;
    let headers_size = align_up_32(PE_HEADER_OFFSET + PE_HEADERS_TOTAL, FILE_ALIGNMENT)?;
    let image_size = sect[Idata as usize].1.next_v_addr()?;
    let file_size = sect[Idata as usize].1.next_r_ptr();
    let mut out = Vec::with_capacity(usize::try_from(file_size)?);
    out.extend_from_slice(b"MZ");
    out.resize(PE_HEADER_OFFSET as usize, 0);
    out[0x3C..0x40].copy_from_slice(&PE_HEADER_OFFSET.to_le_bytes());
    let exe_path = Path::new(input).with_extension("exe");
    let exe = exe_path.to_string_lossy().to_string();
    let mut writer =
      io::BufWriter::with_capacity(file_size as usize, fs::File::create(exe.clone())?);
    write_all!(
      writer,
      *b"MZ",
      [0; PE_HEADER_OFFSET as usize - 6],
      PE_HEADER_OFFSET.to_le_bytes(),
      *b"PE\0\0",
      MACHINE_X64.to_le_bytes(),
      NUMBER_OF_SECTIONS.to_le_bytes(),
      time_stamp().to_le_bytes(),
      [0; 8],
      OPTIONAL_HEADER_SIZE.to_le_bytes(),
      COFF_CHARACTERISTICS.to_le_bytes(),
      PE32PLUS.to_le_bytes(),
      [VER_MAJOR, VER_MINOR],
      sect[Text as usize].1.r_size.to_le_bytes(),
      (sect[Text as usize].1.r_size + sect[Data as usize].1.r_size).to_le_bytes(),
      align_up_32(sect[Bss as usize].1.v_size, FILE_ALIGNMENT)?.to_le_bytes(),
      SECTION_ALIGNMENT.to_le_bytes(),
      SECTION_ALIGNMENT.to_le_bytes(),
      IMAGE_BASE.to_le_bytes(),
      SECTION_ALIGNMENT.to_le_bytes(),
      FILE_ALIGNMENT.to_le_bytes(),
      4u64.to_le_bytes(),
      *b"\x05\x00\x02\x00\x00\x00\x00\x00",
      image_size.to_le_bytes(),
      headers_size.to_le_bytes(),
      *b"\0\0\0\0\x03\0\0\0",
      *b"\0\0\x20\0\0\0\0\0",
      *b"\0\x10\0\0\0\0\0\0",
      *b"\0\0\x10\0\0\0\0\0",
      *b"\0\x10\0\0\0\0\0\0",
      *b"\0\0\0\0\x10\0\0\0",
      [0; 8],
      sect[Idata as usize].1.v_addr.to_le_bytes(),
      self.sizeof_idt()?.to_le_bytes(),
      [0; 8],
      sect[Pdata as usize].1.v_addr.to_le_bytes(),
      sect[Pdata as usize].1.v_size.to_le_bytes(),
      [0; 64],
      self.i_f_rva(0, 0)?.to_le_bytes(),
      self.sizeof_iat()?.to_le_bytes(),
      [0; 24],
      sect[Text as usize].1.encode(),
      sect[Data as usize].1.encode(),
      sect[Rdata as usize].1.encode(),
      sect[Pdata as usize].1.encode(),
      sect[Xdata as usize].1.encode(),
      sect[Bss as usize].1.encode(),
      sect[Idata as usize].1.encode(),
    );
    let mut file = writer.into_inner()?;
    file.set_len(u64::from(file_size))?;
    file.seek(Start(u64::from(sect[Text as usize].1.r_ptr)))?;
    file.write_all(&take(&mut sect[Text as usize].0))?;
    file.seek(Start(u64::from(sect[Data as usize].1.r_ptr)))?;
    file.write_all(&take(&mut sect[Data as usize].0))?;
    file.seek(Start(u64::from(sect[Rdata as usize].1.r_ptr)))?;
    file.write_all(&take(&mut sect[Rdata as usize].0))?;
    file.seek(Start(u64::from(sect[Pdata as usize].1.r_ptr)))?;
    file.write_all(&take(&mut sect[Pdata as usize].0))?;
    file.seek(Start(u64::from(sect[Xdata as usize].1.r_ptr)))?;
    file.write_all(&take(&mut sect[Xdata as usize].0))?;
    file.seek(Start(u64::from(sect[Idata as usize].1.r_ptr)))?;
    file.write_all(&take(&mut sect[Idata as usize].0))?;
    Ok(())
  }
}
