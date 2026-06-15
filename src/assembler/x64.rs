pub(crate) mod disp;
mod encode;
pub(crate) mod inst;
pub(crate) mod ops;
mod pe;
pub(crate) mod register;
pub(crate) mod rm;
pub(crate) mod section;
mod sizeof;
use crate::prelude::*;
pub(crate) struct X64Assembler {
  dlls: Vec<DyLib>,
  handlers: Handlers,
  labels: HashMap<u32, (Section, u32)>,
  root_id: LabelId,
  rva: [u32; NUMBER_OF_SECTIONS as usize],
}
impl X64Assembler {
  pub(crate) fn assemble(
    mut self,
    insts: &[Vec<Vec<X64Inst>>],
    data_insts: Vec<(LabelId, GlobalData)>,
    file: &str,
    mut seh: Seh,
  ) -> ErrOR<()> {
    self.labels.clear();
    let mut data = vec![];
    let mut rdata = vec![];
    let mut bss_v_size: u32 = 0;
    for (lbl, data_inst) in data_insts {
      self.enc_data_lbl(lbl, data_inst, &mut data, &mut rdata, &mut bss_v_size)?;
    }
    let sizes = self.resolve_text_labels(insts)?;
    let text_size: u32 = sizes.iter().sum();
    seh.retain(|seh_elem| self.labels.contains_key(&seh_elem.0));
    self.rva[TextX as usize] = SECTION_ALIGN;
    let (mut pdata, stack_sizes) = self.build_pdata(&mut seh)?;
    let base_h = SectionHeader::from(TextX, HEADERS_SIZE, 0, r_size(HEADERS_SIZE)?, 0);
    let text_h = base_h.next(TextX, text_size)?;
    let data_h = text_h.next(DataX, len_u32(&data)?)?;
    let rdata_h = data_h.next(RData, len_u32(&rdata)?)?;
    let pdata_h = rdata_h.next(PData, len_u32(&pdata)?)?;
    let xdata_v_addr = pdata_h.next_v_addr()?;
    let xdata = self.build_xdata(xdata_v_addr, &mut pdata, &stack_sizes)?;
    let xdata_h = pdata_h.next(XData, len_u32(&xdata)?)?;
    let bss_h = SectionHeader::from(BssX, bss_v_size, xdata_h.next_v_addr()?, 0, 0);
    self.rva[DataX as usize] = data_h.v_addr;
    self.rva[RData as usize] = rdata_h.v_addr;
    self.rva[PData as usize] = pdata_h.v_addr;
    self.rva[XData as usize] = xdata_h.v_addr;
    self.rva[BssX as usize] = bss_h.v_addr;
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
    let mut invalid_inst_opt = None;
    for (inst, _size) in insts.iter().flatten().flatten().zip(sizes) {
      let bytes = self.enc_inst(len_u32(&text)?, inst)?;
      let actual = len_u32(&bytes)?;
      if actual != _size {
        invalid_inst_opt = Some(format!(
          "{}actual: {actual} != expected: {_size} {inst:?}\n",
          invalid_inst_opt.unwrap_or_default(),
        ));
      }
      text.extend_from_slice(&bytes);
    }
    if let Some(err) = invalid_inst_opt {
      return Err(InvalidInst(err).into());
    }
    self.build_pe(
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
impl X64Assembler {
  pub(crate) fn get_rel(&self, rva: u32, size: u32, inst_size: u32) -> ErrOR<i32> {
    let next_rva = self.rva[TextX as usize] + size + inst_size;
    Ok(i32::try_from(rva)? - i32::try_from(next_rva)?)
  }
  pub(crate) fn get_rva(&self, id: LabelId) -> ErrOR<u32> {
    let Some((sect, offset)) = self.labels.get(&id) else {
      return Err(UnknownLabel.into());
    };
    Ok(self.rva[*sect as usize] + offset)
  }
  pub(crate) fn i_f_rva(&self, (dll_idx, func_idx): Api) -> ErrOR<u32> {
    let mut lookup_off = self.sizeof_idt()?;
    let mut lookup_size = 0;
    for dll in &self.dlls[0..=(dll_idx as usize)] {
      lookup_off += lookup_size;
      lookup_size = sizeof_entry(dll)?;
      lookup_off += lookup_size;
    }
    Ok(self.rva[IData as usize] + lookup_off + func_idx * 8)
  }
  pub(crate) fn insert_lbl(&mut self, id: LabelId, sect: Section, offset: u32) -> ErrOR<()> {
    if self.labels.insert(id, (sect, offset)).is_some() {
      return Err(DuplicateLabel.into());
    }
    Ok(())
  }
  pub(crate) fn new(dlls: Vec<DyLib>, root_id: LabelId, handlers: Handlers) -> Self {
    Self { labels: HashMap::new(), rva: [0; NUMBER_OF_SECTIONS as usize], dlls, root_id, handlers }
  }
  fn resolve_text_labels(&mut self, insts: &[Vec<Vec<X64Inst>>]) -> ErrOR<Vec<u32>> {
    let mut sizes = vec![];
    for inst in insts.iter().flatten().flatten() {
      let size = self.sizeof_inst(inst, None)?;
      sizes.push(size);
    }
    loop {
      self.labels.retain(|_, (section, _)| *section != TextX);
      let mut offset = 0;
      for (inst, size) in insts.iter().flatten().flatten().zip(&sizes) {
        if let LblX(id) = inst {
          self.insert_lbl(*id, TextX, offset)?;
        }
        offset += *size;
      }
      let mut changed = false;
      let mut pos = 0;
      for (i, inst) in insts.iter().flatten().flatten().enumerate() {
        let new_size = self.sizeof_inst(inst, Some(pos))?;
        if new_size != sizes[i] {
          sizes[i] = new_size;
          changed = true;
        }
        pos += sizes[i];
      }
      if !changed {
        return Ok(sizes);
      }
    }
  }
}
impl X64Inst {
  // TODO
  #[expect(dead_code)]
  pub(crate) fn modified_regs(&self) -> Vec<X64Reg> {
    match self {
      Repeat(_, _, kind) => match kind {
        MovS | CmpS => vec![Rcx, Rdi, Rsi],
        StoS | ScaS => vec![Rcx, Rdi],
        LodS => vec![Rax, Rcx, Rsi],
      },
      IDivR(_) => vec![Rax, Rdx],
      Cqo => vec![Rdx],
      BitTest(Bt, ..) => vec![],
      BitTest(_, reg, _) => vec![*reg],
      CMovCc(_, dst, _)
      | Clear(dst)
      | CvtTSd2Si(dst, _)
      | DecR(dst)
      | IMulR2(dst, _)
      | IncR(dst)
      | LeaRM(dst, _)
      | RR(_, Add | Sub | And | Or | Xor, dst, _)
      | RM(_, Add | Sub | And | Or | Xor, dst, _)
      | MR(_, Add | Sub | And | Or | Xor, _, dst)
      | MId(_, Add | Sub | And | Or | Xor, Reg(dst), _)
      | MovSxDRMd(dst, _)
      | Unary(_, _, dst)
      | Pop(dst)
      | SetCc(dst, _)
      | ShiftR(_, dst, _)
      | MovRI(dst, _)
      | MovMId(Reg(dst), _)
      | MovMIb(Reg(dst), _)
      | MovRO(_, dst, _)
      | MovOR(_, Reg(dst), _) => vec![*dst],
      RR(_, Cmp, ..)
      | RM(_, Cmp, ..)
      | MR(_, Cmp, ..)
      | MId(..)
      | TestRR(..)
      | CvtSi2Sd(..)
      | ArithSd(..)
      | SqrtSd(..)
      | DecMd(_)
      | IncMd(_)
      | JCc(..)
      | MovMIb(..)
      | MovMId(..)
      | MovOR(..)
      | MovSdO(..)
      | MovOSd(..)
      | Push(_)
      | Jmp(_)
      | RetX
      | UComISd(..)
      | DF(_) => vec![],
      Call(_) | CallApi(_) | CallApiCheck(_) => vec![Rax, Rcx, Rdx, R8, R9, R10, R11],
      LblX(_) => {
        vec![Rax, Rcx, Rdx, Rbx, Rsp, Rbp, Rsi, Rdi, R8, R9, R10, R11, R12, R13, R14, R15]
      }
    }
  }
}
impl Address {
  pub(crate) fn modrm_sib_disp(self) -> u32 {
    match self {
      Global(_) => 5,
      Local(_, offset) => 1 + Disp::from(offset).sizeof(Rbp as u8),
    }
  }
}
impl X64Assembler {
  pub(crate) fn sizeof_iat(&self) -> ErrOR<u32> {
    self.dlls.iter().map(sizeof_entry).sum()
  }
  pub(crate) fn sizeof_idt(&self) -> ErrOR<u32> {
    Ok((len_u32(&self.dlls)? + 1) * 20)
  }
}
pub(crate) fn r_size(data: u32) -> ErrOR<u32> {
  align_up_u32(data, FILE_ALIGN)
}
pub(crate) fn sizeof_entry(dll: &DyLib) -> ErrOR<u32> {
  Ok((len_u32(&dll.1)? + 1) * 8)
}
