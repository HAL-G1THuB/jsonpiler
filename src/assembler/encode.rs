use super::utility::*;
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
  fn encode_branch(&self, opc: &[u8], id: LabelId, size: u32, inst: &Inst) -> ErrOR<Vec<u8>> {
    let mut vec = opc.to_vec();
    let inst_size = self.sizeof_inst(inst, size)?;
    vec.extend_from_slice(&self.get_rel(self.get_rva(id)?, size, inst_size)?.to_le_bytes());
    Ok(vec)
  }
  pub(crate) fn encode_data_lbl(
    &mut self,
    data_inst: DataLbl,
    data: &mut Vec<u8>,
    rdata: &mut Vec<u8>,
    bss_v_size: &mut u32,
  ) -> ErrOR<()> {
    match data_inst {
      BssLbl(idx, size, align) => {
        *bss_v_size = align_up_u32(*bss_v_size, align)?;
        self.labels.insert(idx, (Bss, *bss_v_size));
        *bss_v_size += size;
      }
      Byte(idx, byte) => {
        self.labels.insert(idx, (Data, len_u32(data)?));
        data.push(byte);
      }
      Quad(idx, qword) => {
        data.resize(align_up(data.len(), 8)?, 0);
        self.labels.insert(idx, (Data, len_u32(data)?));
        data.extend_from_slice(&qword.to_le_bytes());
      }
      StrLbl(idx, string) => {
        self.labels.insert(idx, (RData, len_u32(rdata)?));
        rdata.extend_from_slice(string.as_bytes());
        rdata.push(0x00);
      }
      WStrLbl(id, w_chars) => {
        rdata.resize(align_up(rdata.len(), 2)?, 0);
        self.labels.insert(id, (RData, len_u32(rdata)?));
        for w_char in w_chars.encode_utf16() {
          rdata.extend_from_slice(&w_char.to_le_bytes());
        }
        rdata.extend_from_slice(&[0; 2]);
      }
    }
    Ok(())
  }
  pub(crate) fn encode_inst(&mut self, size: u32, inst: &Inst) -> ErrOR<Vec<u8>> {
    Ok(match inst {
      ShiftR(direction, reg, operand) => RM::Reg(*reg).encode_imm(
        1,
        &[operand.opcode()],
        direction.reg_field(),
        operand.imm().as_slice(),
      ),
      MovBB(operands) => self.encode_mov_b(size, *operands)?,
      MovDD(operands) => self.encode_mov_d(size, *operands)?,
      MovQQ(operands) => self.encode_mov_q(size, *operands)?,
      AddRR(dst, src) => RM::Reg(*dst).encode(1, &[0x01], *src),
      SubRR(dst, src) => RM::Reg(*dst).encode(1, &[0x29], *src),
      LogicRbRb(lo, dst, src) => RM::Reg(src.rb()?).encode(0, &[*lo as u8], dst.rb()?),
      LogicRR(lo, dst, src) => RM::Reg(*src).encode(1, &[*lo as u8 + 1], *dst),
      Clear(reg) => RM::Reg(*reg).encode(0, &[0x31], *reg),
      CallApiCheck((dll, func)) => self.encode_alias(
        &[CallApi((*dll, *func)), LogicRR(Test, Rax, Rax), JCc(E, self.handlers.win)],
        size,
      )?,
      Custom(bytes) => bytes.to_vec(),
      Push(reg) => reg.encode_plus_reg(&[], 0, 0x50, &[]),
      Pop(reg) => reg.encode_plus_reg(&[], 0, 0x58, &[]),
      MovSxDRMd(reg, addr) => self.rm(*addr, size, inst)?.encode(1, &[0x63], *reg),
      AddRId(reg, imm) => RM::Reg(*reg).encode_imm(1, &[0x81], Rax, &imm.to_le_bytes()),
      SubRId(reg, imm) => RM::Reg(*reg).encode_imm(1, &[0x81], Rbp, &imm.to_le_bytes()),
      LeaRM(reg, addr) => self.rm(*addr, size, inst)?.encode(1, &[0x8D], *reg),
      SetCc(reg, cc) => RM::Reg(*reg).encode(0, &two(0x90 + *cc as u8), Rax),
      Call(id) => self.encode_branch(&[0xE8], *id, size, inst)?,
      Jmp(id) => self.encode_jmp(*id, size, inst, 0xEB, &[0xE9])?,
      IDivR(reg) => RM::Reg(*reg).encode(1, &[0xF7], Rdi),
      IncMd(addr) => self.rm(*addr, size, inst)?.encode(0, &[0xFF], Rax),
      DecMd(addr) => self.rm(*addr, size, inst)?.encode(0, &[0xFF], Rcx),
      IncR(reg) => RM::Reg(*reg).encode(1, &[0xFF], Rax),
      DecR(reg) => RM::Reg(*reg).encode(1, &[0xFF], Rcx),
      CallApi(api) => {
        let disp = self.get_rel(self.i_f_rva(*api)?, size, self.sizeof_inst(inst, size)?)?;
        RM::RipRel(disp).encode(0, &[0xFF], Rdx)
      }
      MovSdM(xmm, addr) => self.rm(*addr, size, inst)?.encode_ex(0xF2, 0, &two(0x10), *xmm, &[]),
      MovMSd(addr, xmm) => self.rm(*addr, size, inst)?.encode_ex(0xF2, 0, &two(0x11), *xmm, &[]),
      MovSdRef(xmm, reg) => RM::Base(*reg, Disp::Zero).encode_ex(0xF2, 0, &two(0x10), *xmm, &[]),
      MovRefSd(reg, xmm) => RM::Base(*reg, Disp::Zero).encode_ex(0xF2, 0, &two(0x11), *xmm, &[]),
      CvtSi2Sd(xmm, reg) => RM::Reg(*reg).encode_ex(0xF2, 1, &two(0x2A), *xmm, &[]),
      CvtTSd2Si(reg, xmm) => RM::Reg(*xmm).encode_ex(0xF2, 1, &two(0x2C), *reg, &[]),
      CMovCc(cc, dst, src) => RM::Reg(*src).encode(1, &two(0x40 + *cc as u8), *dst),
      SqrtSd(dst, src) => RM::Reg(*dst).encode_ex(0xF2, 0, &two(0x51), *src, &[]),
      ArithSd(kind, xmm, xmm2) => RM::Reg(*xmm2).encode_ex(0xF2, 0, &two(*kind as u8), *xmm, &[]),
      UComISd(xmm, xmm2) => RM::Reg(*xmm2).encode_ex(0x66, 0, &two(0x2E), *xmm, &[]),
      JCc(cc, id) => self.encode_jmp(*id, size, inst, 0x70 + *cc as u8, &two(0x80 + *cc as u8))?,
      IMulRR(dst, src) => RM::Reg(*src).encode(1, &two(0xAF), *dst),
      UnaryR(kind, reg) => RM::Reg(*reg).encode(1, &[0xF7], kind.reg_field()),
      UnaryRb(kind, reg) => RM::Reg(reg.rb()?).encode(0, &[0xF6], kind.reg_field()),
      Lbl(_) => vec![],
    })
  }
  fn encode_jmp(
    &self,
    id: LabelId,
    size: u32,
    inst: &Inst,
    short: u8,
    long: &[u8],
  ) -> ErrOR<Vec<u8>> {
    if let Some((Text, offset)) = self.labels.get(&id)
      && let Ok(disp_b) = i8::try_from(i32::try_from(*offset)? - 2 - i32::try_from(size)?)
      && disp_b.is_negative()
    {
      Ok(vec![short, disp_b.cast_unsigned()])
    } else {
      self.encode_branch(long, id, size, inst)
    }
  }
  fn encode_mov_b(&mut self, size: u32, operands: (Operand<u8>, Operand<u8>)) -> ErrOR<Vec<u8>> {
    Ok(match operands {
      (Reg(dst), Ref(src)) => RM::Base(src.rb()?, Disp::Zero).encode(0, &[0x8A], dst.rb()?),
      (Ref(dst), Reg(src)) => RM::Base(dst.rb()?, Disp::Zero).encode(0, &[0x88], src.rb()?),
      (Reg(dst), Reg(src)) => RM::Reg(dst.rb()?).encode(0, &[0x88], src.rb()?),
      (Reg(dst), Mem(src)) => self.rm(src, size, &MovBB(operands))?.encode(0, &[0x8A], dst.rb()?),
      (Reg(dst), SibDisp(sib, disp)) => RM::Sib(sib, disp).encode(0, &[0x8A], dst.rb()?),
      (Mem(dst), Reg(src)) => self.rm(dst, size, &MovBB(operands))?.encode(0, &[0x88], src.rb()?),
      (Mem(addr), Imm(imm)) => {
        self.rm(addr, size, &MovBB(operands))?.encode_imm(0, &[0xC6], Rax, &[imm])
      }
      (Reg(dst), Imm(imm)) => dst.rb()?.encode_plus_reg(&[], 0, 0xB0, &[imm]),
      _ => return Err(Internal(InvalidInst(format!("MovBB{operands:?}")))),
    })
  }
  fn encode_mov_d(&mut self, size: u32, operands: (Operand<u32>, Operand<u32>)) -> ErrOR<Vec<u8>> {
    Ok(match operands {
      (Reg(dst), Ref(src)) => RM::Base(src, Disp::Zero).encode(0, &[0x8B], dst),
      (Ref(dst), Reg(src)) => RM::Base(dst, Disp::Zero).encode(0, &[0x89], src),
      (Reg(dst), Reg(src)) => RM::Reg(dst).encode(0, &[0x89], src),
      (Reg(dst), Mem(src)) => self.rm(src, size, &MovDD(operands))?.encode(0, &[0x8B], dst),
      (Mem(dst), Reg(src)) => self.rm(dst, size, &MovDD(operands))?.encode(0, &[0x89], src),
      (Mem(addr), Imm(imm)) => {
        self.rm(addr, size, &MovDD(operands))?.encode_imm(0, &[0xC7], Rax, &imm.to_le_bytes())
      }
      (Reg(dst), Imm(imm)) => dst.encode_plus_reg(&[], 0, 0xB8, &imm.to_le_bytes()),
      (SibDisp(sib, disp), Reg(src)) => RM::Sib(sib, disp).encode(0, &[0x89], src),
      _ => return Err(Internal(InvalidInst(format!("MovDD{operands:?}")))),
    })
  }
  fn encode_mov_q(&mut self, size: u32, operands: (Operand<u64>, Operand<u64>)) -> ErrOR<Vec<u8>> {
    Ok(match operands {
      (Reg(dst), Args(nth)) => RM::Base(Rsp, Disp::from((nth - 1) * 8)).encode(1, &[0x8B], dst),
      (Args(nth), Reg(src)) => RM::Base(Rsp, Disp::from((nth - 1) * 8)).encode(1, &[0x89], src),
      (Reg(dst), Ref(src)) => RM::Base(src, Disp::Zero).encode(1, &[0x8B], dst),
      (Ref(dst), Reg(src)) => RM::Base(dst, Disp::Zero).encode(1, &[0x89], src),
      (Reg(dst), Reg(src)) => RM::Reg(dst).encode(1, &[0x89], src),
      (Reg(dst), Mem(src)) => self.rm(src, size, &MovQQ(operands))?.encode(1, &[0x8B], dst),
      (Mem(dst), Reg(src)) => self.rm(dst, size, &MovQQ(operands))?.encode(1, &[0x89], src),
      (Reg(dst), Imm(imm)) => dst.encode_plus_reg(&[], 1, 0xB8, &imm.to_le_bytes()),
      (SibDisp(sib, disp), Reg(src)) => RM::Sib(sib, disp).encode(1, &[0x89], src),
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
fn two(opc: u8) -> [u8; 2] {
  [0x0F, opc]
}
