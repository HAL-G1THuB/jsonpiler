use crate::Operand::{Args, Imm, Mem, Ref, Reg, Sib};
use crate::prelude::*;
impl Assembler {
  fn sizeof_alias(&self, insts: &[Inst], size: u32) -> ErrOR<u32> {
    let mut code_size = 0;
    for inst in insts {
      code_size += self.sizeof_inst(inst, size + code_size)?;
    }
    Ok(code_size)
  }
  pub(crate) fn sizeof_iat(&self) -> ErrOR<u32> {
    let mut iat_size = 0;
    for dll in &self.dlls {
      iat_size += sizeof_entry(dll)?;
    }
    Ok(iat_size)
  }
  pub(crate) fn sizeof_idt(&self) -> ErrOR<u32> {
    Ok(u32::try_from((self.dlls.len() + 1) * 20)?)
  }
  pub(crate) fn sizeof_inst(&self, inst: &Inst, size: u32) -> ErrOR<u32> {
    Ok(match inst {
      Custom(bytes) => v_size(bytes)?,
      NegR(_) | NotR(_) | LogicRR(..) | IncR(_) | DecR(_) | Shl1R(_) | IDivR(_) | SubRR(..)
      | AddRR(..) => 3,
      CMovCc(..) | SarRIb(..) | ShrRIb(..) | ShlRIb(..) | IMulRR(..) | CmpRIb(..) => 4,
      CvtSi2Sd(..) | CvtTSd2Si(..) | Call(_) => 5,
      CallApi(_) => 6,
      SubRId(..) | AddRId(..) => 7,
      JCc(_, id) => {
        if let Some((Text, offset)) = self.labels.get(id)
          && i8::try_from(i32::try_from(*offset)? - 2 - i32::try_from(size)?)
            .is_ok_and(i8::is_negative)
        {
          2
        } else {
          6
        }
      }
      Jmp(id) => {
        if let Some((Text, offset)) = self.labels.get(id)
          && i8::try_from(i32::try_from(*offset)? - 2 - i32::try_from(size)?)
            .is_ok_and(i8::is_negative)
        {
          2
        } else {
          5
        }
      }
      CallApiNull(api) => self
        .sizeof_alias(&[CallApi(*api), LogicRR(Test, Rax, Rax), JCc(E, self.win_handler)], size)?,
      AddSd(x, x2) | SubSd(x, x2) | MulSd(x, x2) | DivSd(x, x2) | SqrtSd(x, x2) => {
        1 + u32::from(x.rex() | x2.rex()) + 2 + 1
      }
      MovSdXRef(xmm, reg) | MovSdRefX(reg, xmm) => {
        1 + u32::from(reg.rex() | xmm.rex()) + 2 + 1 + Disp::Zero.sizeof(reg.reg_bits())
      }
      MovSxDRMd(_, addr) | LeaRM(_, addr) => 2 + addr.modrm_sib_disp(),
      MovSdXM(xmm, addr) | MovSdMX(addr, xmm) => {
        1 + u32::from(xmm.rex()) + 2 + addr.modrm_sib_disp()
      }
      IncMd(addr) | DecMd(addr) => 1 + addr.modrm_sib_disp(),
      NegRb(reg) | NotRb(reg) | Clear(reg) => u32::from(reg.rex()) + 2,
      MovBB(operands) => sizeof_mov_b(*operands)?,
      MovQQ(operands) => sizeof_mov_q(*operands)?,
      MovDD(operands) => sizeof_mov_d(*operands)?,
      LogicRbRb(_, dst, src) => u32::from(dst.rex() | src.rex()) + 2,
      Pop(reg) | Push(reg) => u32::from(reg.rex()) + 1,
      SetCc(_, reg) => u32::from(reg.rex()) + 3,
      Lbl(_) => 0,
    })
  }
}
pub(crate) fn sizeof_entry(dll: &Dll) -> ErrOR<u32> {
  Ok(u32::try_from((dll.1.len() + 1) * 8)?)
}
pub(crate) fn sizeof_mov_q(operands: (Operand<u64>, Operand<u64>)) -> ErrOR<u32> {
  Ok(match operands {
    (Reg(_), Reg(_)) => 1 + 2,
    (Reg(_), Ref(reg)) | (Ref(reg), Reg(_)) => 1 + 2 + u32::from(matches!(reg, Rsp | R12)),
    (Reg(_), Mem(addr)) | (Mem(addr), Reg(_)) => 1 + 1 + addr.modrm_sib_disp(),
    (Reg(_), Args(nth)) | (Args(nth), Reg(_)) => 4 + Disp::from((nth - 1) << 3).sizeof(Rsp as u8),
    (Reg(_), Imm(_)) => 1 + 1 + 8,
    (Sib(sib, disp), Reg(_)) => 1 + 3 + disp.sizeof(sib.base.reg_bits()),
    _ => return Err(Internal(InvalidInst(format!("MovQQ{operands:?}")))),
  })
}
pub(crate) fn sizeof_mov_b(operands: (Operand<u8>, Operand<u8>)) -> ErrOR<u32> {
  Ok(match operands {
    (Reg(r2), Ref(r1)) | (Ref(r1), Reg(r2)) => {
      u32::from(r1.rex() | r2.rex()) + 2 + Disp::Zero.sizeof(r1.reg_bits())
    }
    (Reg(r1), Reg(r2)) => u32::from(r1.rex() | r2.rex()) + 2,
    (Reg(reg), Mem(addr)) | (Mem(addr), Reg(reg)) => {
      u32::from(reg.rex()) + 1 + addr.modrm_sib_disp()
    }
    (Mem(addr), Imm(_)) => 1 + addr.modrm_sib_disp() + 1,
    (Reg(reg), Imm(_)) => u32::from(reg.rex()) + 1 + 1,
    _ => return Err(Internal(InvalidInst(format!("MovBB{operands:?}")))),
  })
}
pub(crate) fn sizeof_mov_d(operands: (Operand<u32>, Operand<u32>)) -> ErrOR<u32> {
  Ok(match operands {
    (Reg(r1), Reg(r2)) => u32::from(r1.rex() | r2.rex()) + 2,
    (Reg(r2), Ref(r1)) | (Ref(r1), Reg(r2)) => {
      u32::from(r1.rex() | r2.rex()) + 2 + Disp::Zero.sizeof(r1.reg_bits())
    }
    (Reg(reg), Mem(addr)) | (Mem(addr), Reg(reg)) => {
      u32::from(reg.rex()) + 1 + addr.modrm_sib_disp()
    }
    (Mem(addr), Imm(_)) => 1 + addr.modrm_sib_disp() + 4,
    (Reg(reg), Imm(_)) => u32::from(reg.rex()) + 1 + 4,
    (Sib(sib, disp), Reg(src)) => {
      u32::from(sib.base.rex() | sib.index.rex() | src.rex()) + 3 + disp.sizeof(sib.base.reg_bits())
    }
    _ => return Err(Internal(InvalidInst(format!("MovDD{operands:?}")))),
  })
}
