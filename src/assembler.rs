use crate::{
  Assembler,
  DataInst::{self, *},
  Disp, Dlls, ErrOR,
  Inst::{self, *},
  InternalErrKind::*,
  JsonpilerErr::*,
  Memory::{self, Global, GlobalD, Local, Tmp},
  Operand::{self, *},
  RM,
  Register::{self, *},
  Sect, Sib,
  utility::{align_up, align_up_32, mov_q},
};
use core::iter::Chain;
use core::slice::Iter;
use std::collections::HashMap;
impl Assembler {
  fn add_label(&mut self, sect: Sect, idx: u32, offset: u32) {
    self.sym_addr.insert(idx, offset);
    self.addr_sect.insert(idx, sect);
  }
  pub(crate) fn assemble_and_link(
    mut self, insts: Chain<Iter<Inst>, Iter<Inst>>, data_insts: Vec<DataInst>, seh_handler: u32,
  ) -> ErrOR<Vec<u8>> {
    self.sym_addr.clear();
    self.addr_sect.clear();
    let mut text: u32 = 0;
    let mut data = vec![];
    let mut rdata = vec![];
    let mut seh = vec![];
    let mut stack_size = vec![];
    let mut pdata = vec![];
    let mut xdata = vec![];
    let mut bss: u32 = 0;
    for data_inst in data_insts {
      match data_inst {
        Bss(idx, size, align) => {
          bss = align_up_32(bss, align)?;
          self.add_label(Sect::Bss, idx, bss);
          bss += size;
        }
        Byte(idx, byte) => {
          self.add_label(Sect::Data, idx, to_rva(&data)?);
          data.push(byte);
        }
        Quad(idx, qword) => {
          data.resize(align_up(data.len(), 8)?, 0);
          self.add_label(Sect::Data, idx, to_rva(&data)?);
          data.extend_from_slice(&qword.to_le_bytes());
        }
        Bytes(idx, string) => {
          self.add_label(Sect::Rdata, idx, to_rva(&rdata)?);
          rdata.extend_from_slice(string.as_bytes());
          rdata.push(0x00);
        }
        RDAlign(align) => rdata.resize(align_up(rdata.len(), align)?, 0),
        Seh(prologue, epilogue, size) => seh.push((prologue, epilogue, size)),
      }
    }
    #[cfg(debug_assertions)]
    let mut validate_vec = vec![];
    for inst in insts.clone() {
      let inst_len = self.inst_size(inst, text)?;
      text += inst_len;
      #[cfg(debug_assertions)]
      validate_vec.push(inst_len);
    }
    self.rva.insert(Sect::Text, 0x1000);
    seh.sort_by(|lhs, rhs| self.sym_addr[&lhs.0].cmp(&self.sym_addr[&rhs.0]));
    for (prologue, epilogue, size) in seh {
      pdata.extend_from_slice(&(self.sym_addr[&prologue] + self.rva[&Sect::Text]).to_le_bytes());
      pdata.extend_from_slice(&(self.sym_addr[&epilogue] + self.rva[&Sect::Text]).to_le_bytes());
      pdata.extend_from_slice(&[0; 4]);
      stack_size.push(size);
    }
    self.rva.insert(Sect::Data, 0x1000 + align_up_32(text, 0x1000)?);
    let data_raw_size = align_up_32(to_rva(&data)?, 0x1000)?;
    self.rva.insert(Sect::Rdata, self.rva[&Sect::Data] + data_raw_size);
    let rdata_raw_size = align_up_32(to_rva(&data)?, 0x1000)?;
    self.rva.insert(Sect::Pdata, self.rva[&Sect::Rdata] + rdata_raw_size);
    let pdata_raw_size = align_up_32(to_rva(&pdata)?, 0x1000)?;
    self.rva.insert(Sect::Xdata, self.rva[&Sect::Pdata] + pdata_raw_size);
    #[expect(clippy::cast_possible_truncation)]
    for (idx, size) in stack_size.iter().enumerate() {
      xdata.resize(align_up(xdata.len(), 4)?, 0);
      pdata[idx * 12 + 8..idx * 12 + 12]
        .copy_from_slice(&(to_rva(&xdata)? + self.rva[&Sect::Xdata]).to_le_bytes());
      let push_offset = self.inst_size(&Push(Rbp), 0)? as u8;
      let mov_offset = push_offset + self.inst_size(&mov_q(Rbp, Rsp), 0)? as u8;
      let sub_offset = mov_offset + self.inst_size(&SubRId(Rsp, *size), 0)? as u8;
      xdata.extend_from_slice(&[9, sub_offset, 4, Rbp as u8, sub_offset, 1]);
      xdata.extend_from_slice(&(((*size + 7) >> 3) as u16).to_le_bytes());
      xdata.extend_from_slice(&[mov_offset, 3, push_offset, (Rbp as u8) << 4u8]);
      xdata.extend_from_slice(&(self.sym_addr[&seh_handler] + self.rva[&Sect::Text]).to_le_bytes());
    }
    let xdata_raw_size = align_up_32(to_rva(&xdata)?, 0x1000)?;
    self.rva.insert(Sect::Bss, self.rva[&Sect::Xdata] + xdata_raw_size);
    self.rva.insert(Sect::Idata, self.rva[&Sect::Bss] + align_up_32(bss, 0x1000)?);
    let idata = self.build_idata_section(self.rva[&Sect::Idata])?;
    let mut code = vec![];
    #[cfg(not(debug_assertions))]
    for inst in insts {
      self.encode_inst(inst, &mut code)?;
    }
    #[cfg(debug_assertions)]
    for (idx, inst) in insts.enumerate() {
      let code_len = to_rva(&code)?;
      self.encode_inst(inst, &mut code)?;
      let inst_len = to_rva(&code)? - code_len;
      #[expect(clippy::print_stdout, clippy::use_debug)]
      if inst_len != validate_vec[idx] {
        println!("InternalError: actual: {} != expected: {} {inst:?}", inst_len, validate_vec[idx]);
      }
    }
    self.build_pe(code, data, rdata, pdata, xdata, bss, idata)
  }
  #[expect(clippy::too_many_lines)]
  fn encode_inst(&mut self, inst: &Inst, code: &mut Vec<u8>) -> ErrOR<()> {
    match inst {
      CMovCc(cc, dst, src) => {
        code.extend(RM::Reg(*src).encode_romsd(vec![0x0F, 0x40 + *cc as u8], *dst, true));
      }
      SetCc(cc, reg) => {
        code.extend(RM::Reg(*reg).encode_romsd(vec![0x0F, 0x90 + *cc as u8], Rax, false));
      }
      Shl1R(reg) => {
        code.extend(RM::Reg(*reg).encode_romsd(vec![0xD1], Rsp, true));
      }
      JCc(cc, lbl) => {
        let rel = self.get_rel(*lbl, to_rva(code)?, 6)?;
        code.push(0x0F);
        code.push(0x80 + *cc as u8);
        code.extend_from_slice(&rel.to_le_bytes());
      }
      DecR(reg) => {
        code.extend(RM::Reg(*reg).encode_romsd(vec![0xFF], Rcx, true));
      }
      IncR(reg) => {
        code.extend(RM::Reg(*reg).encode_romsd(vec![0xFF], Rax, true));
      }
      TestRdRd(dst, src) => {
        code.extend(RM::Reg(*dst).encode_romsd(vec![0x85], *src, false));
      }
      AddSd(xmm, xmm2) => {
        code.extend(RM::Reg(*xmm2).encode_romsd(vec![0xF2, 0x0F, 0x58], *xmm, false));
      }
      SubSd(xmm, xmm2) => {
        code.extend(RM::Reg(*xmm2).encode_romsd(vec![0xF2, 0x0F, 0x5C], *xmm, false));
      }
      MulSd(xmm, xmm2) => {
        code.extend(RM::Reg(*xmm2).encode_romsd(vec![0xF2, 0x0F, 0x59], *xmm, false));
      }
      DivSd(xmm, xmm2) => {
        code.extend(RM::Reg(*xmm2).encode_romsd(vec![0xF2, 0x0F, 0x5E], *xmm, false));
      }
      MovSdMX(mem, xmm) => {
        let size = self.inst_size(inst, 0)?;
        let memory = self.memory(*mem, to_rva(code)?, size)?;
        code.extend(memory.encode_romsd(vec![0xF2, 0x0F, 0x11], *xmm, false));
      }
      MovSdXM(xmm, mem) => {
        let size = self.inst_size(inst, 0)?;
        code.extend(self.memory(*mem, to_rva(code)?, size)?.encode_romsd(
          vec![0xF2, 0x0F, 0x10],
          *xmm,
          false,
        ));
      }
      CvtSi2Sd(xmm, reg) => {
        code.push(0xF2);
        code.extend(RM::Reg(*reg).encode_romsd(vec![0x0F, 0x2A], *xmm, true));
      }
      CvtTSd2Si(reg, xmm) => {
        code.push(0xF2);
        code.extend(RM::Reg(*xmm).encode_romsd(vec![0x0F, 0x2C], *reg, true));
      }
      LogicRbRb(logic, dst, src) => {
        dst.guard_reg8()?;
        src.guard_reg8()?;
        code.extend(RM::Reg(*src).encode_romsd(vec![*logic as u8], *dst, false));
      }
      LogicRR(logic, dst, src) => {
        code.extend(RM::Reg(*src).encode_romsd(vec![*logic as u8 + 1], *dst, true));
      }
      AddRR(dst, src) => {
        code.extend(RM::Reg(*dst).encode_romsd(vec![0x01], *src, true));
      }
      SubRR(dst, src) => {
        code.extend(RM::Reg(*dst).encode_romsd(vec![0x29], *src, true));
      }
      IMulRR(dst, src) => {
        code.extend(RM::Reg(*src).encode_romsd(vec![0x0F, 0xAF], *dst, true));
      }
      Call(lbl) => {
        let rel = self.get_rel(*lbl, to_rva(code)?, 5)?;
        code.push(0xE8);
        code.extend_from_slice(&rel.to_le_bytes());
      }
      CallApi((dll, func)) => {
        let cur_rva = self.rva[&Sect::Text] + to_rva(code)?;
        let func_address_rva = self.resolve_address_rva(*dll, *func)?;
        let rip_rel_disp = i32::try_from(func_address_rva)? - i32::try_from(cur_rva)? - 6i32;
        code.extend_from_slice(&RM::RipRel(rip_rel_disp).encode_romsd(vec![0xFF], Rdx, false));
      }
      Custom(bytes) => code.extend_from_slice(bytes),
      #[expect(clippy::cast_sign_loss)]
      CmpRIb(reg, imm) => {
        code.extend_from_slice(&RM::Reg(*reg).encode_romsd(vec![0x83], Rdi, true));
        code.push(*imm as u8);
      }
      Jmp(lbl) => {
        let rel = self.get_rel(*lbl, to_rva(code)?, 5)?;
        code.push(0xE9);
        code.extend_from_slice(&rel.to_le_bytes());
      }
      LeaRM(reg, mem) => {
        let size = self.inst_size(inst, 0)?;
        let memory = self.memory(*mem, to_rva(code)?, size)?;
        code.extend(memory.encode_romsd(vec![0x8D], *reg, true));
      }
      MovBB(operands) => self.encode_mov_b_b(code, **operands)?,
      MovDD(operands) => self.encode_mov_d_d(code, **operands)?,
      MovQQ(operands) => self.encode_mov_q_q(code, **operands)?,
      ShlRIb(reg, byte) => {
        code.extend(RM::Reg(*reg).encode_romsd(vec![0xC1], Rsp, true));
        code.push(*byte);
      }
      ShrRIb(reg, byte) => {
        code.extend(RM::Reg(*reg).encode_romsd(vec![0xC1], Rbp, true));
        code.push(*byte);
      }
      SarRIb(reg, byte) => {
        code.extend(RM::Reg(*reg).encode_romsd(vec![0xC1], Rdi, true));
        code.push(*byte);
      }
      NegR(reg) => {
        code.extend(RM::Reg(*reg).encode_romsd(vec![0xF7], Rbx, true));
      }
      NegRb(reg) => {
        code.extend(RM::Reg(*reg).encode_romsd(vec![0xF6], Rbx, false));
      }
      NotR(reg) => {
        code.extend(RM::Reg(*reg).encode_romsd(vec![0xF7], Rdx, true));
      }
      NotRb(reg) => {
        reg.guard_reg8()?;
        code.extend(RM::Reg(*reg).encode_romsd(vec![0xF6], Rdx, false));
      }
      Pop(reg) => code.extend(reg.mini_opcode(&[], 0x58, false)),
      Push(reg) => code.extend(reg.mini_opcode(&[], 0x50, false)),
      IDivR(reg) => {
        code.extend(RM::Reg(*reg).encode_romsd(vec![0xF7], Rdi, true));
      }
      SubRId(reg, imm) => {
        code.extend_from_slice(&RM::Reg(*reg).encode_romsd(vec![0x81], Rbp, true));
        code.extend_from_slice(&imm.to_le_bytes());
      }
      AddRId(reg, imm) => {
        code.extend_from_slice(&RM::Reg(*reg).encode_romsd(vec![0x81], Rax, true));
        code.extend_from_slice(&imm.to_le_bytes());
      }
      Clear(reg) => {
        code.extend_from_slice(&RM::Reg(*reg).encode_romsd(vec![0x31], *reg, false));
      }
      Lbl(_) => {}
    }
    Ok(())
  }
  fn encode_mov_b_b(
    &mut self, code: &mut Vec<u8>, operands: (Operand<u8>, Operand<u8>),
  ) -> ErrOR<()> {
    match operands {
      (Reg(dst), Reg(src)) => {
        dst.guard_reg8()?;
        src.guard_reg8()?;
        code.extend(RM::Reg(dst).encode_romsd(vec![0x88], src, false));
      }
      (Reg(dst), Mem(src)) => {
        dst.guard_reg8()?;
        let size = size_of_mov_b_b(operands)?;
        code.extend(self.memory(src, to_rva(code)?, size)?.encode_romsd(vec![0x8A], dst, false));
      }
      (Mem(dst), Reg(src)) => {
        src.guard_reg8()?;
        let size = size_of_mov_b_b(operands)?;
        code.extend(self.memory(dst, to_rva(code)?, size)?.encode_romsd(vec![0x88], src, false));
      }
      (Mem(mem), Imm(imm)) => {
        let size = size_of_mov_b_b(operands)?;
        code.extend(self.memory(mem, to_rva(code)?, size)?.encode_romsd(vec![0xC6], Rax, false));
        code.extend_from_slice(&imm.to_le_bytes());
      }
      (Reg(dst), Imm(imm)) => {
        dst.guard_reg8()?;
        code.extend(dst.mini_opcode(&[], 0xB0, false));
        code.extend_from_slice(&imm.to_le_bytes());
      }
      _ => {
        return Err(InternalError(InvalidInst(format!("MovBB{operands:?}"))));
      }
    }
    Ok(())
  }
  fn encode_mov_d_d(
    &mut self, code: &mut Vec<u8>, operands: (Operand<u32>, Operand<u32>),
  ) -> ErrOR<()> {
    match operands {
      (Reg(dst), Ref(src)) => {
        let mem = RM::Base(src, Disp::Zero);
        code.extend(mem.encode_romsd(vec![0x8B], dst, false));
      }
      (Ref(dst), Reg(src)) => {
        let mem = RM::Base(dst, Disp::Zero);
        code.extend(mem.encode_romsd(vec![0x89], src, false));
      }
      (Reg(dst), Reg(src)) => {
        code.extend(RM::Reg(dst).encode_romsd(vec![0x89], src, false));
      }
      (Reg(dst), Mem(src)) => {
        let size = size_of_mov_d_d(operands)?;
        code.extend(self.memory(src, to_rva(code)?, size)?.encode_romsd(vec![0x8B], dst, false));
      }
      (Mem(dst), Reg(src)) => {
        let size = size_of_mov_d_d(operands)?;
        code.extend(self.memory(dst, to_rva(code)?, size)?.encode_romsd(vec![0x89], src, false));
      }
      (Mem(mem), Imm(imm)) => {
        let size = size_of_mov_d_d(operands)?;
        code.extend(self.memory(mem, to_rva(code)?, size)?.encode_romsd(vec![0xC7], Rax, false));
        code.extend_from_slice(&imm.to_le_bytes());
      }
      (Reg(dst), Imm(imm)) => {
        code.extend(dst.mini_opcode(&[], 0xB8, false));
        code.extend_from_slice(&imm.to_le_bytes());
      }
      _ => {
        return Err(InternalError(InvalidInst(format!("MovDD{operands:?}"))));
      }
    }
    Ok(())
  }
  fn encode_mov_q_q(
    &mut self, code: &mut Vec<u8>, operands: (Operand<u64>, Operand<u64>),
  ) -> ErrOR<()> {
    match operands {
      (Reg(dst), Args(offset)) => {
        let mem = RM::Base(
          Rsp,
          if offset == 0 {
            Disp::Zero
          } else if let Ok(byte_offset) = i8::try_from(offset) {
            Disp::Byte(byte_offset)
          } else {
            Disp::Dword(i32::try_from(offset)?)
          },
        );
        code.extend(mem.encode_romsd(vec![0x8B], dst, true));
      }
      (Reg(dst), Ref(src)) => {
        let mem = RM::Base(src, Disp::Zero);
        code.extend(mem.encode_romsd(vec![0x8B], dst, true));
      }
      (Ref(dst), Reg(src)) => {
        let mem = RM::Base(dst, Disp::Zero);
        code.extend(mem.encode_romsd(vec![0x89], src, true));
      }
      (Args(offset), Reg(src)) => {
        let mem = RM::Base(
          Rsp,
          if offset == 0 {
            Disp::Zero
          } else if let Ok(byte_offset) = i8::try_from(offset) {
            Disp::Byte(byte_offset)
          } else {
            Disp::Dword(i32::try_from(offset)?)
          },
        );
        code.extend(mem.encode_romsd(vec![0x89], src, true));
      }
      (Reg(dst), Reg(src)) => {
        code.extend(RM::Reg(dst).encode_romsd(vec![0x89], src, true));
      }
      (Reg(dst), Mem(src)) => {
        let size = size_of_mov_q_q(operands)?;
        code.extend(self.memory(src, to_rva(code)?, size)?.encode_romsd(vec![0x8B], dst, true));
      }
      (Mem(dst), Reg(src)) => {
        let size = size_of_mov_q_q(operands)?;
        code.extend(self.memory(dst, to_rva(code)?, size)?.encode_romsd(vec![0x89], src, true));
      }
      (Reg(dst), Imm(imm)) => {
        code.extend(dst.mini_opcode(&[], 0xB8, true));
        code.extend_from_slice(&imm.to_le_bytes());
      }
      _ => {
        return Err(InternalError(InvalidInst(format!("MovQQ{operands:?}"))));
      }
    }
    Ok(())
  }
  fn get_rel(&self, lbl: u32, code_len: u32, inst_len: u32) -> ErrOR<i32> {
    let next_rva = self.rva[&Sect::Text] + code_len + inst_len;
    let sect = self.addr_sect.get(&lbl).ok_or(InternalError(UnknownLabel))?;
    let target = self.rva[sect] + *self.sym_addr.get(&lbl).ok_or(InternalError(UnknownLabel))?;
    Ok(i32::try_from(target)? - i32::try_from(next_rva)?)
  }
  fn inst_size(&mut self, inst: &Inst, text: u32) -> ErrOR<u32> {
    Ok(match inst {
      Custom(bytes) => to_rva(bytes)?,
      NegR(_) | NotR(_) | LogicRR(..) | IncR(_) | DecR(_) | Shl1R(_) | IDivR(_) | SubRR(..)
      | AddRR(..) => 3,
      CMovCc(..) | SarRIb(..) | ShrRIb(..) | ShlRIb(..) | IMulRR(..) | CmpRIb(..) => 4,
      CvtSi2Sd(..) | CvtTSd2Si(..) | Jmp(_) | Call(_) => 5,
      JCc(..) | CallApi(_) => 6,
      SubRId(..) | AddRId(..) => 7,
      AddSd(xmm, xmm2) | SubSd(xmm, xmm2) | MulSd(xmm, xmm2) | DivSd(xmm, xmm2) => {
        4 + (xmm.rex_size() | xmm2.rex_size())
      }
      MovSdXM(xmm, mem) | MovSdMX(mem, xmm) => 3 + xmm.rex_size() + mem.size_of_mo_si_di(),
      LeaRM(_, mem) => 2 + mem.size_of_mo_si_di(),
      NegRb(reg) | NotRb(reg) | Clear(reg) => reg.rex_size() + 2,
      MovBB(operands) => size_of_mov_b_b(**operands)?,
      MovQQ(operands) => size_of_mov_q_q(**operands)?,
      MovDD(operands) => size_of_mov_d_d(**operands)?,
      LogicRbRb(_, dst, src) | TestRdRd(dst, src) => (dst.rex_size() | src.rex_size()) + 2,
      Pop(reg) | Push(reg) => reg.rex_size() + 1,
      SetCc(_, reg) => reg.rex_size() + 3,
      Lbl(idx) => {
        self.add_label(Sect::Text, *idx, text);
        0
      }
    })
  }
  pub(crate) fn memory(&self, lbl: Memory, code_len: u32, len_inst: u32) -> ErrOR<RM> {
    Ok(match lbl {
      Global { id } => RM::RipRel(self.get_rel(id, code_len, len_inst)?),
      GlobalD { id, disp } => RM::RipRel(self.get_rel(id, code_len, len_inst)? + disp),
      Local { offset } | Tmp { offset } => RM::Base(
        Rbp,
        if let Ok(l_i8) = i8::try_from(-offset) { Disp::Byte(l_i8) } else { Disp::Dword(-offset) },
      ),
    })
  }
  pub(crate) fn new(dlls: Dlls) -> Self {
    Self { sym_addr: HashMap::new(), addr_sect: HashMap::new(), rva: HashMap::new(), dlls }
  }
  pub(crate) fn resolve_address_rva(&self, dll_idx: u32, func_idx: u32) -> ErrOR<u32> {
    let dll_index = usize::try_from(dll_idx)?;
    let mut lookup_offset = (to_rva(&self.dlls)? + 1) * 20;
    for dll in &self.dlls[0..dll_index] {
      let lookup_size = (to_rva(&dll.1)? + 1) * 8;
      lookup_offset += lookup_size * 2;
    }
    let lookup_size = (to_rva(&self.dlls[dll_index].1)? + 1) * 8;
    let address_offset = lookup_offset + lookup_size;
    Ok(self.rva[&Sect::Idata] + address_offset + func_idx * 8)
  }
  pub(crate) fn resolve_iat_size(&self) -> usize {
    let mut iat_size = 0;
    for dll in &self.dlls {
      iat_size += (dll.1.len() + 1) * 8;
    }
    iat_size
  }
}
impl Register {
  fn guard_reg8(self) -> ErrOR<()> {
    if Rdi >= self && self >= Rsp {
      Err(InternalError(InvalidInst("spl, bpl ,sil and dil".into())))
    } else {
      Ok(())
    }
  }
  fn mini_opcode(self, prefix: &[u8], opcode: u8, rex_w: bool) -> Vec<u8> {
    let mut code = vec![];
    let (reg_bits, rex_b) = self.reg_field();
    code.extend_from_slice(prefix);
    if rex_b || rex_w {
      code.push(0x40 + u8::from(rex_b) + (u8::from(rex_w) << 3u8));
    }
    code.push(opcode + reg_bits);
    code
  }
  fn reg_field(self) -> (u8, bool) {
    let num = self as u8 & 7;
    let rex_high = self >= R8;
    (num, rex_high)
  }
  fn rex_size(self) -> u32 {
    u32::from(u8::from(self >= R8))
  }
}
impl RM {
  pub(crate) fn encode_romsd(&self, mut opcode: Vec<u8>, reg: Register, rex_w: bool) -> Vec<u8> {
    let (reg_bits, rex_r) = reg.reg_field();
    let mut rex_x = false;
    let mut rex_b = false;
    match self {
      RM::Base(base, disp) => {
        let (base_bits, rb) = base.reg_field();
        rex_b = rb;
        let mod_bits = match disp {
          Disp::Byte(_) => 0x40,
          Disp::Dword(_) => 0x80,
          Disp::Zero if base_bits == 5 => 0x40,
          Disp::Zero => 0,
        };
        opcode.push(mod_bits | (reg_bits << 3u8) | base_bits);
        if *base == Rsp || *base == R12 {
          opcode.push((4 << 3u8) | base_bits);
        }
        #[expect(clippy::cast_sign_loss)]
        match disp {
          Disp::Byte(int) => opcode.push(*int as u8),
          Disp::Dword(int) => opcode.extend(int.to_le_bytes()),
          Disp::Zero => {}
        }
      }
      RM::RipRel(disp) => {
        opcode.push((reg_bits << 3u8) | 5);
        opcode.extend(disp.to_le_bytes());
      }
      RM::Sib(Sib { index, scale, base, disp }) => {
        let (index_bits, rx) = index.reg_field();
        if index_bits != 4 {
          rex_x = rx;
        }
        let (base_bits, rb) = base.reg_field();
        rex_b = rb;
        opcode.push(
          match disp {
            Disp::Byte(_) => 0x40,
            Disp::Dword(_) => 0x80,
            Disp::Zero if base_bits == 5 => 0x40,
            Disp::Zero => 0,
          } | (reg_bits << 3u8)
            | 4,
        );
        opcode.push(((*scale as u8) << 6u8) | (index_bits << 3u8) | base_bits);
        #[expect(clippy::cast_sign_loss)]
        match disp {
          Disp::Byte(int) => opcode.push(*int as u8),
          Disp::Dword(int) => opcode.extend(int.to_le_bytes()),
          Disp::Zero => {}
        }
      }
      RM::Reg(reg2) => {
        let (bits, rb) = reg2.reg_field();
        rex_b = rb;
        opcode.push(0xC0 | (reg_bits << 3u8) | bits);
      }
    }
    if rex_w || rex_r || rex_x || rex_b {
      opcode.insert(
        0,
        0x40
          | (u8::from(rex_w) << 3u8)
          | (u8::from(rex_r) << 2u8)
          | (u8::from(rex_x) << 1u8)
          | u8::from(rex_b),
      );
    }
    opcode
  }
  /*
  pub(crate) fn size_of_inst(&self, w: bool, opcode_size: u32, imm_size: u32) -> u32 {
    let rex = u32::from(u8::from(w))
      | match self {
        RM::Sib(Sib { index, base, .. }) => base.rex_size() & index.rex_size(),
        RM::Base(base, _) => base.rex_size(),
        RM::Reg(reg) => reg.rex_size(),
        RM::RipRel(_) => 0,
      };
    let sib = 1;
    let disp = match self {
      RM::Base(_, disp) | RM::Sib(Sib { disp, .. }) => disp.size(),
      RM::Reg(_) => 0,
      RM::RipRel(_) => 4,
    };
    rex + opcode_size + sib + disp + imm_size
  }
  */
}
fn size_of_mov_q_q(operands: (Operand<u64>, Operand<u64>)) -> ErrOR<u32> {
  Ok(match operands {
    (Reg(_), Reg(_)) => 3,
    (Reg(_), Ref(reg)) | (Ref(reg), Reg(_)) => {
      if reg == Rsp || reg == R12 {
        4
      } else {
        3
      }
    }
    (Reg(_), Mem(mem)) | (Mem(mem), Reg(_)) => 2 + mem.size_of_mo_si_di(),
    (Reg(_), Args(offset)) | (Args(offset), Reg(_)) => {
      4 + if offset == 0 {
        0
      } else if i8::try_from(offset).is_ok() {
        1
      } else {
        4
      }
    }
    (Reg(_), Imm(_)) => 10,
    _ => {
      return Err(InternalError(InvalidInst(format!("MovQQ{operands:?}"))));
    }
  })
}
fn size_of_mov_b_b(operands: (Operand<u8>, Operand<u8>)) -> ErrOR<u32> {
  Ok(match operands {
    (Reg(r1), Reg(r2)) => (r1.rex_size() | r2.rex_size()) + 2,
    (Reg(reg), Mem(mem)) | (Mem(mem), Reg(reg)) => reg.rex_size() + 1 + mem.size_of_mo_si_di(),
    (Mem(mem), Imm(_)) => 2 + mem.size_of_mo_si_di(),
    (Reg(reg), Imm(_)) => reg.rex_size() + 2,
    _ => {
      return Err(InternalError(InvalidInst(format!("MovBB{operands:?}"))));
    }
  })
}
fn size_of_mov_d_d(operands: (Operand<u32>, Operand<u32>)) -> ErrOR<u32> {
  Ok(match operands {
    (Reg(r1), Reg(r2)) => (r1.rex_size() | r2.rex_size()) + 2,
    (Reg(r1), Ref(r2)) | (Ref(r1), Reg(r2)) => {
      (r1.rex_size() | r2.rex_size()) + if r1 == Rsp || r1 == R12 { 3 } else { 2 }
    }
    (Reg(reg), Mem(mem)) | (Mem(mem), Reg(reg)) => reg.rex_size() + 1 + mem.size_of_mo_si_di(),
    (Mem(mem), Imm(_)) => 5 + mem.size_of_mo_si_di(),
    (Reg(reg), Imm(_)) => reg.rex_size() + 5,
    _ => {
      return Err(InternalError(InvalidInst(format!("MovDD{operands:?}"))));
    }
  })
}
fn to_rva<T>(data: &[T]) -> ErrOR<u32> {
  u32::try_from(data.len()).map_err(|_| InternalError(TooLargeSection))
}
