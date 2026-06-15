use crate::prelude::*;
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub(crate) enum Scale {
//   S1 = 0,
//   S2 = 1,
//   S4 = 2,
//   S8 = 3,
// }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(clippy::arbitrary_source_item_ordering)]
pub(crate) struct Sib {
  pub scale: RegSize,
  pub index: X64Reg,
  pub base: X64Reg,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModRM {
  Base(X64Reg, Disp),
  Reg(X64Reg),
  RipRel(i32),
  Sib(Sib, Disp),
}
impl ModRM {
  pub(crate) fn enc(&self, rex_w: u8, opc: &[u8], reg: X64Reg) -> Vec<u8> {
    let mut bytes = self.get_rex(rex_w, reg);
    bytes.extend_from_slice(opc);
    bytes.push((self.get_mod() << 6) | (reg.reg_bits() << 3) | self.get_rm());
    bytes.extend_from_slice(&self.get_sib());
    bytes.extend_from_slice(&self.get_disp());
    bytes
  }
  pub(crate) fn enc_ex(
    &self,
    prefix: u8,
    rex_w: u8,
    opc: &[u8],
    reg: X64Reg,
    imm: &[u8],
  ) -> Vec<u8> {
    let mut code = vec![prefix];
    extend!(code, self.enc(rex_w, opc, reg), imm);
    code
  }
  pub(crate) fn enc_imm(&self, rex_w: u8, opc: &[u8], reg: X64Reg, imm: &[u8]) -> Vec<u8> {
    let mut code = self.enc(rex_w, opc, reg);
    code.extend_from_slice(imm);
    code
  }
  pub(crate) fn get_disp(&self) -> Vec<u8> {
    match self {
      ModRM::Reg(_) => vec![],
      ModRM::RipRel(disp) => disp.to_le_bytes().to_vec(),
      ModRM::Base(base, disp) => disp.encode(base.reg_bits()),
      ModRM::Sib(sib, disp) => disp.encode(sib.base.reg_bits()),
    }
  }
  pub(crate) fn get_mod(&self) -> u8 {
    match self {
      ModRM::Reg(_) => 3,
      ModRM::RipRel(_) => 0,
      ModRM::Base(base, disp) => disp.to_mod(base.reg_bits()),
      ModRM::Sib(sib, disp) => disp.to_mod(sib.base.reg_bits()),
    }
  }
  pub(crate) fn get_rex(&self, rex_w: u8, reg: X64Reg) -> Vec<u8> {
    let (rex_x, rex_b) = self.get_rex_x_b();
    let rex = 0x40 | (rex_w << 3) | (reg.rex() << 2) | (rex_x << 1) | rex_b;
    if rex == 0x40 { vec![] } else { vec![rex] }
  }
  pub(crate) fn get_rex_x_b(&self) -> (u8, u8) {
    match self {
      ModRM::Reg(reg2) => (0, reg2.rex()),
      ModRM::RipRel(_) => (0, 0),
      ModRM::Base(base, _) => (0, base.rex()),
      ModRM::Sib(sib, _) => (sib.index.rex(), sib.base.rex()),
    }
  }
  pub(crate) fn get_rm(&self) -> u8 {
    match self {
      ModRM::Reg(reg) => reg.reg_bits(),
      ModRM::RipRel(_) => 5,
      ModRM::Base(base, _) => base.reg_bits(),
      ModRM::Sib(..) => 4,
    }
  }
  pub(crate) fn get_sib(&self) -> Vec<u8> {
    match self {
      ModRM::Base(base, _) if base.reg_bits() == 4 => vec![0o44],
      ModRM::RipRel(_) | ModRM::Reg(_) | ModRM::Base(..) => vec![],
      ModRM::Sib(sib, _) => {
        vec![
          (match sib.scale {
            S1 => 0,
            // S2 => 1,
            S4 => 2,
            S8 => 3,
          } << 6)
            | (sib.index.reg_bits() << 3)
            | sib.base.reg_bits(),
        ]
      }
    }
  }
}
