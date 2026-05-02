use crate::prelude::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
#[expect(clippy::min_ident_chars)]
pub(crate) enum ConditionCode {
  O = 0,
  #[expect(dead_code)]
  No = 1,
  B = 2,
  Ae = 3,
  E = 4,
  Ne = 5,
  Be = 6,
  A = 7,
  #[expect(dead_code)]
  S = 8,
  #[expect(dead_code)]
  Ns = 9,
  #[expect(dead_code)]
  P = 10,
  #[expect(dead_code)]
  Np = 11,
  L = 12,
  Ge = 13,
  Le = 14,
  G = 15,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Logic {
  And = 0x22,
  Or = 0x0A,
  Xor = 0x32,
  Cmp = 0x3A,
  Test = 0x84,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnaryKind {
  Neg,
  Not,
}
impl UnaryKind {
  pub(crate) fn reg_field(self) -> Register {
    match self {
      Neg => Rbx,
      Not => Rdx,
    }
  }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArithSdKind {
  Add = 0x58,
  Div = 0x5E,
  Mul = 0x59,
  Sub = 0x5C,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operand<T> {
  Args(i32),
  Imm(T),
  Mem(Address),
  Ref(Register),
  Reg(Register),
  SibDisp(Sib, Disp),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShiftDirection {
  #[expect(dead_code)]
  Sar,
  Shl,
  Shr,
}
impl ShiftDirection {
  pub(crate) fn reg_field(self) -> Register {
    match self {
      Shl => Rsp,
      Shr => Rbp,
      Sar => Rdi,
    }
  }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
