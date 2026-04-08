use crate::prelude::*;
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[expect(clippy::arbitrary_source_item_ordering)]
pub(crate) enum Register {
  Rax = 0,
  Rcx = 1,
  Rdx = 2,
  Rbx = 3,
  Rsp = 4,
  Rbp = 5,
  Rsi = 6,
  Rdi = 7,
  R8 = 8,
  R9 = 9,
  R10 = 10,
  R11 = 11,
  R12 = 12,
  R13 = 13,
  R14 = 14,
  R15 = 15,
}
impl Register {
  pub(crate) fn encode_plus_reg(self, prefix: &[u8], rex_w: u8, opc: u8, imm: &[u8]) -> Vec<u8> {
    let mut code = prefix.to_vec();
    if self.rex() | rex_w == 1 {
      code.push(0x40 + (rex_w << 3) + self.rex());
    }
    code.push(opc + self.reg_bits());
    code.extend_from_slice(imm);
    code
  }
  pub(crate) fn rb(self) -> ErrOR<Self> {
    if self < Rsp || Rdi < self {
      Ok(self)
    } else {
      Err(Internal(InvalidInst("spl, bpl ,sil and dil".into())))
    }
  }
  pub(crate) fn reg_bits(self) -> u8 {
    self as u8 & 7
  }
  pub(crate) fn rex(self) -> u8 {
    u8::from(R8 <= self)
  }
  pub(crate) fn rex_size(self) -> u32 {
    u32::from(R8 <= self)
  }
}
