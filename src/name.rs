//! Implementation of the `Json`.
use crate::{
  ErrOR, Name, ScopeInfo,
  VarKind::{Global, Local, Tmp},
};
use core::fmt::{self, Display};
impl Name {
  /// Describes summary of the `Name`.
  pub(crate) fn describe(&self) -> &str {
    match self.var {
      Tmp => "Temporary local variable",
      Local => "Local variable",
      Global => "Global variable",
    }
  }
  /// Generates a label.
  pub(crate) fn to_def(&self) -> String {
    format!(".L{:x}:\n", self.id)
  }
  /// Generates a label.
  pub(crate) fn to_ref(&self) -> String {
    format!(".L{:x}", self.id)
  }
  /// Tries to free and convert to string.
  pub(crate) fn try_free_and_2str(&self, scope: &mut ScopeInfo) -> ErrOR<String> {
    if self.var == Tmp {
      scope.free(self.id, 8)?;
    }
    Ok(format!("qword{self}"))
  }
}
impl Display for Name {
  #[expect(clippy::min_ident_chars, reason = "default name is 'f'")]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self.var {
      Global => write!(f, " ptr {}[rip]", self.to_ref()),
      Local | Tmp => write!(f, " ptr -{:#x}[rbp]", self.id),
    }
  }
}
