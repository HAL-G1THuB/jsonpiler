use crate::prelude::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scale {
  S1 = 0,
  #[expect(dead_code)]
  S2 = 1,
  S4 = 2,
  #[expect(dead_code)]
  S8 = 3,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(clippy::arbitrary_source_item_ordering)]
pub(crate) struct Sib {
  pub scale: Scale,
  pub index: Register,
  pub base: Register,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RM {
  Base(Register, Disp),
  Reg(Register),
  RipRel(i32),
  Sib(Sib, Disp),
}
impl RM {
  pub(crate) fn encode(&self, rex_w: u8, opc: &[u8], reg: Register) -> Vec<u8> {
    let mut bytes = self.get_rex(rex_w, reg);
    bytes.extend_from_slice(opc);
    bytes.push((self.get_mod() << 6) | (reg.reg_bits() << 3) | self.get_rm());
    bytes.extend_from_slice(&self.get_sib());
    bytes.extend_from_slice(&self.get_disp());
    bytes
  }
  pub(crate) fn encode_ex(
    &self,
    prefix: u8,
    rex_w: u8,
    opc: &[u8],
    reg: Register,
    imm: &[u8],
  ) -> Vec<u8> {
    let mut code = vec![prefix];
    extend!(code, self.encode(rex_w, opc, reg), imm);
    code
  }
  pub(crate) fn encode_imm(&self, rex_w: u8, opc: &[u8], reg: Register, imm: &[u8]) -> Vec<u8> {
    let mut code = self.encode(rex_w, opc, reg);
    code.extend_from_slice(imm);
    code
  }
  pub(crate) fn get_disp(&self) -> Vec<u8> {
    match self {
      RM::Reg(_) => vec![],
      RM::RipRel(disp) => disp.to_le_bytes().to_vec(),
      RM::Base(base, disp) => disp.encode(base.reg_bits()),
      RM::Sib(sib, disp) => disp.encode(sib.base.reg_bits()),
    }
  }
  pub(crate) fn get_mod(&self) -> u8 {
    match self {
      RM::Reg(_) => 3,
      RM::RipRel(_) => 0,
      RM::Base(base, disp) => disp.to_mod(base.reg_bits()),
      RM::Sib(sib, disp) => disp.to_mod(sib.base.reg_bits()),
    }
  }
  pub(crate) fn get_rex(&self, rex_w: u8, reg: Register) -> Vec<u8> {
    let (rex_x, rex_b) = self.get_rex_x_b();
    let rex = 0x40 | (rex_w << 3) | (reg.rex() << 2) | (rex_x << 1) | rex_b;
    if rex == 0x40 { vec![] } else { vec![rex] }
  }
  pub(crate) fn get_rex_x_b(&self) -> (u8, u8) {
    match self {
      RM::Reg(reg2) => (0, reg2.rex()),
      RM::RipRel(_) => (0, 0),
      RM::Base(base, _) => (0, base.rex()),
      RM::Sib(sib, _) => (sib.index.rex(), sib.base.rex()),
    }
  }
  pub(crate) fn get_rm(&self) -> u8 {
    match self {
      RM::Reg(reg) => reg.reg_bits(),
      RM::RipRel(_) => 5,
      RM::Base(base, _) => base.reg_bits(),
      RM::Sib(..) => 4,
    }
  }
  pub(crate) fn get_sib(&self) -> Vec<u8> {
    match self {
      RM::Base(base, _) if base.reg_bits() == 4 => vec![0o44],
      RM::RipRel(_) | RM::Reg(_) | RM::Base(..) => vec![],
      RM::Sib(sib, _) => {
        vec![((sib.scale as u8) << 6) | (sib.index.reg_bits() << 3) | sib.base.reg_bits()]
      }
    }
  }
}
