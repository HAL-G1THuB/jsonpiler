//! Implementation of the `Json`.
use crate::{
  ErrOR, Name, ScopeInfo,
  VarKind::{Global, Local, Tmp},
};
use core::fmt::{self, Display};
impl Name {
  /// Try to free and convert to string.
  pub(crate) fn try_free_and_2str(&self, info: &mut ScopeInfo) -> ErrOR<String> {
    if self.var == Tmp {
      info.free(self.seed, 8)?;
    }
    Ok(format!("qword{self}"))
  }
}
impl Display for Name {
  #[expect(clippy::min_ident_chars, reason = "default name is 'f'")]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self.var {
      Global => write!(f, " ptr .L{:x}[rip]", self.seed),
      Local | Tmp => write!(f, " ptr -0x{:x}[rbp]", self.seed),
    }
  }
}
