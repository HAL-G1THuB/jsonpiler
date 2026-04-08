pub(crate) mod disp;
mod encode;
mod pe;
pub(crate) mod register;
pub(crate) mod rm;
pub(crate) mod sect_header;
mod sizeof;
use crate::prelude::*;
use sizeof::*;
pub(crate) type Api = (u32, u32);
pub(crate) struct Assembler {
  dlls: Vec<Dll>,
  labels: HashMap<u32, (Section, u32)>,
  root_id: LabelId,
  rva: [u32; NUMBER_OF_SECTIONS as usize],
  seh_handler: LabelId,
  win_handler: LabelId,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scale {
  S1 = 0,
  #[expect(dead_code)]
  S2 = 1,
  S4 = 2,
  #[expect(dead_code)]
  S8 = 3,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(clippy::arbitrary_source_item_ordering)]
pub(crate) struct Sib {
  pub scale: Scale,
  pub index: Register,
  pub base: Register,
}
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
#[expect(clippy::min_ident_chars)]
pub(crate) enum ConditionCode {
  O = 0,
  #[expect(dead_code)]
  No = 1,
  B = 2,
  Ae = 3,
  E = 4,
  Ne = 5,
  Be = 6,
  A = 7,
  #[expect(dead_code)]
  S = 8,
  #[expect(dead_code)]
  Ns = 9,
  #[expect(dead_code)]
  P = 10,
  #[expect(dead_code)]
  Np = 11,
  L = 12,
  Ge = 13,
  Le = 14,
  G = 15,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Logic {
  And = 0x22,
  Or = 0x0A,
  Xor = 0x32,
  Cmp = 0x3A,
  Test = 0x84,
}
#[derive(Clone, Debug)]
pub(crate) enum DataLbl {
  BssLbl(LabelId, u32, u32),
  Byte(LabelId, u8),
  Quad(LabelId, u64),
  StrLbl(LabelId, String),
  WStrLbl(LabelId, String),
}
#[must_use]
#[derive(Copy, Clone, Debug)]
pub(crate) enum Inst {
  AddRId(Register, i32),
  AddRR(Register, Register),
  ArithSd(ArithSdKind, Register, Register),
  CMovCc(ConditionCode, Register, Register),
  Call(LabelId),
  CallApi(Api),
  CallApiCheck(Api),
  Clear(Register),
  Custom(&'static [u8]),
  CvtSi2Sd(Register, Register),
  CvtTSd2Si(Register, Register),
  DecMd(Address),
  DecR(Register),
  IDivR(Register),
  IMulRR(Register, Register),
  IncMd(Address),
  IncR(Register),
  JCc(ConditionCode, LabelId),
  Jmp(LabelId),
  Lbl(LabelId),
  LeaRM(Register, Address),
  LogicRR(Logic, Register, Register),
  LogicRbRb(Logic, Register, Register),
  MovBB((Operand<u8>, Operand<u8>)),
  MovDD((Operand<u32>, Operand<u32>)),
  MovMSd(Address, Register),
  MovQQ((Operand<u64>, Operand<u64>)),
  MovRefSd(Register, Register),
  MovSdM(Register, Address),
  MovSdRef(Register, Register),
  MovSxDRMd(Register, Address),
  Pop(Register),
  Push(Register),
  SetCc(Register, ConditionCode),
  ShiftR(ShiftDirection, Register, Shift),
  SqrtSd(Register, Register),
  SubRId(Register, i32),
  SubRR(Register, Register),
  UComISd(Register, Register),
  UnaryR(UnaryKind, Register),
  UnaryRb(UnaryKind, Register),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnaryKind {
  Neg,
  Not,
}
impl UnaryKind {
  pub(crate) fn reg_field(self) -> Register {
    match self {
      Neg => Rbx,
      Not => Rdx,
    }
  }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArithSdKind {
  Add = 0x58,
  Div = 0x5E,
  Mul = 0x59,
  Sub = 0x5C,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operand<T> {
  Args(i32),
  Imm(T),
  Mem(Address),
  Ref(Register),
  Reg(Register),
  SibDisp(Sib, Disp),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShiftDirection {
  #[expect(dead_code)]
  Sar,
  Shl,
  Shr,
}
impl ShiftDirection {
  pub(crate) fn reg_field(self) -> Register {
    match self {
      Shl => Rsp,
      Shr => Rbp,
      Sar => Rdi,
    }
  }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shift {
  Cl,
  Ib(u8),
  One,
}
impl Shift {
  pub(crate) fn imm(self) -> Vec<u8> {
    match self {
      Shift::Cl | Shift::One => vec![],
      Shift::Ib(imm) => vec![imm],
    }
  }
  pub(crate) fn opcode(self) -> u8 {
    match self {
      Shift::Cl => 0xD3,
      Shift::One => 0xD1,
      Shift::Ib(_) => 0xC1,
    }
  }
}
impl Assembler {
  pub(crate) fn assemble(
    mut self,
    insts: &[Inst],
    data_insts: Vec<DataLbl>,
    file: &str,
    mut seh: Vec<(LabelId, LabelId, i32)>,
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
    for inst in insts {
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
    for inst in insts {
      text.extend_from_slice(&self.encode_inst(len_u32(&text)?, inst)?);
    }
    #[expect(clippy::print_stderr, clippy::use_debug)]
    #[cfg(debug_assertions)]
    {
      let mut is_invalid_inst = false;
      for (inst, size) in insts.iter().zip(validate_vec) {
        let bytes = self.encode_inst(len_u32(&text)?, inst)?;
        if len_u32(&bytes)? != size {
          is_invalid_inst = true;
          eprintln!(
            "{INTERNAL_ERR}\n| actual: {} != expected: {size} {inst:?}{ERR_END}",
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
  pub(crate) fn get_rel(&self, rva: u32, size: u32, inst_size: u32) -> ErrOR<i32> {
    let next_rva = self.rva[Text as usize] + size + inst_size;
    Ok(i32::try_from(rva)? - i32::try_from(next_rva)?)
  }
  pub(crate) fn get_rva(&self, id: LabelId) -> ErrOR<u32> {
    let (sect, offset) = self.labels.get(&id).ok_or(Internal(UnknownLabel))?;
    Ok(self.rva[*sect as usize] + offset)
  }
  pub(crate) fn i_f_rva(&self, (dll_idx, func_idx): Api) -> ErrOR<u32> {
    let mut lookup_offset = self.sizeof_idt()?;
    let mut lookup_size = 0;
    for dll in &self.dlls[0..=(dll_idx as usize)] {
      lookup_offset += lookup_size;
      lookup_size = sizeof_entry(dll)?;
      lookup_offset += lookup_size;
    }
    Ok(self.rva[IData as usize] + lookup_offset + func_idx * 8)
  }
  pub(crate) fn new(
    dlls: Vec<Dll>,
    root_id: LabelId,
    win_handler: LabelId,
    seh_handler: LabelId,
  ) -> Self {
    Self {
      labels: HashMap::new(),
      rva: [0; NUMBER_OF_SECTIONS as usize],
      dlls,
      root_id,
      win_handler,
      seh_handler,
    }
  }
}
impl Inst {
  // TODO
  #[expect(dead_code)]
  pub(crate) fn modified_regs(&self) -> Vec<Register> {
    match self {
      IDivR(_) => vec![Rax, Rdx],
      AddRR(dst, _)
      | CMovCc(_, dst, _)
      | Clear(dst)
      | CvtTSd2Si(dst, _)
      | DecR(dst)
      | IMulRR(dst, _)
      | IncR(dst)
      | LeaRM(dst, _)
      | LogicRR(And | Or | Xor, dst, _)
      | LogicRbRb(And | Or | Xor, dst, _)
      | MovSxDRMd(dst, _)
      | UnaryR(_, dst)
      | UnaryRb(_, dst)
      | Pop(dst)
      | SetCc(dst, _)
      | ShiftR(_, dst, _)
      | AddRId(dst, _)
      | SubRId(dst, _)
      | SubRR(dst, _)
      | MovBB((Reg(dst), _))
      | MovDD((Reg(dst), _))
      | MovQQ((Reg(dst), _)) => vec![*dst],
      LogicRR(Cmp | Test, ..)
      | LogicRbRb(Cmp | Test, ..)
      | CvtSi2Sd(..)
      | ArithSd(..)
      | SqrtSd(..)
      | DecMd(_)
      | IncMd(_)
      | JCc(..)
      | MovBB(_)
      | MovDD(_)
      | MovQQ(_)
      | MovMSd(..)
      | MovRefSd(..)
      | MovSdM(..)
      | MovSdRef(..)
      | Push(_)
      | Jmp(_)
      | UComISd(..) => vec![],
      Call(_) | CallApi(_) | CallApiCheck(_) => vec![Rax, Rcx, Rdx, R8, R9, R10, R11],
      Custom(_) | Lbl(_) => {
        vec![Rax, Rcx, Rdx, Rbx, Rsp, Rbp, Rsi, Rdi, R8, R9, R10, R11, R12, R13, R14, R15]
      }
    }
  }
}
