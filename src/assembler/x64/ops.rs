use crate::prelude::*;
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
#[non_exhaustive]
#[expect(clippy::min_ident_chars)]
pub(crate) enum X64Cc {
  O = 0,
  #[expect(dead_code)]
  No = 1,
  B = 2,
  Ae = 3,
  E = 4,
  Ne = 5,
  Be = 6,
  A = 7,
  S = 8,
  #[expect(dead_code)]
  Ns = 9,
  // P = 10,
  // Np = 11,
  L = 12,
  Ge = 13,
  Le = 14,
  G = 15,
}
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub(crate) enum Group1 {
  Add = 0x00,
  Or = 0x08,
  And = 0x20,
  Sub = 0x28,
  Xor = 0x30,
  Cmp = 0x38,
}
impl Group1 {
  pub(crate) fn reg_field(self) -> X64Reg {
    match self {
      Add => Rax,
      Or => Rcx,
      And => Rsp,
      Sub => Rbp,
      Xor => Rsi,
      Cmp => Rdi,
    }
  }
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum UnaryKind {
  Neg,
  Not,
}
impl UnaryKind {
  pub(crate) fn reg_field(self) -> X64Reg {
    match self {
      Neg => Rbx,
      Not => Rdx,
    }
  }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArithSdKind {
  AddSd = 0x58,
  Div = 0x5E,
  Mul = 0x59,
  SubSd = 0x5C,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShiftDirection {
  Sar,
  Shl,
  Shr,
}
impl ShiftDirection {
  pub(crate) fn reg_field(self) -> X64Reg {
    match self {
      Shl => Rsp,
      Shr => Rbp,
      Sar => Rdi,
    }
  }
}
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy)]
pub(crate) enum BitTestKind {
  #[expect(dead_code)]
  Bt,
  Btc,
  Btr,
  #[expect(dead_code)]
  Bts,
}
impl BitTestKind {
  pub(crate) fn reg_field(self) -> X64Reg {
    match self {
      Bt => Rsp,
      Bts => Rbp,
      Btr => Rsi,
      Btc => Rdi,
    }
  }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepeatKind {
  RepE = 0xF3,
  RepNe = 0xF2,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(clippy::arbitrary_source_item_ordering)]
pub(crate) enum StrInstKind {
  MovS = 0xA4,
  #[expect(dead_code)]
  CmpS = 0xA6,
  #[expect(dead_code)]
  StoS = 0xAA,
  #[expect(dead_code)]
  LodS = 0xAC,
  ScaS = 0xAE,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum X64Operand {
  Args(i32),
  Mem(Address),
  Ref(X64Reg),
  Reg(X64Reg),
  SibDisp(Sib, Disp),
}
impl From<X64Reg> for X64Operand {
  fn from(src: X64Reg) -> X64Operand {
    Reg(src)
  }
}
impl From<Address> for X64Operand {
  fn from(src: Address) -> X64Operand {
    Mem(src)
  }
}
impl X64Operand {
  pub(crate) fn rb(self) -> ErrOR<Self> {
    match self {
      Reg(reg) => {
        reg.rb()?;
      }
      Ref(_) | Mem(_) | SibDisp(_, _) | Args(_) => (),
    }
    Ok(self)
  }
  pub(crate) fn sizeof(self, rex: u32, opcode_len: u32) -> ErrOR<u32> {
    Ok(match self {
      Reg(reg) => (rex | reg.rex_size()) + opcode_len + 1,
      Ref(mem) => (rex | mem.rex_size()) + opcode_len + 1 + Disp::Zero.sizeof(mem.reg_bits()),
      Mem(addr) => rex + opcode_len + addr.modrm_sib_disp(),
      Args(nth) => rex + opcode_len + 2 + Disp::from((nth - 1) << 3).sizeof(Rsp as u8),
      SibDisp(sib, disp) => {
        (rex | sib.base.rex_size() | sib.index.rex_size())
          + opcode_len
          + 2
          + disp.sizeof(sib.base.reg_bits())
      }
    })
  }
}
impl X64Assembler {
  pub(crate) fn o2rm(&self, operand: X64Operand, offset: u32, inst: &X64Inst) -> ErrOR<ModRM> {
    Ok(match operand {
      Reg(reg) => ModRM::Reg(reg),
      Ref(mem) => ModRM::Base(mem, Disp::Zero),
      Mem(addr) => self.mem2rm(addr, offset, inst)?,
      Args(nth) => ModRM::Base(Rsp, Disp::from((nth - 1) << 3)),
      SibDisp(sib, disp) => ModRM::Sib(sib, disp),
    })
  }
}
