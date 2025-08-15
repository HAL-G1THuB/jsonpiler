use crate::{
  Label,
  VarKind::{self, Global, Local, Tmp},
  utility::get_prefix,
};
use core::fmt::{self, Display};
impl Label {
  pub(crate) fn describe(&self) -> &str {
    match self.kind {
      Tmp { .. } => "Temporary value",
      Local { .. } => "Local variable",
      Global { .. } => "Global variable",
    }
  }
}
impl Display for Label {
  #[expect(clippy::min_ident_chars)]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(get_prefix(self.size).ok_or(fmt::Error)?)?;
    write!(f, "\tptr\t")?;
    match self.kind {
      Global { id } => write!(f, ".L{id:#X}[rip]"),
      Local { offset } | Tmp { offset } => {
        write!(f, "-{offset:#X}[rbp]")
      }
    }
  }
}
impl VarKind {
  pub(crate) fn size_of_mo_si_di(&self) -> u32 {
    match self {
      Global { .. } => 5,
      Local { .. } | Tmp { .. } => 6,
    }
  }
}
