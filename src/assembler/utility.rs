use crate::prelude::*;
impl Assembler {
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
  pub(crate) fn new(dlls: Vec<Dll>, root_id: LabelId, handlers: Handlers) -> Self {
    Self { labels: HashMap::new(), rva: [0; NUMBER_OF_SECTIONS as usize], dlls, root_id, handlers }
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
impl Address {
  pub(crate) fn modrm_sib_disp(self) -> u32 {
    match self {
      Global(_) => 5,
      Local(_, offset) => 1 + Disp::from(offset).sizeof(Rbp as u8),
    }
  }
}
impl Assembler {
  pub(crate) fn sizeof_iat(&self) -> ErrOR<u32> {
    self.dlls.iter().map(sizeof_entry).sum()
  }
  pub(crate) fn sizeof_idt(&self) -> ErrOR<u32> {
    Ok((len_u32(&self.dlls)? + 1) * 20)
  }
}
pub(crate) fn r_size(data: u32) -> ErrOR<u32> {
  align_up_u32(data, FILE_ALIGNMENT)
}
pub(crate) fn sizeof_entry(dll: &Dll) -> ErrOR<u32> {
  Ok((len_u32(&dll.1)? + 1) * 8)
}
pub(crate) fn align_up(num: usize, align: usize) -> ErrOR<usize> {
  num.div_ceil(align).checked_mul(align).ok_or(Internal(InternalOverFlow))
}
