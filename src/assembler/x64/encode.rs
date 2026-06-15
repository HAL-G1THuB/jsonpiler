use crate::prelude::*;
impl X64Assembler {
  fn enc_alias(&mut self, insts: &[X64Inst], mut offset: u32) -> ErrOR<Vec<u8>> {
    let mut vec = vec![];
    for inst in insts {
      let bytes = self.enc_inst(offset, inst)?;
      offset += len_u32(&bytes)?;
      vec.extend_from_slice(&bytes);
    }
    Ok(vec)
  }
  fn enc_branch(&self, opc: &[u8], id: LabelId, offset: u32, inst: &X64Inst) -> ErrOR<Vec<u8>> {
    let mut vec = opc.to_vec();
    let inst_size = self.sizeof_inst(inst, Some(offset))?;
    vec.extend_from_slice(&self.get_rel(self.get_rva(id)?, offset, inst_size)?.to_le_bytes());
    Ok(vec)
  }
  pub(crate) fn enc_data_lbl(
    &mut self,
    id: LabelId,
    data_inst: GlobalData,
    data: &mut Vec<u8>,
    rdata: &mut Vec<u8>,
    bss_v_size: &mut u32,
  ) -> ErrOR<()> {
    match data_inst {
      Zeros(size, align) => {
        *bss_v_size = align_up_u32(*bss_v_size, align)?;
        self.insert_lbl(id, BssX, *bss_v_size)?;
        *bss_v_size += size;
      }
      Byte(byte) => {
        self.insert_lbl(id, DataX, len_u32(data)?)?;
        data.push(byte);
      }
      Quad(qword) => {
        data.resize(align_up(data.len(), 8)?, 0);
        self.insert_lbl(id, DataX, len_u32(data)?)?;
        data.extend_from_slice(&qword.to_le_bytes());
      }
      Chars(string) => {
        self.insert_lbl(id, RData, len_u32(rdata)?)?;
        rdata.extend_from_slice(string.as_bytes());
        rdata.push(0x00);
      }
      WChars(w_chars) => {
        rdata.resize(align_up(rdata.len(), 2)?, 0);
        self.insert_lbl(id, RData, len_u32(rdata)?)?;
        for w_char in w_chars.encode_utf16() {
          rdata.extend_from_slice(&w_char.to_le_bytes());
        }
        rdata.extend_from_slice(&[0; 2]);
      }
    }
    Ok(())
  }
  pub(crate) fn enc_inst(&mut self, offset: u32, inst: &X64Inst) -> ErrOR<Vec<u8>> {
    Ok(match inst {
      Repeat(reg_size, rep, kind) => {
        let prefix = *rep as u8;
        let mut opc = *kind as u8;
        if *reg_size == S4 || *reg_size == S8 {
          opc += 1;
        }
        let mut bytes = vec![prefix];
        if *reg_size == S8 {
          bytes.push(0x48);
        }
        bytes.push(opc);
        bytes
      }
      ShiftR(dir, reg, operand) => {
        if let Shift::Ib(imm) = operand
          && *imm >= 64
        {
          return Err(InvalidInst(format!("Shift immediate out of range: {imm}")).into());
        }
        ModRM::Reg(*reg).enc_imm(1, &[operand.opcode()], dir.reg_field(), operand.imm().as_slice())
      }
      MId(reg_size, group, operand, imm) => {
        if *reg_size == S1 {
          return Err(InvalidInst(format!("OId S1 {operand:?}")).into());
        }
        self.o2rm(*operand, offset, inst)?.enc_imm(
          reg_size.rex_w(),
          &[0x81],
          group.reg_field(),
          &imm.to_le_bytes(),
        )
      }
      MovMIb(operand, ib) => self.encode_mov_ib(offset, *operand, *ib)?,
      MovMId(operand, id) => self.encode_mov_id(offset, *operand, *id)?,
      MovRI(dst, imm) => dst.encode_plus_reg(&[], 1, 0xB8, &imm.to_le_bytes()),
      MovRO(reg_size, dst, src) => match reg_size {
        S1 => self.o2rm(src.rb()?, offset, &MovRO(S1, *dst, *src))?.enc(0, &[0x8A], dst.rb()?),
        S4 | S8 => {
          let rm = self.o2rm(*src, offset, &MovRO(*reg_size, *dst, *src))?;
          rm.enc(reg_size.rex_w(), &[0x8B], *dst)
        }
      },
      MovOR(reg_size, dst, src) => match reg_size {
        S1 => self.o2rm(dst.rb()?, offset, &MovOR(S1, *dst, *src))?.enc(0, &[0x88], src.rb()?),
        S4 | S8 => {
          let rm = self.o2rm(*dst, offset, &MovOR(*reg_size, *dst, *src))?;
          rm.enc(reg_size.rex_w(), &[0x89], *src)
        }
      },
      RR(reg_size, lo, dst, src) => match reg_size {
        S1 => ModRM::Reg(dst.rb()?).enc(0, &[*lo as u8], src.rb()?),
        S4 | S8 => ModRM::Reg(*dst).enc(reg_size.rex_w(), &[*lo as u8 + 1], *src),
      },
      MR(reg_size, lo, operand, src) => match reg_size {
        S1 => {
          let rm = self.o2rm(operand.rb()?, offset, &MR(S1, *lo, *operand, *src))?;
          rm.enc(0, &[*lo as u8], src.rb()?)
        }
        S4 | S8 => {
          let rm = self.o2rm(*operand, offset, &MR(*reg_size, *lo, *operand, *src))?;
          rm.enc(reg_size.rex_w(), &[*lo as u8 + 1], *src)
        }
      },
      RM(reg_size, lo, dst, operand) => match reg_size {
        S1 => {
          let rm = self.o2rm(operand.rb()?, offset, &RM(S1, *lo, *dst, *operand))?;
          rm.enc(0, &[*lo as u8 + 2], dst.rb()?)
        }
        S4 | S8 => {
          let rm = self.o2rm(*operand, offset, &RM(*reg_size, *lo, *dst, *operand))?;
          rm.enc(reg_size.rex_w(), &[*lo as u8 + 3], *dst)
        }
      },
      TestRR(reg_size, reg) => match reg_size {
        S1 => ModRM::Reg(reg.rb()?).enc(0, &[0x84], reg.rb()?),
        S4 | S8 => ModRM::Reg(*reg).enc(reg_size.rex_w(), &[0x85], *reg),
      },
      Clear(reg) => ModRM::Reg(*reg).enc(0, &[0x31], *reg),
      CallApiCheck((dll, func)) => self
        .enc_alias(&[CallApi((*dll, *func)), TestRR(S8, Rax), JCc(E, self.handlers.os)], offset)?,
      Push(reg) => reg.encode_plus_reg(&[], 0, 0x50, &[]),
      Pop(reg) => reg.encode_plus_reg(&[], 0, 0x58, &[]),
      MovSxDRMd(reg, addr) => self.mem2rm(*addr, offset, inst)?.enc(1, &[0x63], *reg),
      LeaRM(reg, addr) => self.mem2rm(*addr, offset, inst)?.enc(1, &[0x8D], *reg),
      SetCc(reg, cc) => ModRM::Reg(*reg).enc(0, &two(0x90 + *cc as u8), Rax),
      Call(id) => self.enc_branch(&[0xE8], *id, offset, inst)?,
      Jmp(id) => self.enc_jmp(*id, offset, inst, 0xEB, &[0xE9])?,
      IDivR(reg) => ModRM::Reg(*reg).enc(1, &[0xF7], Rdi),
      IncMd(addr) => self.mem2rm(*addr, offset, inst)?.enc(0, &[0xFF], Rax),
      DecMd(addr) => self.mem2rm(*addr, offset, inst)?.enc(0, &[0xFF], Rcx),
      IncR(reg) => ModRM::Reg(*reg).enc(1, &[0xFF], Rax),
      DecR(reg) => ModRM::Reg(*reg).enc(1, &[0xFF], Rcx),
      CallApi(api) => {
        let inst_size = self.sizeof_inst(inst, Some(offset))?;
        let disp = self.get_rel(self.i_f_rva(*api)?, offset, inst_size)?;
        ModRM::RipRel(disp).enc(0, &[0xFF], Rdx)
      }
      MovSdO(xmm, src) => self.o2rm(*src, offset, inst)?.enc_ex(0xF2, 0, &two(0x10), *xmm, &[]),
      MovOSd(dst, xmm) => self.o2rm(*dst, offset, inst)?.enc_ex(0xF2, 0, &two(0x11), *xmm, &[]),
      CvtSi2Sd(xmm, reg) => ModRM::Reg(*reg).enc_ex(0xF2, 1, &two(0x2A), *xmm, &[]),
      CvtTSd2Si(reg, xmm) => ModRM::Reg(*xmm).enc_ex(0xF2, 1, &two(0x2C), *reg, &[]),
      CMovCc(cc, dst, src) => ModRM::Reg(*src).enc(1, &two(0x40 + *cc as u8), *dst),
      SqrtSd(dst, src) => ModRM::Reg(*dst).enc_ex(0xF2, 0, &two(0x51), *src, &[]),
      ArithSd(kind, xmm, xmm2) => ModRM::Reg(*xmm2).enc_ex(0xF2, 0, &two(*kind as u8), *xmm, &[]),
      UComISd(xmm, xmm2) => ModRM::Reg(*xmm2).enc_ex(0x66, 0, &two(0x2E), *xmm, &[]),
      JCc(cc, id) => self.enc_jmp(*id, offset, inst, 0x70 + *cc as u8, &two(0x80 + *cc as u8))?,
      IMulR2(dst, src) => ModRM::Reg(*src).enc(1, &two(0xAF), *dst),
      Unary(reg_size, kind, reg) => match reg_size {
        S1 => ModRM::Reg(reg.rb()?).enc(0, &[0xF6], kind.reg_field()),
        S4 | S8 => ModRM::Reg(*reg).enc(reg_size.rex_w(), &[0xF7], kind.reg_field()),
      },
      RetX => vec![0xC3],
      Cqo => vec![0x48, 0x99],
      LblX(_) => vec![],
      BitTest(bt_kind, reg, idx) => {
        if *idx >= 64 {
          return Err(InvalidInst(format!("Bit index out of range for BT: {idx}")).into());
        }
        ModRM::Reg(*reg).enc_imm(1, &[0x0F, 0xBA], bt_kind.reg_field(), &[*idx])
      }
      DF(set) => vec![0xFC + u8::from(*set)],
    })
  }
  fn enc_jmp(
    &self,
    id: LabelId,
    offset: u32,
    inst: &X64Inst,
    short: u8,
    long: &[u8],
  ) -> ErrOR<Vec<u8>> {
    if let Some((TextX, lbl_off)) = self.labels.get(&id)
      && let Ok(disp_b) = i8::try_from(i32::try_from(*lbl_off)? - 2 - i32::try_from(offset)?)
    {
      Ok(vec![short, disp_b.cast_unsigned()])
    } else {
      self.enc_branch(long, id, offset, inst)
    }
  }
  fn encode_mov_ib(&mut self, offset: u32, operand: X64Operand, ib: u8) -> ErrOR<Vec<u8>> {
    Ok(match operand {
      Reg(dst) => dst.rb()?.encode_plus_reg(&[], 0, 0xB0, &[ib]),
      Mem(_) | Ref(_) | SibDisp(..) | Args(_) => {
        self.o2rm(operand, offset, &MovMIb(operand, ib))?.enc_imm(0, &[0xC6], Rax, &[ib])
      }
    })
  }
  fn encode_mov_id(&mut self, offset: u32, operand: X64Operand, id: u32) -> ErrOR<Vec<u8>> {
    let bytes = &id.to_le_bytes();
    Ok(match operand {
      Reg(dst) => dst.encode_plus_reg(&[], 0, 0xB8, bytes),
      Mem(_) | Ref(_) | SibDisp(..) | Args(_) => {
        self.o2rm(operand, offset, &MovMId(operand, id))?.enc_imm(0, &[0xC7], Rax, bytes)
      }
    })
  }
  pub(crate) fn mem2rm(&self, addr: Address, offset: u32, inst: &X64Inst) -> ErrOR<ModRM> {
    Ok(match addr {
      Global(id) => {
        let inst_size = self.sizeof_inst(inst, Some(offset))?;
        ModRM::RipRel(self.get_rel(self.get_rva(id)?, offset, inst_size)?)
      }
      Local(_, off) => ModRM::Base(Rbp, Disp::from(off)),
    })
  }
}
fn two(opc: u8) -> [u8; 2] {
  [0x0F, opc]
}
