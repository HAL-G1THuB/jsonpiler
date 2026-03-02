pub(crate) mod disp;
mod encode;
mod pe;
pub(crate) mod register;
pub(crate) mod rm;
pub(crate) mod sect_header;
mod sizeof;
use crate::prelude::*;
use sizeof::*;
pub(crate) struct Assembler {
  dlls: Vec<Dll>,
  labels: HashMap<u32, (Sect, u32)>,
  rva: [u32; NUMBER_OF_SECTIONS as usize],
  win_handler: u32,
}
impl Assembler {
  pub(crate) fn assemble(
    mut self,
    insts: &[Inst],
    data_insts: Vec<DataInst>,
    seh_handler: u32,
    file: &str,
  ) -> ErrOR<()> {
    self.labels.clear();
    let mut text_size: u32 = 0;
    let mut data = vec![];
    let mut rdata = vec![];
    let mut seh = vec![];
    let mut bss_v_size: u32 = 0;
    for data_inst in data_insts {
      self.encode_data_inst(data_inst, &mut data, &mut rdata, &mut seh, &mut bss_v_size)?;
    }
    #[cfg(debug_assertions)]
    let mut validate_vec = vec![];
    for inst in insts {
      if let Lbl(idx) = inst {
        self.labels.insert(*idx, (Text, text_size));
      }
      let inst_size = self.sizeof_inst(inst, text_size)?;
      text_size += inst_size;
      #[cfg(debug_assertions)]
      validate_vec.push(inst_size);
    }
    self.rva[Text as usize] = SECTION_ALIGNMENT;
    let (mut pdata, stack_sizes) = self.build_pdata(&mut seh)?;
    let pe_h = SectHeader::from([0; 8], HEADERS_V_SIZE, 0, r_size(HEADERS_V_SIZE)?, 0, 0);
    let text_h = SectHeader::from_prev(*b".text\0\0\0", text_size, &pe_h, TEXT_C)?;
    let data_h = SectHeader::from_prev(*b".data\0\0\0", v_size(&data)?, &text_h, DATA_C)?;
    let rdata_h = SectHeader::from_prev(*b".rdata\0\0", v_size(&rdata)?, &data_h, N_DATA_C)?;
    let pdata_h = SectHeader::from_prev(*b".pdata\0\0", v_size(&pdata)?, &rdata_h, N_DATA_C)?;
    let xdata_v_addr = pdata_h.next_v_addr()?;
    let xdata = self.build_xdata(xdata_v_addr, &mut pdata, &stack_sizes, seh_handler)?;
    let xdata_h = SectHeader::from_prev(*b".xdata\0\0", v_size(&xdata)?, &pdata_h, N_DATA_C)?;
    let bss_h = SectHeader::from(*b".bss\0\0\0\0", bss_v_size, xdata_h.next_v_addr()?, 0, 0, BSS_C);
    self.rva[Data as usize] = data_h.v_addr;
    self.rva[Rdata as usize] = rdata_h.v_addr;
    self.rva[Pdata as usize] = pdata_h.v_addr;
    self.rva[Xdata as usize] = xdata_h.v_addr;
    self.rva[Bss as usize] = bss_h.v_addr;
    self.rva[Idata as usize] = bss_h.next_v_addr()?;
    let idata = self.build_idata()?;
    let idata_h = SectHeader::from(
      *b".idata\0\0",
      v_size(&idata)?,
      bss_h.next_v_addr()?,
      r_size(v_size(&idata)?)?,
      xdata_h.next_r_ptr(),
      N_DATA_C,
    );
    let mut text = vec![];
    #[cfg(not(debug_assertions))]
    for inst in insts {
      text.extend_from_slice(&self.encode_inst(v_size(&text)?, inst)?);
    }
    #[expect(clippy::print_stdout, clippy::use_debug)]
    #[cfg(debug_assertions)]
    {
      let mut is_invalid_inst = false;
      for (inst, size) in insts.iter().zip(validate_vec) {
        let bytes = self.encode_inst(v_size(&text)?, inst)?;
        if v_size(&bytes)? != size {
          is_invalid_inst = true;
          println!(
            "{INTERNAL_ERROR}actual: {} != expected: {size} {inst:?}{ERR_END}",
            v_size(&bytes)?,
          );
        }
        text.extend_from_slice(&bytes);
      }
      if is_invalid_inst {
        println!("{REPORT_MSG}INST_SIZE`");
      }
    };
    self.link(
      [
        (text, text_h),
        (data, data_h),
        (rdata, rdata_h),
        (pdata, pdata_h),
        (xdata, xdata_h),
        (vec![], bss_h),
        (idata, idata_h),
      ],
      file,
    )
  }
  pub(crate) fn from(dlls: Vec<Dll>, win_handler: u32) -> Self {
    Self { labels: HashMap::new(), rva: [0; NUMBER_OF_SECTIONS as usize], dlls, win_handler }
  }
  pub(crate) fn get_rel(&self, rva: u32, size: u32, inst_size: u32) -> ErrOR<i32> {
    let next_rva = self.rva[Text as usize] + size + inst_size;
    Ok(i32::try_from(rva)? - i32::try_from(next_rva)?)
  }
  pub(crate) fn get_rva(&self, id: u32) -> ErrOR<u32> {
    let (sect, offset) = self.labels.get(&id).ok_or(Internal(UnknownLabel))?;
    Ok(self.rva[*sect as usize] + offset)
  }
  pub(crate) fn i_f_rva(&self, dll_idx: u32, func_idx: u32) -> ErrOR<u32> {
    let mut lookup_offset = self.sizeof_idt()?;
    let mut lookup_size = 0;
    for dll in &self.dlls[0..=usize::try_from(dll_idx)?] {
      lookup_offset += lookup_size;
      lookup_size = sizeof_entry(dll)?;
      lookup_offset += lookup_size;
    }
    Ok(self.rva[Idata as usize] + lookup_offset + func_idx * 8)
  }
}
