pub(crate) mod disp;
mod encode;
pub(crate) mod inst;
pub(crate) mod ops;
mod pe;
pub(crate) mod register;
pub(crate) mod rm;
pub(crate) mod section;
mod sizeof;
mod utility;
use self::utility::*;
use crate::prelude::*;
pub(crate) struct Assembler {
  dlls: Vec<Dll>,
  handlers: Handlers,
  labels: HashMap<u32, (Section, u32)>,
  root_id: LabelId,
  rva: [u32; NUMBER_OF_SECTIONS as usize],
}
impl Assembler {
  pub(crate) fn assemble(
    mut self,
    insts: &[Vec<Inst>],
    data_insts: Vec<DataLbl>,
    file: &str,
    mut seh: Seh,
  ) -> ErrOR<()> {
    self.labels.clear();
    let mut text_size: u32 = 0;
    let mut data = vec![];
    let mut rdata = vec![];
    let mut bss_v_size: u32 = 0;
    for data_inst in data_insts {
      self.encode_data_lbl(data_inst, &mut data, &mut rdata, &mut bss_v_size)?;
    }
    #[cfg(debug_assertions)]
    let mut validate_vec = vec![];
    for inst in insts.iter().flatten() {
      if let Lbl(idx) = inst
        && self.labels.insert(*idx, (Text, text_size)).is_some()
      {
        return Err(Internal(DuplicateLabel));
      }
      let inst_size = self.sizeof_inst(inst, text_size)?;
      text_size += inst_size;
      #[cfg(debug_assertions)]
      validate_vec.push(inst_size);
    }
    seh.retain(|seh_elem| self.labels.contains_key(&seh_elem.0));
    self.rva[Text as usize] = SECTION_ALIGNMENT;
    let (mut pdata, stack_sizes) = self.build_pdata(&mut seh)?;
    let base_h = SectionHeader::from(Text, HEADERS_SIZE, 0, r_size(HEADERS_SIZE)?, 0);
    let text_h = base_h.next(Text, text_size)?;
    let data_h = text_h.next(Data, len_u32(&data)?)?;
    let rdata_h = data_h.next(RData, len_u32(&rdata)?)?;
    let pdata_h = rdata_h.next(PData, len_u32(&pdata)?)?;
    let xdata_v_addr = pdata_h.next_v_addr()?;
    let xdata = self.build_xdata(xdata_v_addr, &mut pdata, &stack_sizes)?;
    let xdata_h = pdata_h.next(XData, len_u32(&xdata)?)?;
    let bss_h = SectionHeader::from(Bss, bss_v_size, xdata_h.next_v_addr()?, 0, 0);
    self.rva[Data as usize] = data_h.v_addr;
    self.rva[RData as usize] = rdata_h.v_addr;
    self.rva[PData as usize] = pdata_h.v_addr;
    self.rva[XData as usize] = xdata_h.v_addr;
    self.rva[Bss as usize] = bss_h.v_addr;
    self.rva[IData as usize] = bss_h.next_v_addr()?;
    let idata = self.build_idata()?;
    let idata_h = SectionHeader::from(
      IData,
      len_u32(&idata)?,
      bss_h.next_v_addr()?,
      r_size(len_u32(&idata)?)?,
      xdata_h.next_r_ptr(),
    );
    let mut text = vec![];
    #[cfg(not(debug_assertions))]
    for inst in insts.iter().flatten() {
      text.extend_from_slice(&self.encode_inst(len_u32(&text)?, inst)?);
    }
    #[expect(clippy::print_stderr, clippy::use_debug)]
    #[cfg(debug_assertions)]
    {
      let mut is_invalid_inst = false;
      for (inst, size) in insts.iter().flatten().zip(validate_vec) {
        let bytes = self.encode_inst(len_u32(&text)?, inst)?;
        if len_u32(&bytes)? != size {
          is_invalid_inst = true;
          eprintln!(
            "{}\n| actual: {} != expected: {size} {inst:?}{ERR_END}",
            make_header(INTERNAL_ERR),
            len_u32(&bytes)?,
          );
        }
        text.extend_from_slice(&bytes);
      }
      if is_invalid_inst {
        eprintln!("{ISSUE}INST_SIZE`");
      }
    };
    self.link(
      &[
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
}
