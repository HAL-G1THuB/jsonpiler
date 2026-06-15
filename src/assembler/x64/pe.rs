use super::*;
use crate::prelude::*;
use io::{Seek as _, Write as _};
impl X64Assembler {
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
        let hint_name_off = u64::from(hint_name_start) + u64::try_from(hint_name.len())?;
        i_la_t.extend_from_slice(&hint_name_off.to_le_bytes());
        extend!(hint_name, [0; 2], func.as_bytes(), [0]);
        hint_name.resize(align_up(hint_name.len(), 2)?, 0);
      }
      i_la_t.resize(i_la_t.len() + 8, 0);
      let hint_name_off = hint_name_start + len_u32(&hint_name)?;
      let iat_off = cur_rva + entry_size;
      extend!(
        idt,
        cur_rva.to_le_bytes(),
        [0; 8],
        hint_name_off.to_le_bytes(),
        iat_off.to_le_bytes()
      );
      extend!(ilt_iat, i_la_t, i_la_t);
      extend!(hint_name, dll.0.as_bytes(), [0]);
      cur_rva += entry_size * 2;
    }
    idt.resize(idt.len() + 20, 0);
    hint_name.resize(align_up(hint_name.len(), 2)?, 0);
    let idata_size = hint_name_start as usize + hint_name.len();
    let mut idata = Vec::with_capacity(idata_size);
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
    for (idx, &size) in stack_sizes.iter().enumerate() {
      xdata.resize(align_up(xdata.len(), 4)?, 0);
      let unwind_info = idx * 12 + 8;
      let xdata_off = xdata_v_addr + len_u32(&xdata)?;
      pdata[unwind_info..unwind_info + 4].copy_from_slice(&xdata_off.to_le_bytes());
      let push_rbp = u8::try_from(self.sizeof_inst(&Push(Rbp), Some(0))?)?;
      let mov_rbp_rsp =
        push_rbp + u8::try_from(self.sizeof_inst(&mov(S8, Rbp, Rsp), Some(u32::from(push_rbp)))?)?;
      let sub_rsp_size = mov_rbp_rsp
        + u8::try_from(self.sizeof_inst(&m8i(Sub, Rsp, size), Some(u32::from(mov_rbp_rsp)))?)?;
      extend!(
        xdata,
        [0o11, sub_rsp_size, 4, Rbp as u8, sub_rsp_size, 1],
        ((size >> 3).cast_unsigned() as u16).to_le_bytes(),
        [mov_rbp_rsp, 3, push_rbp, (Rbp as u8) << 4u8],
        self.get_rva(self.handlers.internal)?.to_le_bytes()
      );
    }
    Ok(xdata)
  }
}
impl X64Assembler {
  pub(crate) fn build_pe(self, sect: &[(Vec<u8>, SectionHeader); 7], input: &str) -> ErrOR<()> {
    const PE32PLUS: u16 = 0x020B;
    const MACHINE_X64: u16 = 0x8664;
    const COFF_CHARACTERISTICS: u16 = 0x0222;
    let data_size: u32 =
      [DataX, RData, PData, XData, IData].into_iter().map(|se| sect[se as usize].1.v_size).sum();
    let headers_size = align_up_u32(HEADERS_SIZE, FILE_ALIGN)?;
    let image_size = sect[IData as usize].1.next_v_addr()?;
    let file_size = sect[IData as usize].1.next_r_ptr();
    let mut header = le_bytes!(
      u16::from_le_bytes(*b"MZ"),
      0u16,
      0u128,
      0u128,
      0u128,
      0u64,
      PE_HEADER_OFFSET,
      u32::from_le_bytes(*b"PE\0\0"),
      MACHINE_X64,
      NUMBER_OF_SECTIONS,
      // time_date_stamp
      0u32,
      0u64,
      OPTIONAL_HEADER_SIZE,
      COFF_CHARACTERISTICS,
      PE32PLUS,
      u16::from_le_bytes(VER_MAJOR_MINOR),
      // size_and_entry_point
      0u32,
      0u64,
      0u64,
      IMAGE_BASE,
      SECTION_ALIGN,
      FILE_ALIGN,
      4u64,
      0x2_0005u64,
      // image_headers_size
      0u64,
      0x03_0000_0000u64,
      0x00_0020_0000u64,
      0x00_0000_1000u64,
      0x00_0010_0000u64,
      0x00_0000_1000u64,
      0x10_0000_0000u64,
    );
    let time_date_stamp_off = PE_HEADER_OFFSET as usize + 4 + 2 + 2;
    let time_date_stamp = (now().as_secs() as u32).to_le_bytes();
    header[time_date_stamp_off..time_date_stamp_off + 4].copy_from_slice(&time_date_stamp);
    let optional_etc_off = PE_HEADER_OFFSET as usize + COFF_HEADER_SIZE as usize + 2 + 2;
    let etc = [
      sect[TextX as usize].1.v_size,
      data_size,
      sect[BssX as usize].1.v_size,
      self.get_rva(self.root_id)?,
      self.rva[TextX as usize],
    ];
    for (idx, value) in etc.into_iter().enumerate() {
      let offset = optional_etc_off + idx * 4;
      header[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    let i_h_size_off = optional_etc_off + 20 + 8 + 4 + 4 + 8 + 8;
    header[i_h_size_off..i_h_size_off + 4].copy_from_slice(&image_size.to_le_bytes());
    header[i_h_size_off + 4..i_h_size_off + 8].copy_from_slice(&headers_size.to_le_bytes());
    let mut data_dir = [0; 0x80];
    fn data_dir_entry(data_dir: &mut [u8], idx: usize, rva: u32, size: u32) {
      let offset = idx * 8;
      data_dir[offset..offset + 4].copy_from_slice(&rva.to_le_bytes());
      data_dir[offset + 4..offset + 8].copy_from_slice(&size.to_le_bytes());
    }
    let iat_off = self.rva[IData as usize] + self.sizeof_idt()?;
    data_dir_entry(&mut data_dir, 1, self.rva[IData as usize], self.sizeof_idt()?);
    data_dir_entry(&mut data_dir, 3, self.rva[PData as usize], sect[PData as usize].1.v_size);
    data_dir_entry(&mut data_dir, 12, iat_off, self.sizeof_iat()? * 2);
    let exe = Path::new(input).with_extension("exe").to_string_lossy().to_string();
    let mut file = fs::File::create(exe)?;
    file.set_len(u64::from(file_size))?;
    file.write_all(&header)?;
    file.write_all(&data_dir)?;
    for (_, sect_header) in sect {
      file.write_all(&sect_header.encode())?;
    }
    for (data, section_header) in sect {
      if section_header.r_size > 0 {
        file.seek(io::SeekFrom::Start(u64::from(section_header.r_ptr)))?;
        file.write_all(data)?;
      }
    }
    Ok(())
  }
}
