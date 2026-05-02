use crate::prelude::*;
impl Assembler {
  fn sizeof_alias(&self, insts: &[Inst], size: u32) -> ErrOR<u32> {
    let mut code_size = 0;
    for inst in insts {
      code_size += self.sizeof_inst(inst, size + code_size)?;
    }
    Ok(code_size)
  }
  pub(crate) fn sizeof_inst(&self, inst: &Inst, size: u32) -> ErrOR<u32> {
    Ok(match inst {
      ShiftR(_, _, operand) => {
        let imm_size = len_u32(operand.imm().as_slice())?;
        3 + imm_size
      }
      Custom(bytes) => len_u32(bytes)?,
      UnaryR(..) | LogicRR(..) | IncR(_) | DecR(_) | IDivR(_) | SubRR(..) | AddRR(..) => 3,
      CMovCc(..) | IMulRR(..) => 4,
      CvtSi2Sd(..) | CvtTSd2Si(..) | Call(_) => 5,
      CallApi(_) => 6,
      SubRId(..) | AddRId(..) => 7,
      JCc(_, id) => self.sizeof_jmp(*id, size, 6)?,
      Jmp(id) => self.sizeof_jmp(*id, size, 5)?,
      CallApiCheck(api) => self
        .sizeof_alias(&[CallApi(*api), LogicRR(Test, Rax, Rax), JCc(E, self.handlers.win)], size)?,
      UComISd(xmm, xmm2) | ArithSd(_, xmm, xmm2) | SqrtSd(xmm, xmm2) => {
        1 + (xmm.rex_size() | xmm2.rex_size()) + 2 + 1
      }
      MovSdRef(xmm, reg) | MovRefSd(reg, xmm) => {
        1 + (reg.rex_size() | xmm.rex_size()) + 2 + 1 + Disp::Zero.sizeof(reg.reg_bits())
      }
      MovSxDRMd(_, addr) | LeaRM(_, addr) => 2 + addr.modrm_sib_disp(),
      MovSdM(xmm, addr) | MovMSd(addr, xmm) => 1 + xmm.rex_size() + 2 + addr.modrm_sib_disp(),
      IncMd(addr) | DecMd(addr) => 1 + addr.modrm_sib_disp(),
      UnaryRb(_, reg) | Clear(reg) => reg.rex_size() + 2,
      MovBB(operands) => sizeof_mov_b(*operands)?,
      MovQQ(operands) => sizeof_mov_q(*operands)?,
      MovDD(operands) => sizeof_mov_d(*operands)?,
      LogicRbRb(_, dst, src) => (dst.rex_size() | src.rex_size()) + 2,
      Pop(reg) | Push(reg) => reg.rex_size() + 1,
      SetCc(reg, _) => reg.rex_size() + 3,
      Lbl(_) => 0,
    })
  }
  fn sizeof_jmp(&self, id: LabelId, size: u32, long: u32) -> ErrOR<u32> {
    if let Some((Text, offset)) = self.labels.get(&id)
      && i8::try_from(i32::try_from(*offset)? - 2 - i32::try_from(size)?).is_ok_and(i8::is_negative)
    {
      Ok(2)
    } else {
      Ok(long)
    }
  }
}
pub(crate) fn sizeof_mov_q(operands: (Operand<u64>, Operand<u64>)) -> ErrOR<u32> {
  Ok(match operands {
    (Reg(_), Reg(_)) => 1 + 2,
    (Reg(_), Ref(reg)) | (Ref(reg), Reg(_)) => 1 + 2 + u32::from(reg.reg_bits() == Rsp as u8),
    (Reg(_), Mem(addr)) | (Mem(addr), Reg(_)) => 1 + 1 + addr.modrm_sib_disp(),
    (Reg(_), Args(nth)) | (Args(nth), Reg(_)) => 4 + Disp::from((nth - 1) << 3).sizeof(Rsp as u8),
    (Reg(_), Imm(_)) => 1 + 1 + 8,
    (SibDisp(sib, disp), Reg(_)) => 1 + 3 + disp.sizeof(sib.base.reg_bits()),
    _ => return Err(Internal(InvalidInst(format!("MovQQ{operands:?}")))),
  })
}
pub(crate) fn sizeof_mov_b(operands: (Operand<u8>, Operand<u8>)) -> ErrOR<u32> {
  Ok(match operands {
    (Reg(reg), Ref(mem)) | (Ref(mem), Reg(reg)) => {
      (reg.rex_size() | mem.rex_size()) + 2 + Disp::Zero.sizeof(mem.reg_bits())
    }
    (Reg(dst), Reg(src)) => (dst.rex_size() | src.rex_size()) + 2,
    (Reg(reg), Mem(addr)) | (Mem(addr), Reg(reg)) => reg.rex_size() + 1 + addr.modrm_sib_disp(),
    (Mem(addr), Imm(_)) => 1 + addr.modrm_sib_disp() + 1,
    (Reg(reg), Imm(_)) => reg.rex_size() + 1 + 1,
    (Reg(dst), SibDisp(sib, disp)) => {
      (sib.base.rex_size() | sib.index.rex_size() | dst.rex_size())
        + 3
        + disp.sizeof(sib.base.reg_bits())
    }
    _ => return Err(Internal(InvalidInst(format!("MovBB{operands:?}")))),
  })
}
pub(crate) fn sizeof_mov_d(operands: (Operand<u32>, Operand<u32>)) -> ErrOR<u32> {
  Ok(match operands {
    (Reg(dst), Reg(src)) => (dst.rex_size() | src.rex_size()) + 2,
    (Reg(reg), Ref(mem)) | (Ref(mem), Reg(reg)) => {
      (reg.rex_size() | mem.rex_size()) + 2 + Disp::Zero.sizeof(mem.reg_bits())
    }
    (Reg(reg), Mem(addr)) | (Mem(addr), Reg(reg)) => reg.rex_size() + 1 + addr.modrm_sib_disp(),
    (Mem(addr), Imm(_)) => 1 + addr.modrm_sib_disp() + 4,
    (Reg(reg), Imm(_)) => reg.rex_size() + 1 + 4,
    (SibDisp(sib, disp), Reg(src)) => {
      (sib.base.rex_size() | sib.index.rex_size() | src.rex_size())
        + 3
        + disp.sizeof(sib.base.reg_bits())
    }
    _ => return Err(Internal(InvalidInst(format!("MovDD{operands:?}")))),
  })
}
