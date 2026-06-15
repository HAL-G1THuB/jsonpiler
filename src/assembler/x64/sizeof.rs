use crate::prelude::*;
impl X64Assembler {
  fn sizeof_alias(&self, insts: &[X64Inst], offset: Option<u32>) -> ErrOR<u32> {
    let mut code_size = 0;
    for inst in insts {
      code_size += self.sizeof_inst(inst, offset.map(|off| off + code_size))?;
    }
    Ok(code_size)
  }
  pub(crate) fn sizeof_inst(&self, inst: &X64Inst, offset: Option<u32>) -> ErrOR<u32> {
    Ok(match inst {
      Repeat(reg_size, _, _) => 1 + u32::from(*reg_size == S8) + 1,
      ShiftR(_, _, operand) => 3 + len_u32(&operand.imm())?,
      IncR(_) | DecR(_) | IDivR(_) => 3,
      CMovCc(..) | IMulR2(..) => 4,
      BitTest(..) | CvtSi2Sd(..) | CvtTSd2Si(..) | Call(_) => 5,
      CallApi(_) => 6,
      JCc(_, id) => self.sizeof_jmp(*id, offset, 6)?,
      Jmp(id) => self.sizeof_jmp(*id, offset, 5)?,
      CallApiCheck(api) => {
        self.sizeof_alias(&[CallApi(*api), TestRR(S8, Rax), JCc(E, self.handlers.os)], offset)?
      }
      UComISd(xmm, xmm2) | ArithSd(_, xmm, xmm2) | SqrtSd(xmm, xmm2) => {
        1 + (xmm.rex_size() | xmm2.rex_size()) + 2 + 1
      }
      MovSxDRMd(_, addr) | LeaRM(_, addr) => 2 + addr.modrm_sib_disp(),
      MovSdO(xmm, operand) => 1 + operand.sizeof(xmm.rex_size(), 2)?,
      MovOSd(operand, xmm) => 1 + operand.sizeof(xmm.rex_size(), 2)?,
      IncMd(addr) | DecMd(addr) => 1 + addr.modrm_sib_disp(),
      Clear(reg) => reg.rex_size() + 2,
      Unary(reg_size, _, reg) => match reg_size {
        S8 => 3,
        S4 | S1 => reg.rex_size() + 2,
      },
      MovRI(..) => 1 + 1 + 8,
      MovMId(operand, _) => sizeof_mov_id(*operand)?,
      MovMIb(operand, _) => sizeof_mov_ib(*operand)?,
      MovRO(reg_size, dst, src) => {
        let operands = (Reg(*dst), *src);
        match reg_size {
          S1 => sizeof_mov_b(operands)?,
          S4 | S8 => sizeof_mov_dq(*reg_size, operands)?,
        }
      }
      MovOR(reg_size, dst, src) => {
        let operands = (*dst, Reg(*src));
        match reg_size {
          S1 => sizeof_mov_b(operands)?,
          S4 | S8 => sizeof_mov_dq(*reg_size, operands)?,
        }
      }
      RR(reg_size, _, dst, src) => {
        (u32::from(reg_size.rex_w()) | dst.rex_size() | src.rex_size()) + 2
      }
      MR(reg_size, _, operand, reg) | RM(reg_size, _, reg, operand) => {
        operand.sizeof(u32::from(reg_size.rex_w()) | reg.rex_size(), 1)?
      }
      TestRR(reg_size, reg) => (u32::from(reg_size.rex_w()) | reg.rex_size()) + 2,
      Pop(reg) | Push(reg) => reg.rex_size() + 1,
      SetCc(reg, _) => reg.rex_size() + 3,
      RetX | DF(_) => 1,
      Cqo => 2,
      MId(reg_size, _, operand, _) => {
        if *reg_size == S1 {
          return Err(InvalidInst(format!("OId S1 {operand:?}")).into());
        }
        operand.sizeof(u32::from(reg_size.rex_w()), 1)? + 4
      }
      LblX(_) => 0,
    })
  }
  pub(crate) fn sizeof_jmp(&self, id: LabelId, offset: Option<u32>, long: u32) -> ErrOR<u32> {
    if let Some((TextX, lbl_off)) = self.labels.get(&id)
      && let Some(off) = offset
      && i8::try_from(i32::try_from(*lbl_off)? - 2 - i32::try_from(off)?).is_ok()
    {
      Ok(2)
    } else {
      Ok(long)
    }
  }
}
pub(crate) fn sizeof_mov_id(operand: X64Operand) -> ErrOR<u32> {
  Ok(if let Reg(reg) = operand { reg.rb()?.rex_size() + 1 } else { operand.sizeof(0, 1)? } + 4)
}
pub(crate) fn sizeof_mov_ib(operand: X64Operand) -> ErrOR<u32> {
  Ok(if let Reg(reg) = operand { reg.rb()?.rex_size() + 1 } else { operand.sizeof(0, 1)? } + 1)
}
pub(crate) fn sizeof_mov_b(operands: (X64Operand, X64Operand)) -> ErrOR<u32> {
  Ok(match operands {
    (Reg(dst), src) => src.rb()?.sizeof(dst.rb()?.rex_size(), 1)?,
    (dst, Reg(src)) => dst.rb()?.sizeof(src.rb()?.rex_size(), 1)?,
    _ => return Err(InvalidInst(format!("MovBB{operands:?}")).into()),
  })
}
pub(crate) fn sizeof_mov_dq(reg_size: RegSize, operands: (X64Operand, X64Operand)) -> ErrOR<u32> {
  let rex_w = u32::from(reg_size.rex_w());
  Ok(match operands {
    (Reg(dst), src) => src.sizeof(rex_w | dst.rex_size(), 1)?,
    (dst, Reg(src)) => dst.sizeof(rex_w | src.rex_size(), 1)?,
    _ => return Err(InvalidInst(format!("MovDD{operands:?}")).into()),
  })
}
