use crate::{
  Assembler, ErrOR, Sect, extend_bytes,
  utility::{align_up, align_up_32, get_time_stamp},
};
impl Assembler {
  pub(crate) fn build_idata_section(&self, base_rva: u32) -> ErrOR<Vec<u8>> {
    let dll_count = self.dlls.len();
    let idt_size = (dll_count + 1) * 0x14;
    let mut lookup_offsets = Vec::with_capacity(dll_count);
    let mut address_offsets = Vec::with_capacity(dll_count);
    let mut dll_name_offsets = Vec::with_capacity(dll_count);
    let mut hint_name_table = Vec::with_capacity(256);
    let mut current_offset = u32::try_from(idt_size)?;
    let mut func_name_offsets: Vec<Vec<u32>> = Vec::with_capacity(dll_count);
    for dll in &self.dlls {
      let funcs_count = u32::try_from(dll.1.len())?;
      let lookup_size = (funcs_count + 1) * 8;
      lookup_offsets.push(current_offset);
      address_offsets.push(current_offset + lookup_size);
      current_offset += lookup_size * 2;
      let mut offsets = Vec::with_capacity(dll.1.len());
      for (hint, func) in &dll.1 {
        let offset = align_up(hint_name_table.len(), 8)?;
        hint_name_table.resize(offset, 0);
        offsets.push(u32::try_from(offset)?);
        hint_name_table.extend_from_slice(&hint.to_le_bytes());
        hint_name_table.extend_from_slice(func.as_bytes());
        hint_name_table.push(0);
      }
      func_name_offsets.push(offsets);
      let dll_name_offset = align_up_32(u32::try_from(hint_name_table.len())?, 8)?;
      hint_name_table.resize(usize::try_from(dll_name_offset)?, 0);
      hint_name_table.extend_from_slice(dll.0.as_bytes());
      hint_name_table.push(0);
      dll_name_offsets.push(dll_name_offset);
    }
    let aligned_hint_name_len = align_up(hint_name_table.len(), 8)?;
    hint_name_table.resize(aligned_hint_name_len, 0);
    let total_lookup_size: usize = self.dlls.iter().map(|dll| (dll.1.len() + 1) * 8 * 2).sum();
    let mut idata = Vec::with_capacity(idt_size + total_lookup_size + aligned_hint_name_len);
    for i in 0..dll_count {
      let lookup_rva = base_rva + lookup_offsets[i];
      let address_rva = base_rva + address_offsets[i];
      let name_rva = base_rva + current_offset + dll_name_offsets[i];
      idata.extend_from_slice(&lookup_rva.to_le_bytes());
      idata.extend_from_slice(&[0; 8]);
      idata.extend_from_slice(&name_rva.to_le_bytes());
      idata.extend_from_slice(&address_rva.to_le_bytes());
    }
    idata.extend_from_slice(&[0; 20]);
    let mut lookup_address_data = Vec::with_capacity(total_lookup_size);
    for (dll_i, dll) in self.dlls.iter().enumerate() {
      for &offset in &func_name_offsets[dll_i] {
        let rva = base_rva + current_offset + offset;
        lookup_address_data.extend_from_slice(&u64::from(rva).to_le_bytes());
      }
      lookup_address_data.extend_from_slice(&[0; 8]);
      let lookup_start = lookup_address_data.len() - (dll.1.len() + 1) * 8;
      let address = lookup_address_data[lookup_start..].to_vec();
      lookup_address_data.extend(address);
    }
    idata.extend(lookup_address_data);
    idata.extend(hint_name_table);
    Ok(idata)
  }
  #[expect(clippy::too_many_lines)]
  pub(crate) fn build_pe(self, code: &[u8], data: &[u8], bss: u32, idata: &[u8]) -> ErrOR<Vec<u8>> {
    const IMAGE_BASE: u64 = 0x1_4000_0000;
    const FILE_ALIGNMENT: u32 = 0x200;
    const SECTION_ALIGNMENT: u32 = 0x1000;
    const PE_HEADER_OFFSET: u32 = 0x80;
    const NUMBER_OF_SECTIONS: u16 = 4;
    let pe_headers_total = 4 + 20 + 0xF0 + 40 * usize::from(NUMBER_OF_SECTIONS);
    let size_of_headers_unaligned = usize::try_from(PE_HEADER_OFFSET)? + pe_headers_total;
    let size_of_headers = align_up_32(u32::try_from(size_of_headers_unaligned)?, FILE_ALIGNMENT)?;
    let text_v_size = u32::try_from(code.len())?;
    let text_v_address = self.rva[&Sect::Text];
    let text_raw_size = align_up_32(text_v_size, FILE_ALIGNMENT)?;
    let text_raw_ptr = align_up_32(size_of_headers, FILE_ALIGNMENT)?;
    let data_v_size = u32::try_from(data.len())?;
    let data_v_address = self.rva[&Sect::Data];
    let data_raw_size = align_up_32(data_v_size, FILE_ALIGNMENT)?;
    let data_raw_ptr = text_raw_ptr + text_raw_size;
    let bss_v_size = bss;
    let bss_v_address = self.rva[&Sect::Bss];
    let idata_v_size = u32::try_from(idata.len())?;
    let idata_v_address = self.rva[&Sect::Idata];
    let idata_raw_size = align_up_32(u32::try_from(idata.len())?, FILE_ALIGNMENT)?;
    let idata_raw_ptr = data_raw_ptr + data_raw_size;
    let size_of_image = align_up_32(
      idata_v_address + align_up_32(idata_v_size, SECTION_ALIGNMENT)?,
      SECTION_ALIGNMENT,
    )?;
    let size_of_file = idata_raw_ptr + idata_raw_size;
    let mut out = Vec::with_capacity(usize::try_from(size_of_file)?);
    extend_bytes!(
      out,
      include_bytes!("bin/dos_stub.bin"),
      b"PE\0\0",
      &0x8664u16.to_le_bytes(),
      &NUMBER_OF_SECTIONS.to_le_bytes(),
      &get_time_stamp()?.to_le_bytes(),
      &[0; 8],
      &0xF0u16.to_le_bytes(),
      &0x0222u16.to_le_bytes(),
      &0x20Bu16.to_le_bytes(),
      &[1; 2],
      &text_raw_size.to_le_bytes(),
      &(text_raw_size + data_raw_size).to_le_bytes(),
      &align_up_32(bss_v_size, FILE_ALIGNMENT)?.to_le_bytes(),
      &SECTION_ALIGNMENT.to_le_bytes(),
      &SECTION_ALIGNMENT.to_le_bytes(),
      &IMAGE_BASE.to_le_bytes(),
      &SECTION_ALIGNMENT.to_le_bytes(),
      &FILE_ALIGNMENT.to_le_bytes(),
      &4u16.to_le_bytes(),
      &[0; 6],
      &5u16.to_le_bytes(),
      &2u16.to_le_bytes(),
      &[0; 4],
      &size_of_image.to_le_bytes(),
      &size_of_headers.to_le_bytes(),
      &[0; 4],
      &3u16.to_le_bytes(),
      &0u16.to_le_bytes(),
      &0x20_0000u64.to_le_bytes(),
      &0x1000u64.to_le_bytes(),
      &0x10_0000u64.to_le_bytes(),
      &0x1000u64.to_le_bytes(),
      &[0; 4],
      &16u32.to_le_bytes(),
    );
    out.resize(out.len() + 8, 0);
    out.extend_from_slice(&idata_v_address.to_le_bytes());
    out.extend_from_slice(&u32::try_from((self.dlls.len() + 1) * 20)?.to_le_bytes());
    out.resize(out.len() + 8 * 10, 0);
    out.extend_from_slice(&self.resolve_address_rva(0, 0)?.to_le_bytes());
    out.extend_from_slice(&u32::try_from(self.resolve_iat_size())?.to_le_bytes());
    out.resize(out.len() + 8 * 3, 0);
    extend_bytes!(
      out,
      b".text\0\0\0",
      &text_v_size.to_le_bytes(),
      &text_v_address.to_le_bytes(),
      &text_raw_size.to_le_bytes(),
      &text_raw_ptr.to_le_bytes(),
      &[0; 12],
      &0x6000_0020u32.to_le_bytes(),
      b".data\0\0\0",
      &data_v_size.to_le_bytes(),
      &data_v_address.to_le_bytes(),
      &data_raw_size.to_le_bytes(),
      &data_raw_ptr.to_le_bytes(),
      &[0; 12],
      &0xC000_0040u32.to_le_bytes(),
      b".bss\0\0\0\0",
      &bss_v_size.to_le_bytes(),
      &bss_v_address.to_le_bytes(),
      &[0; 20],
      &0xC000_0080u32.to_le_bytes(),
      b".idata\0\0",
      &idata_v_size.to_le_bytes(),
      &idata_v_address.to_le_bytes(),
      &idata_raw_size.to_le_bytes(),
      &idata_raw_ptr.to_le_bytes(),
      &[0; 12],
      &0x4000_0040u32.to_le_bytes(),
    );
    out.resize(usize::try_from(text_raw_ptr)?, 0);
    out.extend_from_slice(code);
    out.resize(usize::try_from(data_raw_ptr)?, 0);
    out.extend_from_slice(data);
    out.resize(usize::try_from(idata_raw_ptr)?, 0);
    out.extend_from_slice(idata);
    out.resize(usize::try_from(size_of_file)?, 0);
    Ok(out)
  }
}
