use crate::prelude::*;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RM {
  Base(Register, Disp),
  Reg(Register),
  RipRel(i32),
  Sib(Sib, Disp),
}
impl RM {
  pub(crate) fn encode(&self, rex_w: u8, mut opc: Vec<u8>, reg: Register) -> Vec<u8> {
    let (rex_x, rex_b) = self.get_rex_x_b();
    if let Some(rex) =
      Some(0x40 | (rex_w << 3) | (reg.rex() << 2) | (rex_x << 1) | rex_b).filter(|rex| *rex != 0x40)
    {
      opc.insert(0, rex);
    }
    opc.push((self.get_mod() << 6) | (reg.reg_bits() << 3) | self.get_rm());
    if let Some(sib) = self.get_sib() {
      opc.push(sib);
    }
    opc.extend_from_slice(&self.get_disp());
    opc
  }
  pub(crate) fn encode_f2(&self, rex_w: u8, opc: Vec<u8>, reg: Register, imm: &[u8]) -> Vec<u8> {
    let mut code = vec![];
    extend!(code, [0xF2], self.encode(rex_w, opc, reg), imm);
    code
  }
  pub(crate) fn encode_imm(&self, rex_w: u8, opc: Vec<u8>, reg: Register, imm: &[u8]) -> Vec<u8> {
    let mut code = vec![];
    extend!(code, self.encode(rex_w, opc, reg), imm);
    code
  }
  pub(crate) fn get_disp(&self) -> Vec<u8> {
    match self {
      RM::Reg(_) => vec![],
      RM::RipRel(disp) => disp.to_le_bytes().into(),
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
  pub(crate) fn get_sib(&self) -> Option<u8> {
    match self {
      RM::RipRel(_) | RM::Reg(_) => None,
      RM::Base(base, _) => (base.reg_bits() == 4).then_some(0o44),
      RM::Sib(sib, _) => {
        Some(((sib.scale as u8) << 6) | (sib.index.reg_bits() << 3) | sib.base.reg_bits())
      }
    }
  }
}
