use super::sizeof_entry;
use crate::prelude::*;
use std::io::{self, Seek as _, SeekFrom::Start, Write as _};
impl Assembler {
  pub(crate) fn build_idata(&self) -> ErrOR<Vec<u8>> {
    let idt_size = self.sizeof_idt()?;
    let i_la_t_size = self.sizeof_iat()?;
    let mut cur_rva = self.rva[IData as usize] + idt_size;
    let hint_name_start = cur_rva + i_la_t_size * 2;
    let estimated_hint_name = (i_la_t_size * 3) as usize;
    let mut hint_name = Vec::with_capacity(estimated_hint_name);
    let mut idt = Vec::with_capacity(idt_size as usize);
    let mut ilt_iat = Vec::with_capacity((i_la_t_size * 2) as usize);
    for dll in &self.dlls {
      let entry_size = sizeof_entry(dll)?;
      let mut i_la_t = Vec::with_capacity(entry_size as usize);
      for func in &dll.1 {
        let hint_name_offset = u64::from(hint_name_start) + u64::try_from(hint_name.len())?;
        i_la_t.extend_from_slice(&hint_name_offset.to_le_bytes());
        extend!(hint_name, [0; 2], func.as_bytes(), [0]);
        hint_name.resize(align_up(hint_name.len(), 2)?, 0);
      }
      i_la_t.extend_from_slice(&[0; 8]);
      let hint_name_offset = (hint_name_start + len_u32(&hint_name)?).to_le_bytes();
      let iat_offset = (cur_rva + entry_size).to_le_bytes();
      extend!(idt, cur_rva.to_le_bytes(), [0; 8], hint_name_offset, iat_offset);
      extend!(ilt_iat, i_la_t, i_la_t);
      extend!(hint_name, dll.0.as_bytes(), [0]);
      cur_rva += entry_size * 2;
    }
    idt.extend_from_slice(&[0; 20]);
    hint_name.resize(align_up(hint_name.len(), 2)?, 0);
    let mut idata = Vec::with_capacity(hint_name_start as usize + hint_name.len());
    extend!(idata, idt, ilt_iat, hint_name);
    Ok(idata)
  }
  pub(crate) fn build_pdata(&self, seh: &mut Vec<(u32, u32, i32)>) -> ErrOR<(Vec<u8>, Vec<i32>)> {
    let mut pdata = vec![];
    let mut stack_sizes = vec![];
    seh.sort_by(|lhs, rhs| self.labels[&lhs.0].1.cmp(&self.labels[&rhs.0].1));
    for (id, end, size) in seh {
      extend!(pdata, self.get_rva(*id)?.to_le_bytes(), self.get_rva(*end)?.to_le_bytes(), [0; 4]);
      stack_sizes.push(*size);
    }
    Ok((pdata, stack_sizes))
  }
  pub(crate) fn build_xdata(
    &self,
    xdata_v_addr: u32,
    pdata: &mut [u8],
    stack_sizes: &[i32],
  ) -> ErrOR<Vec<u8>> {
    let mut xdata = vec![];
    #[expect(clippy::cast_possible_truncation)]
    for (idx, &size) in stack_sizes.iter().enumerate() {
      xdata.resize(align_up(xdata.len(), 4)?, 0);
      let unwind_info = idx * 12 + 8;
      let xdata_offset = xdata_v_addr + len_u32(&xdata)?;
      pdata[unwind_info..unwind_info + 4].copy_from_slice(&xdata_offset.to_le_bytes());
      let push_rbp = self.sizeof_inst(&Push(Rbp), 0)?;
      let mov_rbp_rsp = push_rbp + self.sizeof_inst(&mov_q(Rbp, Rsp), push_rbp)?;
      let sub_rsp_size =
        u8::try_from(mov_rbp_rsp + self.sizeof_inst(&SubRId(Rsp, size), mov_rbp_rsp)?)?;
      extend!(
        xdata,
        [0o11, sub_rsp_size, 4, Rbp as u8, sub_rsp_size, 1],
        ((size >> 3).cast_unsigned() as u16).to_le_bytes(),
        [u8::try_from(mov_rbp_rsp)?, 3, u8::try_from(push_rbp)?, (Rbp as u8) << 4u8],
        self.get_rva(self.handlers.seh)?.to_le_bytes()
      );
    }
    Ok(xdata)
  }
  pub(crate) fn link(self, sect: &[(Vec<u8>, SectionHeader); 7], input: &str) -> ErrOR<()> {
    const PE32PLUS: u16 = 0x020B;
    const MACHINE_X64: u16 = 0x8664;
    const COFF_CHARACTERISTICS: u16 = 0x0222;
    let data_size = sect[Data as usize].1.v_size
      + sect[RData as usize].1.v_size
      + sect[PData as usize].1.v_size
      + sect[XData as usize].1.v_size
      + sect[IData as usize].1.v_size;
    let headers_size = align_up_u32(HEADERS_SIZE, FILE_ALIGNMENT)?;
    let image_size = sect[IData as usize].1.next_v_addr()?;
    let file_size = sect[IData as usize].1.next_r_ptr();
    let mut out = Vec::with_capacity(file_size as usize);
    out.extend_from_slice(b"MZ");
    out.resize(PE_HEADER_OFFSET as usize, 0);
    out[0x3C..0x40].copy_from_slice(&PE_HEADER_OFFSET.to_le_bytes());
    let exe_path = Path::new(input).with_extension("exe");
    let exe = exe_path.to_string_lossy().to_string();
    let mut writer = io::BufWriter::with_capacity(file_size as usize, fs::File::create(exe)?);
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
      VER_MAJOR_MINOR,
      sect[Text as usize].1.v_size.to_le_bytes(),
      data_size.to_le_bytes(),
      sect[Bss as usize].1.v_size.to_le_bytes(),
      self.get_rva(self.root_id)?.to_le_bytes(),
      sect[Text as usize].1.v_addr.to_le_bytes(),
      IMAGE_BASE.to_le_bytes(),
      SECTION_ALIGNMENT.to_le_bytes(),
      FILE_ALIGNMENT.to_le_bytes(),
      4u64.to_le_bytes(),
      0x2_0005u64.to_le_bytes(),
      image_size.to_le_bytes(),
      headers_size.to_le_bytes(),
      0x03_0000_0000u64.to_le_bytes(),
      0x00_0020_0000u64.to_le_bytes(),
      0x00_0000_1000u64.to_le_bytes(),
      0x00_0010_0000u64.to_le_bytes(),
      0x00_0000_1000u64.to_le_bytes(),
      0x10_0000_0000u64.to_le_bytes(),
      [0; 8],
      sect[IData as usize].1.v_addr.to_le_bytes(),
      self.sizeof_idt()?.to_le_bytes(),
      [0; 8],
      sect[PData as usize].1.v_addr.to_le_bytes(),
      sect[PData as usize].1.v_size.to_le_bytes(),
      [0; 64],
      (self.rva[IData as usize] + self.sizeof_idt()?).to_le_bytes(),
      self.sizeof_iat()?.to_le_bytes(),
      [0; 24],
      sect[Text as usize].1.encode(),
      sect[Data as usize].1.encode(),
      sect[RData as usize].1.encode(),
      sect[PData as usize].1.encode(),
      sect[XData as usize].1.encode(),
      sect[Bss as usize].1.encode(),
      sect[IData as usize].1.encode(),
    );
    let mut file = writer.into_inner().map_err(|err| IO(err.error().to_string()))?;
    file.set_len(u64::from(file_size))?;
    write_section(&mut file, &sect[Text as usize])?;
    write_section(&mut file, &sect[Data as usize])?;
    write_section(&mut file, &sect[RData as usize])?;
    write_section(&mut file, &sect[PData as usize])?;
    write_section(&mut file, &sect[XData as usize])?;
    write_section(&mut file, &sect[IData as usize])?;
    Ok(())
  }
}
fn write_section(file: &mut fs::File, (data, header): &(Vec<u8>, SectionHeader)) -> ErrOR<()> {
  file.seek(Start(u64::from(header.r_ptr)))?;
  file.write_all(data)?;
  Ok(())
}
