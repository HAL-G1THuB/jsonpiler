#![allow(dead_code)]
use crate::{
  Assembler, Disp, Dlls, ErrOR,
  Inst::{self, *},
  Memory,
  OpQ::{self, *},
  Reg::{self, *},
  Sect, Sib,
  VarKind::{self, Global, Local, Tmp},
  utility::{align_up, align_up_32},
};
use core::iter::Chain;
use core::slice::Iter;
use std::collections::HashMap;
impl Assembler {
  fn add_label(&mut self, sect: Sect, idx: usize, offset: u32) {
    self.sym_addr.insert(idx, offset);
    self.addr_sect.insert(idx, sect);
  }
  pub(crate) fn assemble_and_link(
    mut self, insts: Chain<Iter<Inst>, Iter<Inst>>,
  ) -> ErrOR<Vec<u8>> {
    self.sym_addr.clear();
    self.addr_sect.clear();
    let mut text: u32 = 0;
    let mut data = vec![];
    let mut bss: u32 = 0;
    #[cfg(debug_assertions)]
    let mut validate_vec = Vec::new();
    for inst in insts.clone() {
      let inst_len = self.inst_size(inst, &mut bss, &mut data, text)?;
      text += inst_len;
      #[cfg(debug_assertions)]
      validate_vec.push(inst_len);
    }
    self.rva.insert(Sect::Text, 0x1000);
    self.rva.insert(Sect::Data, 0x1000 + align_up_32(text, 0x1000)?);
    let data_raw_size = align_up_32(u32::try_from(data.len())?, 0x1000)?;
    self.rva.insert(Sect::Bss, self.rva[&Sect::Data] + data_raw_size);
    self.rva.insert(Sect::Idata, self.rva[&Sect::Bss] + align_up_32(bss, 0x1000)?);
    let idata = self.build_idata_section(self.rva[&Sect::Idata])?;
    let mut code = Vec::new();
    #[cfg(not(debug_assertions))]
    for inst in insts {
      self.encode_inst(inst, &mut code)?;
    }
    #[cfg(debug_assertions)]
    for (idx, inst) in insts.enumerate() {
      let code_len = code.len();
      self.encode_inst(inst, &mut code)?;
      #[expect(clippy::print_stdout, clippy::use_debug)]
      if code.len() - code_len != validate_vec[idx].try_into()? {
        println!(
          "InternalError!!! actual: {} != prediction: {} {inst:?}",
          code.len() - code_len,
          validate_vec[idx]
        );
      }
    }
    self.build_pe(&code, &data, bss, &idata)
  }
  #[expect(clippy::too_many_lines)]
  fn encode_inst(&mut self, inst: &Inst, code: &mut Vec<u8>) -> ErrOR<()> {
    match inst {
      Shl1R(reg) => {
        code.extend(Memory::Reg(*reg).encode_romsd(vec![0xD1], Rsp, true));
      }
      Jcc(cc, lbl) => {
        let rel = self.get_rel(*lbl, code.len(), 6)?;
        code.push(0x0F);
        code.push(0x80 + *cc as u8);
        code.extend_from_slice(&rel.to_le_bytes());
      }
      #[expect(clippy::cast_sign_loss)]
      JmpSh(lbl) => {
        let rel = self.get_rel(*lbl, code.len(), 2)?;
        code.push(0xEB);
        code.push(i8::try_from(rel).map_err(|_| "InternalError: Failed short jump")? as u8);
      }
      DecQ(reg) => {
        code.extend(Memory::Reg(*reg).encode_romsd(vec![0xFF], Rcx, true));
      }
      TestRdRd(dst, src) => {
        code.extend(Memory::Reg(*dst).encode_romsd(vec![0x85], *src, false));
      }
      AddSd(xmm, xmm2) => {
        code.extend(Memory::Reg(*xmm2).encode_romsd(vec![0xF2, 0x0F, 0x58], *xmm, false));
      }
      SubSd(xmm, xmm2) => {
        code.extend(Memory::Reg(*xmm2).encode_romsd(vec![0xF2, 0x0F, 0x5C], *xmm, false));
      }
      MulSd(xmm, xmm2) => {
        code.extend(Memory::Reg(*xmm2).encode_romsd(vec![0xF2, 0x0F, 0x59], *xmm, false));
      }
      DivSd(xmm, xmm2) => {
        code.extend(Memory::Reg(*xmm2).encode_romsd(vec![0xF2, 0x0F, 0x5E], *xmm, false));
      }
      MovSdMX(mem, xmm) => {
        let size = self.inst_size(inst, &mut 0, &mut vec![], 0)?;
        let memory = self.memory(*mem, code.len(), size)?;
        code.extend(memory.encode_romsd(vec![0xF2, 0x0F, 0x11], *xmm, false));
      }
      MovSdXM(xmm, mem) => {
        let size = self.inst_size(inst, &mut 0, &mut vec![], 0)?;
        code.extend(self.memory(*mem, code.len(), size)?.encode_romsd(
          vec![0xF2, 0x0F, 0x10],
          *xmm,
          false,
        ));
      }
      CvtTSd2Si(reg, xmm) => {
        code.push(0xF2);
        code.extend(Memory::Reg(*xmm).encode_romsd(vec![0x0F, 0x2C], *reg, true));
      }
      TestRR(dst, src) => {
        code.extend(Memory::Reg(*dst).encode_romsd(vec![0x85], *src, true));
      }
      CmpRR(dst, src) => {
        code.extend(Memory::Reg(*dst).encode_romsd(vec![0x39], *src, true));
      }
      XorRbRb(dst, src) => {
        dst.guard_reg8()?;
        src.guard_reg8()?;
        code.extend(Memory::Reg(*dst).encode_romsd(vec![0x30], *src, false));
      }
      OrRbRb(dst, src) => {
        dst.guard_reg8()?;
        src.guard_reg8()?;
        code.extend(Memory::Reg(*dst).encode_romsd(vec![0x08], *src, false));
      }
      AndRbRb(dst, src) => {
        dst.guard_reg8()?;
        src.guard_reg8()?;
        code.extend(Memory::Reg(*dst).encode_romsd(vec![0x20], *src, false));
      }
      XorRR(dst, src) => {
        code.extend(Memory::Reg(*dst).encode_romsd(vec![0x31], *src, true));
      }
      AddRR(dst, src) => {
        code.extend(Memory::Reg(*dst).encode_romsd(vec![0x01], *src, true));
      }
      SubRR(dst, src) => {
        code.extend(Memory::Reg(*dst).encode_romsd(vec![0x29], *src, true));
      }
      IMulRR(dst, src) => {
        code.extend(Memory::Reg(*src).encode_romsd(vec![0x0F, 0xAF], *dst, true));
      }
      Call(lbl) => {
        let rel = self.get_rel(*lbl, code.len(), 5)?;
        code.push(0xE8);
        code.extend_from_slice(&rel.to_le_bytes());
      }
      CallApi((dll, func)) => {
        let cur_rva = self.rva[&Sect::Text] + u32::try_from(code.len())?;
        let func_address_rva = self.resolve_address_rva(*dll, *func)?;
        let rip_rel_disp = i32::try_from(func_address_rva)? - i32::try_from(cur_rva)? - 6i32;
        code.extend_from_slice(&Memory::RipRel(rip_rel_disp).encode_romsd(vec![0xFF], Rdx, false));
      }
      Custom(bytes) => code.extend(bytes),
      #[expect(clippy::cast_sign_loss)]
      CmpRIb(reg, imm) => {
        code.extend_from_slice(&Memory::Reg(*reg).encode_romsd(vec![0x83], Rdi, true));
        code.push(*imm as u8);
      }
      Jmp(lbl) => {
        let rel = self.get_rel(*lbl, code.len(), 5)?;
        code.push(0xE9);
        code.extend_from_slice(&rel.to_le_bytes());
      }
      TestRbRb(dst, src) => {
        dst.guard_reg8()?;
        src.guard_reg8()?;
        code.extend(Memory::Reg(*dst).encode_romsd(vec![0x84], *src, false));
      }
      LeaRM(reg, mem) => {
        let size = self.inst_size(inst, &mut 0, &mut vec![], 0)?;
        let memory = self.memory(*mem, code.len(), size)?;
        code.extend(memory.encode_romsd(vec![0x8D], *reg, true));
      }
      MovQQ(op1, op2) => self.encode_mov_q_q(code, op1, op2)?,
      MovMbIb(mem, byte) => {
        let size = self.inst_size(inst, &mut 0, &mut vec![], 0)?;
        let memory = self.memory(*mem, code.len(), size)?;
        code.extend(memory.encode_romsd(vec![0xC6], Rax, false));
        code.push(*byte);
      }
      MovMId(dst, dword) => {
        let size = self.inst_size(inst, &mut 0, &mut vec![], 0)?;
        let memory = self.memory(*dst, code.len(), size)?;
        code.extend(memory.encode_romsd(vec![0xC7], Rax, false));
        code.extend_from_slice(&dword.to_le_bytes());
      }
      MovRbIb(reg, byte) => {
        reg.guard_reg8()?;
        code.extend(reg.mini_opcode(0xB0, false));
        code.push(*byte);
      }
      MovRbMb(reg, mem) => {
        reg.guard_reg8()?;
        let size = self.inst_size(inst, &mut 0, &mut vec![], 0)?;
        let memory = self.memory(*mem, code.len(), size)?;
        code.extend(memory.encode_romsd(vec![0x8A], *reg, false));
      }
      MovMbRb(mem, reg) => {
        reg.guard_reg8()?;
        let size = self.inst_size(inst, &mut 0, &mut vec![], 0)?;
        code.extend(self.memory(*mem, code.len(), size)?.encode_romsd(vec![0x88], *reg, false));
      }
      MovRId(reg, dword) => {
        code.extend(reg.mini_opcode(0xB8, false));
        code.extend_from_slice(&dword.to_le_bytes());
      }
      NegR(reg) => {
        reg.guard_reg8()?;
        code.extend(Memory::Reg(*reg).encode_romsd(vec![0xF7], Rbx, false));
      }
      NotRb(reg) => {
        reg.guard_reg8()?;
        code.extend(Memory::Reg(*reg).encode_romsd(vec![0xF6], Rdx, false));
      }
      Pop(reg) => code.extend(reg.mini_opcode(0x58, false)),
      Push(reg) => code.extend(reg.mini_opcode(0x50, false)),
      Ret => code.push(0xC3),
      IDivR(reg) => {
        code.extend(Memory::Reg(*reg).encode_romsd(vec![0xF7], Rdi, true));
      }
      SubRId(reg, imm) => {
        code.extend_from_slice(&Memory::Reg(*reg).encode_romsd(vec![0x81], Rbp, true));
        code.extend_from_slice(&imm.to_le_bytes());
      }
      AddRId(reg, imm) => {
        code.extend_from_slice(&Memory::Reg(*reg).encode_romsd(vec![0x81], Rax, true));
        code.extend_from_slice(&imm.to_le_bytes());
      }
      Clear(reg) => {
        code.extend_from_slice(&Memory::Reg(*reg).encode_romsd(vec![0x31], *reg, false));
      }
      StringZ(..) | Bss(..) | Byte(..) | Lbl(_) | Quad(..) => {}
    }
    Ok(())
  }
  fn encode_mov_q_q(&self, code: &mut Vec<u8>, op1: &OpQ, op2: &OpQ) -> ErrOR<()> {
    match (op1, op2) {
      (Rq(dst), Args(offset)) => {
        let mem = Memory::Base(Rsp, Disp::Byte(i8::try_from(*offset)?));
        code.extend(mem.encode_romsd(vec![0x8B], *dst, true));
      }
      (Args(offset), Rq(src)) => {
        let mem = Memory::Base(Rsp, Disp::Byte(i8::try_from(*offset)?));
        code.extend(mem.encode_romsd(vec![0x89], *src, true));
      }
      (Rq(dst), Rq(src)) => {
        code.extend(Memory::Reg(*dst).encode_romsd(vec![0x89], *src, true));
      }
      (Rq(dst), Mq(src)) => {
        code.extend(self.memory(*src, code.len(), 3)?.encode_romsd(vec![0x8B], *dst, true));
      }
      (Mq(dst), Rq(src)) => {
        code.extend(self.memory(*dst, code.len(), 3)?.encode_romsd(vec![0x89], *src, true));
      }
      (Rq(dst), Iq(imm)) => {
        code.extend(dst.mini_opcode(0xB8, true));
        code.extend_from_slice(&imm.to_le_bytes());
      }
      _ => {
        return Err("InternalError: Unsupported operand types: MovQQ(?, ?)".into());
      }
    }
    Ok(())
  }
  fn get_rel(&self, lbl: usize, code_len: usize, inst_len: u32) -> ErrOR<i32> {
    let next_rva = self.rva[&Sect::Text] + u32::try_from(code_len)? + inst_len;
    let sect = self.addr_sect.get(&lbl).ok_or("unknown label for call")?;
    let target = self.rva[sect] + *self.sym_addr.get(&lbl).ok_or("unknown label for call")?;
    Ok(i32::try_from(target)? - i32::try_from(next_rva)?)
  }
  fn inst_size(&mut self, inst: &Inst, bss: &mut u32, data: &mut Vec<u8>, text: u32) -> ErrOR<u32> {
    Ok(match inst {
      Custom(bytes) => u32::try_from(bytes.len())?,
      Ret => 1,
      JmpSh(_) | XorRbRb(..) | OrRbRb(..) | AndRbRb(..) | TestRbRb(..) => 2,
      DecQ(_) | CmpRR(..) | Shl1R(_) | TestRR(..) | IDivR(_) | XorRR(..) | SubRR(..)
      | AddRR(..) => 3,
      IMulRR(..) | CmpRIb(..) => 4,
      CvtTSd2Si(..) | Jmp(_) | Call(_) => 5,
      Jcc(..) | CallApi(_) => 6,
      SubRId(..) | AddRId(..) => 7,
      TestRdRd(dst, src) => (dst.rex_size() | src.rex_size()) + 1 + 1,
      AddSd(xmm, xmm2) | SubSd(xmm, xmm2) | MulSd(xmm, xmm2) | DivSd(xmm, xmm2) => {
        4 + (xmm.rex_size() | xmm2.rex_size())
      }
      MovSdXM(xmm, mem) | MovSdMX(mem, xmm) => 3 + xmm.rex_size() + mem.size_of_mo_si_di(),
      LeaRM(_, mem) => 2 + mem.size_of_mo_si_di(),
      NotRb(reg) | NegR(reg) | Clear(reg) | MovRbIb(reg, _) => reg.rex_size() + 2,
      MovMbIb(mem, _) => 1 + mem.size_of_mo_si_di() + 1,
      MovRbMb(reg, mem) | MovMbRb(mem, reg) => reg.rex_size() + 1 + mem.size_of_mo_si_di(),
      MovQQ(op1, op2) => match (op1, op2) {
        (Rq(_), Rq(_)) => 3,
        (Rq(_), Mq(mem)) | (Mq(mem), Rq(_)) => 2 + mem.size_of_mo_si_di(),
        (Rq(_), Args(_)) | (Args(_), Rq(_)) => 5,
        (Rq(_), Iq(_)) => 10,
        _ => return Err("InternalError: Unsupported operand types: MovQQ(?, ?)".into()),
      },
      MovMId(mem, _) => 1 + 1 + mem.size_of_mo_si_di() + 4,
      Bss(idx, size) => {
        self.add_label(Sect::Bss, *idx, *bss);
        *bss += *size;
        0
      }
      Byte(idx, byte) => {
        self.add_label(Sect::Data, *idx, u32::try_from(data.len())?);
        data.push(*byte);
        0
      }
      Lbl(idx) => {
        self.add_label(Sect::Text, *idx, text);
        0
      }
      Quad(idx, qword) => {
        data.resize(align_up(data.len(), 8)?, 0);
        self.add_label(Sect::Data, *idx, u32::try_from(data.len())?);
        data.extend_from_slice(&(*qword).to_le_bytes());
        0
      }
      Pop(reg) | Push(reg) => reg.rex_size() + 1,
      MovRId(reg, _) => reg.rex_size() + 5,
      StringZ(idx, string) => {
        self.add_label(Sect::Data, *idx, u32::try_from(data.len())?);
        data.extend_from_slice(string.as_bytes());
        data.push(0x00);
        0
      }
    })
  }
  #[expect(clippy::cast_possible_wrap)]
  pub(crate) fn memory(&self, lbl: VarKind, code_len: usize, len_inst: u32) -> ErrOR<Memory> {
    Ok(match lbl {
      Global { id } => Memory::RipRel(self.get_rel(id, code_len, len_inst)?),
      Local { offset } | Tmp { offset } => Memory::Base(Rbp, crate::Disp::Dword(-(offset as i32))),
    })
  }
  pub(crate) fn new(dlls: Dlls) -> Self {
    Self { sym_addr: HashMap::new(), addr_sect: HashMap::new(), rva: HashMap::new(), dlls }
  }
  pub(crate) fn resolve_address_rva(&self, dll_idx: usize, func_idx: usize) -> ErrOR<u32> {
    let mut lookup_offset = (self.dlls.len() + 1) * 20;
    for dll in &self.dlls[0..dll_idx] {
      let lookup_size = (dll.1.len() + 1) * 8;
      lookup_offset += lookup_size * 2;
    }
    let lookup_size = (self.dlls[dll_idx].1.len() + 1) * 8;
    let address_offset = lookup_offset + lookup_size;
    Ok(self.rva[&Sect::Idata] + u32::try_from(address_offset + func_idx * 8)?)
  }
  pub(crate) fn resolve_iat_size(&self) -> usize {
    let mut iat_size = 0;
    for dll in &self.dlls {
      iat_size += (dll.1.len() + 1) * 8;
    }
    iat_size
  }
}
impl Reg {
  fn guard_reg8(self) -> ErrOR<()> {
    if Rdi >= self && self >= Rsp {
      Err("InternalError: 8-bit registers spl, bpl ,sil and dil are not implemented".into())
    } else {
      Ok(())
    }
  }
  fn mini_opcode(self, opcode: u8, rex_w: bool) -> Vec<u8> {
    let mut code = vec![];
    let (reg_bits, rex_b) = self.reg_field();
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
impl Memory {
  pub(crate) fn encode_romsd(&self, mut opcode: Vec<u8>, reg: Reg, rex_w: bool) -> Vec<u8> {
    let (reg_bits, rex_r) = reg.reg_field();
    let mut rex_x = false;
    let mut rex_b = false;
    match self {
      Memory::Base(base, disp) => {
        let (base_bits, rb) = base.reg_field();
        rex_b = rb;
        opcode.push(
          match disp {
            Disp::Zero => {
              if base_bits == 5 {
                0x40
              } else {
                0
              }
            }
            Disp::Byte(_) => 0x40,
            Disp::Dword(_) => 0x80,
          } | (reg_bits << 3u8)
            | 4,
        );
        opcode.push((4 << 3u8) | (base_bits & 7));
        #[expect(clippy::cast_sign_loss)]
        match disp {
          Disp::Byte(int) => opcode.push(*int as u8),
          Disp::Dword(int) => opcode.extend(int.to_le_bytes()),
          Disp::Zero => {}
        }
      }
      Memory::RipRel(disp) => {
        opcode.push((reg_bits << 3u8) | 5);
        opcode.extend(disp.to_le_bytes());
      }
      Memory::Sib(Sib { index, scale, base, disp }) => {
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
            Disp::Zero => {
              if base_bits == 5 {
                0x40
              } else {
                0
              }
            }
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
      Memory::Reg(reg2) => {
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
  pub(crate) fn size_of_inst(&self, w: bool, opcode_size: u32, imm_size: u32) -> u32 {
    let rex = u32::from(u8::from(w))
      | match self {
        Memory::Sib(Sib { index, base, .. }) => base.rex_size() & index.rex_size(),
        Memory::Base(base, _) => base.rex_size(),
        Memory::Reg(reg) => reg.rex_size(),
        Memory::RipRel(_) => 0,
      };
    let sib_disp = match self {
      Memory::Base(_, disp) | Memory::Sib(Sib { disp, .. }) => match disp {
        Disp::Zero => 1,
        Disp::Byte(_) => 2,
        Disp::Dword(_) => 5,
      },
      Memory::Reg(_) => 1,
      Memory::RipRel(_) => 5,
    };
    rex + opcode_size + sib_disp + imm_size
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn assembler() -> ErrOR<()> {
    use crate::OpQ::{Iq, Rq};
    use std::fs::File;
    use std::io::Write as _;
    let dlls = vec![("kernel32.dll", vec![(0x0167, "ExitProcess")])];
    let sample = &[
      Bss(0, 1),
      Quad(1, 1),
      Lbl(2),
      MovQQ(Rq(Rcx), Iq(0xCAFE_BABE)),
      Call(3),
      CallApi((0, 0)),
      Lbl(3),
      MovQQ(Rq(Rcx), Iq(0xDEAD_BEEF)),
      Ret,
    ];
    let assembler = Assembler::new(dlls);
    let pe = assembler.assemble_and_link(sample.iter().chain([Byte(100, 0)].iter()))?;
    let mut fun = File::create(".ignore/out_minimal.exe")?;
    fun.write_all(&pe)?;
    #[expect(clippy::print_stdout)]
    {
      println!("Wrote out_minimal.exe ({} bytes)", pe.len());
    };
    Ok(())
  }
  #[test]
  fn encode_inst_reg_no_rex() {
    let result = Memory::Reg(Rcx).encode_romsd(vec![0x89], Rax, false);
    assert_eq!(result, vec![0x89, 0b1100_0001]);
  }
  #[test]
  fn encode_inst_reg_with_rex() {
    let result = Memory::Reg(R9).encode_romsd(vec![0x89], R8, true);
    assert_eq!(result, vec![0x4D, 0x89, 0b1100_0001]);
  }
  #[test]
  fn encode_inst_riprel() {
    let result = Memory::RipRel(0x1234).encode_romsd(vec![], Rcx, false);
    assert_eq!(result, vec![0x0D, 0x34, 0x12, 0x00, 0x00]);
  }
  #[test]
  fn encode_inst_sib_mem0_disp8() {
    use crate::Scale;
    let sib = Sib { scale: Scale::S1, index: Rax, base: Rbp, disp: Disp::Byte(0x12) };
    let result = Memory::Sib(sib).encode_romsd(vec![], Rdx, false);
    assert_eq!(result, vec![0b01_010_100, 0b00_000_101, 0x12]);
  }
}
