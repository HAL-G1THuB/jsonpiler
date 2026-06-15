use crate::prelude::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shift16 {
  Lsl0 = 0,
  Lsl16 = 1,
  Lsl32 = 2,
  Lsl48 = 3,
}
#[expect(clippy::arbitrary_source_item_ordering)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum A64Cc {
  Eq = 0,
  Ne = 1,
  Cs = 2,
  Cc = 3,
  Mi = 4,
  Pl = 5,
  Vs = 6,
  Vc = 7,
  Hi = 8,
  Ls = 9,
  Ge = 0xA,
  Lt = 0xB,
  Gt = 0xC,
  Le = 0xD,
  Al = 0xE,
  Nv = 0xF,
}
impl From<X64Cc> for A64Cc {
  fn from(value: X64Cc) -> Self {
    match value {
      O => A64Cc::Vs,
      No => A64Cc::Vc,
      B => A64Cc::Cs,
      Ae => A64Cc::Cc,
      E => A64Cc::Eq,
      Ne => A64Cc::Ne,
      Be => A64Cc::Ls,
      A => A64Cc::Hi,
      S => A64Cc::Mi,
      Ns => A64Cc::Pl,
      L => A64Cc::Lt,
      Ge => A64Cc::Ge,
      Le => A64Cc::Le,
      G => A64Cc::Gt,
    }
  }
}
pub(crate) fn invert_cond(cond: A64Cc) -> A64Cc {
  use A64Cc::*;
  const PAIRS: &[(A64Cc, A64Cc)] =
    &[(Eq, Ne), (Cs, Cc), (Mi, Pl), (Vs, Vc), (Hi, Ls), (Ge, Lt), (Gt, Le), (Al, Nv)];
  for (c1, c2) in PAIRS {
    if cond == *c1 {
      return *c2;
    }
    if cond == *c2 {
      return *c1;
    }
  }
  Nv
}
