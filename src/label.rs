use crate::{
  FuncInfo, Label,
  VarKind::{Global, Local, Tmp},
};
use core::fmt::{self, Display};
impl Label {
  pub(crate) fn describe(&self) -> &str {
    match self.kind {
      Tmp => "Temporary value",
      Local => "Local variable",
      Global => "Global variable",
    }
  }
  pub(crate) fn sched_free_2str(&self, scope: &mut FuncInfo) -> String {
    scope.sched_free_tmp(self);
    format!("{self}")
  }
  pub(crate) fn to_def(self) -> String {
    format!(".L{:x}:\n", self.id)
  }
  pub(crate) fn to_ref(self) -> String {
    format!(".L{:x}", self.id)
  }
}
impl Display for Label {
  #[expect(clippy::min_ident_chars, reason = "default name is 'f'")]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self.size {
      1 => write!(f, "byte"),
      2 => write!(f, "word"),
      4 => write!(f, "dword"),
      8 => write!(f, "qword"),
      _ => Err(fmt::Error),
    }?;
    write!(f, " ptr ")?;
    match self.kind {
      Global => write!(f, "{}[rip]", self.to_ref()),
      Local | Tmp => {
        write!(f, "-{:#x}[rbp]", self.id)
      }
    }
  }
}
