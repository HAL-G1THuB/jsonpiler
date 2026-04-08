#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Disp {
  Byte(i8),
  Dword(i32),
  Zero,
}
impl Disp {
  pub(crate) fn encode(self, base_bits: u8) -> Vec<u8> {
    match self {
      Disp::Byte(int) => vec![int.cast_unsigned()],
      Disp::Dword(int) => int.to_le_bytes().to_vec(),
      Disp::Zero if base_bits == 5 => vec![0],
      Disp::Zero => vec![],
    }
  }
  pub(crate) fn from(offset: i32) -> Self {
    if offset == 0 {
      Disp::Zero
    } else if let Ok(s8) = i8::try_from(offset) {
      Disp::Byte(s8)
    } else {
      Disp::Dword(offset)
    }
  }
  pub(crate) fn sizeof(self, base_bits: u8) -> u32 {
    u32::from(self.to_mod(base_bits).pow(2))
  }
  pub(crate) fn to_mod(self, base_bits: u8) -> u8 {
    match self {
      Disp::Byte(_) => 1,
      Disp::Dword(_) => 2,
      Disp::Zero if base_bits == 5 => 1,
      Disp::Zero => 0,
    }
  }
}
