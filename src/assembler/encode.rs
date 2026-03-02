use crate::Operand::{Args, Imm, Mem, Ref, Reg, Sib};
use crate::prelude::*;
impl Assembler {
  fn encode_alias(&mut self, insts: &[Inst], size: u32) -> ErrOR<Vec<u8>> {
    let mut vec = vec![];
    let mut code_size = size;
    for inst in insts {
      let bytes = self.encode_inst(code_size, inst)?;
      code_size += self.sizeof_inst(inst, code_size)?;
      vec.extend(bytes);
    }
    Ok(vec)
  }
  fn encode_branch(&self, opc: Vec<u8>, id: u32, size: u32, inst: &Inst) -> ErrOR<Vec<u8>> {
    let mut vec = opc;
    let inst_size = self.sizeof_inst(inst, size)?;
    vec.extend_from_slice(&self.get_rel(self.get_rva(id)?, size, inst_size)?.to_le_bytes());
    Ok(vec)
  }
  pub(crate) fn encode_data_inst(
    &mut self,
    data_inst: DataInst,
    data: &mut Vec<u8>,
    rdata: &mut Vec<u8>,
    seh: &mut Vec<(u32, u32, u32)>,
    bss_v_size: &mut u32,
  ) -> ErrOR<()> {
    match data_inst {
      BssAlloc(idx, size, align) => {
        *bss_v_size = align_up_32(*bss_v_size, align)?;
        self.labels.insert(idx, (Bss, *bss_v_size));
        *bss_v_size += size;
      }
      Byte(idx, byte) => {
        self.labels.insert(idx, (Data, v_size(data)?));
        data.push(byte);
      }
      Quad(idx, qword) => {
        data.resize(align_up(data.len(), 8)?, 0);
        self.labels.insert(idx, (Data, v_size(data)?));
        data.extend_from_slice(&qword.to_le_bytes());
      }
      Bytes(idx, string) => {
        self.labels.insert(idx, (Rdata, v_size(rdata)?));
        rdata.extend_from_slice(string.as_bytes());
        rdata.push(0x00);
      }
      WChars(id, w_chars) => {
        rdata.resize(align_up(rdata.len(), 2)?, 0);
        self.labels.insert(id, (Rdata, v_size(rdata)?));
        for w_char in w_chars.encode_utf16() {
          rdata.extend_from_slice(&w_char.to_le_bytes());
        }
        rdata.extend_from_slice(&[0; 2]);
      }
      Seh(prologue, epilogue, size) => seh.push((prologue, epilogue, size)),
    }
    Ok(())
  }
  pub(crate) fn encode_inst(&mut self, size: u32, inst: &Inst) -> ErrOR<Vec<u8>> {
    Ok(match inst {
      MovBB(operands) => self.encode_mov_b(size, *operands)?,
      MovDD(operands) => self.encode_mov_d(size, *operands)?,
      MovQQ(operands) => self.encode_mov_q(size, *operands)?,
      AddRR(dst, src) => RM::Reg(*dst).encode(1, vec![0x01], *src),
      SubRR(dst, src) => RM::Reg(*dst).encode(1, vec![0x29], *src),
      LogicRbRb(lo, dst, src) => RM::Reg(src.rb()?).encode(0, vec![*lo as u8], dst.rb()?),
      LogicRR(lo, dst, src) => RM::Reg(*src).encode(1, vec![*lo as u8 + 1], *dst),
      Clear(reg) => RM::Reg(*reg).encode(0, vec![0x31], *reg),
      CallApiNull((dll, func)) => self.encode_alias(
        &[CallApi((*dll, *func)), LogicRR(Test, Rax, Rax), JCc(E, self.win_handler)],
        size,
      )?,
      Custom(bytes) => bytes.to_vec(),
      Push(reg) => reg.encode_plus_reg(&[], 0, 0x50, &[]),
      Pop(reg) => reg.encode_plus_reg(&[], 0, 0x58, &[]),
      MovSxDRMd(reg, addr) => self.rm(*addr, size, inst)?.encode(1, vec![0x63], *reg),
      AddRId(reg, imm) => RM::Reg(*reg).encode_imm(1, vec![0x81], Rax, &imm.to_le_bytes()),
      SubRId(reg, imm) => RM::Reg(*reg).encode_imm(1, vec![0x81], Rbp, &imm.to_le_bytes()),
      CmpRIb(reg, imm) => RM::Reg(*reg).encode_imm(1, vec![0x83], Rdi, &[imm.cast_unsigned()]),
      LeaRM(reg, addr) => self.rm(*addr, size, inst)?.encode(1, vec![0x8D], *reg),
      SetCc(cc, reg) => RM::Reg(*reg).encode(0, two(0x90 + *cc as u8), Rax),
      ShlRIb(reg, imm) => RM::Reg(*reg).encode_imm(1, vec![0xC1], Rsp, &[*imm]),
      ShrRIb(reg, imm) => RM::Reg(*reg).encode_imm(1, vec![0xC1], Rbp, &[*imm]),
      SarRIb(reg, imm) => RM::Reg(*reg).encode_imm(1, vec![0xC1], Rdi, &[*imm]),
      Shl1R(reg) => RM::Reg(*reg).encode(1, vec![0xD1], Rsp),
      Call(id) => self.encode_branch(vec![0xE8], *id, size, inst)?,
      Jmp(id) => {
        if let Some((Text, offset)) = self.labels.get(id)
          && let Ok(disp_b) = i8::try_from(i32::try_from(*offset)? - 2 - i32::try_from(size)?)
          && disp_b.is_negative()
        {
          vec![0xEB, disp_b as u8]
        } else {
          self.encode_branch(vec![0xE9], *id, size, inst)?
        }
      }
      NegRb(reg) => RM::Reg(reg.rb()?).encode(0, vec![0xF6], Rbx),
      NotRb(reg) => RM::Reg(reg.rb()?).encode(0, vec![0xF6], Rdx),
      NegR(reg) => RM::Reg(*reg).encode(1, vec![0xF7], Rbx),
      NotR(reg) => RM::Reg(*reg).encode(1, vec![0xF7], Rdx),
      IDivR(reg) => RM::Reg(*reg).encode(1, vec![0xF7], Rdi),
      IncMd(addr) => self.rm(*addr, size, inst)?.encode(0, vec![0xFF], Rax),
      DecMd(addr) => self.rm(*addr, size, inst)?.encode(0, vec![0xFF], Rcx),
      IncR(reg) => RM::Reg(*reg).encode(1, vec![0xFF], Rax),
      DecR(reg) => RM::Reg(*reg).encode(1, vec![0xFF], Rcx),
      CallApi((dll, func)) => {
        let disp = self.get_rel(self.i_f_rva(*dll, *func)?, size, self.sizeof_inst(inst, size)?)?;
        RM::RipRel(disp).encode(0, vec![0xFF], Rdx)
      }
      MovSdXM(xmm, addr) => self.rm(*addr, size, inst)?.encode_f2(0, two(0x10), *xmm, &[]),
      MovSdMX(addr, xmm) => self.rm(*addr, size, inst)?.encode_f2(0, two(0x11), *xmm, &[]),
      MovSdXRef(xmm, reg) => RM::Base(*reg, Disp::Zero).encode_f2(0, two(0x10), *xmm, &[]),
      MovSdRefX(reg, xmm) => RM::Base(*reg, Disp::Zero).encode_f2(0, two(0x11), *xmm, &[]),
      CvtSi2Sd(xmm, reg) => RM::Reg(*reg).encode_f2(1, two(0x2A), *xmm, &[]),
      CvtTSd2Si(reg, xmm) => RM::Reg(*xmm).encode_f2(1, two(0x2C), *reg, &[]),
      CMovCc(cc, dst, src) => RM::Reg(*src).encode(1, two(0x40 + *cc as u8), *dst),
      SqrtSd(dst, src) => RM::Reg(*dst).encode_f2(0, two(0x51), *src, &[]),
      AddSd(xmm, xmm2) => RM::Reg(*xmm2).encode_f2(0, two(0x58), *xmm, &[]),
      MulSd(xmm, xmm2) => RM::Reg(*xmm2).encode_f2(0, two(0x59), *xmm, &[]),
      SubSd(xmm, xmm2) => RM::Reg(*xmm2).encode_f2(0, two(0x5C), *xmm, &[]),
      DivSd(xmm, xmm2) => RM::Reg(*xmm2).encode_f2(0, two(0x5E), *xmm, &[]),
      JCc(cc, id) => {
        if let Some((Text, offset)) = self.labels.get(id)
          && let Ok(disp_b) = i8::try_from(i32::try_from(*offset)? - 2 - i32::try_from(size)?)
          && disp_b.is_negative()
        {
          vec![0x70 + *cc as u8, disp_b as u8]
        } else {
          self.encode_branch(two(0x80 + *cc as u8), *id, size, inst)?
        }
      }
      IMulRR(dst, src) => RM::Reg(*src).encode(1, two(0xAF), *dst),
      Lbl(_) => vec![],
    })
  }
  fn encode_mov_b(&mut self, size: u32, operands: (Operand<u8>, Operand<u8>)) -> ErrOR<Vec<u8>> {
    Ok(match operands {
      (Reg(dst), Ref(src)) => RM::Base(src.rb()?, Disp::Zero).encode(0, vec![0x8A], dst.rb()?),
      (Ref(dst), Reg(src)) => RM::Base(dst.rb()?, Disp::Zero).encode(0, vec![0x88], src.rb()?),
      (Reg(dst), Reg(src)) => RM::Reg(dst.rb()?).encode(0, vec![0x88], src.rb()?),
      (Reg(dst), Mem(src)) => {
        self.rm(src, size, &MovBB(operands))?.encode(0, vec![0x8A], dst.rb()?)
      }
      (Mem(dst), Reg(src)) => {
        self.rm(dst, size, &MovBB(operands))?.encode(0, vec![0x88], src.rb()?)
      }
      (Mem(addr), Imm(imm)) => {
        self.rm(addr, size, &MovBB(operands))?.encode_imm(0, vec![0xC6], Rax, &[imm])
      }
      (Reg(dst), Imm(imm)) => dst.rb()?.encode_plus_reg(&[], 0, 0xB0, &[imm]),
      _ => return Err(Internal(InvalidInst(format!("MovBB{operands:?}")))),
    })
  }
  fn encode_mov_d(&mut self, size: u32, operands: (Operand<u32>, Operand<u32>)) -> ErrOR<Vec<u8>> {
    Ok(match operands {
      (Reg(dst), Ref(src)) => RM::Base(src, Disp::Zero).encode(0, vec![0x8B], dst),
      (Ref(dst), Reg(src)) => RM::Base(dst, Disp::Zero).encode(0, vec![0x89], src),
      (Reg(dst), Reg(src)) => RM::Reg(dst).encode(0, vec![0x89], src),
      (Reg(dst), Mem(src)) => self.rm(src, size, &MovDD(operands))?.encode(0, vec![0x8B], dst),
      (Mem(dst), Reg(src)) => self.rm(dst, size, &MovDD(operands))?.encode(0, vec![0x89], src),
      (Mem(addr), Imm(imm)) => {
        self.rm(addr, size, &MovDD(operands))?.encode_imm(0, vec![0xC7], Rax, &imm.to_le_bytes())
      }
      (Reg(dst), Imm(imm)) => dst.encode_plus_reg(&[], 0, 0xB8, &imm.to_le_bytes()),
      (Sib(sib, disp), Reg(src)) => RM::Sib(sib, disp).encode(0, vec![0x89], src),
      _ => return Err(Internal(InvalidInst(format!("MovDD{operands:?}")))),
    })
  }
  fn encode_mov_q(&mut self, size: u32, operands: (Operand<u64>, Operand<u64>)) -> ErrOR<Vec<u8>> {
    Ok(match operands {
      (Reg(dst), Args(nth)) => RM::Base(Rsp, Disp::from((nth - 1) * 8)).encode(1, vec![0x8B], dst),
      (Args(nth), Reg(src)) => RM::Base(Rsp, Disp::from((nth - 1) * 8)).encode(1, vec![0x89], src),
      (Reg(dst), Ref(src)) => RM::Base(src, Disp::Zero).encode(1, vec![0x8B], dst),
      (Ref(dst), Reg(src)) => RM::Base(dst, Disp::Zero).encode(1, vec![0x89], src),
      (Reg(dst), Reg(src)) => RM::Reg(dst).encode(1, vec![0x89], src),
      (Reg(dst), Mem(src)) => self.rm(src, size, &MovQQ(operands))?.encode(1, vec![0x8B], dst),
      (Mem(dst), Reg(src)) => self.rm(dst, size, &MovQQ(operands))?.encode(1, vec![0x89], src),
      (Reg(dst), Imm(imm)) => dst.encode_plus_reg(&[], 1, 0xB8, &imm.to_le_bytes()),
      (Sib(sib, disp), Reg(src)) => RM::Sib(sib, disp).encode(1, vec![0x89], src),
      _ => return Err(Internal(InvalidInst(format!("MovQQ{operands:?}")))),
    })
  }
  fn rm(&self, addr: Address, text_size: u32, inst: &Inst) -> ErrOR<RM> {
    let inst_size = self.sizeof_inst(inst, text_size)?;
    Ok(match addr {
      Global(id) => RM::RipRel(self.get_rel(self.get_rva(id)?, text_size, inst_size)?),
      Local(_, offset) => RM::Base(Rbp, Disp::from(offset)),
    })
  }
}
fn two(opc: u8) -> Vec<u8> {
  vec![0x0F, opc]
}
