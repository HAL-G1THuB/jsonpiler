use crate::{
  Bind::{self, Lit, Var},
  ErrOR, FuncInfo, Json, Label,
  VarKind::{self, *},
  WithPos,
};
use core::fmt::{self, Display};
impl<T> Bind<T> {
  pub(crate) fn describe(&self, ty: &str) -> String {
    format!(
      "{ty} ({})",
      match self {
        Lit(_) => "Literal",
        Var(label) => label.describe(),
      }
    )
  }
}
impl FuncInfo {
  pub(crate) fn arg(&mut self) -> ErrOR<WithPos<Json>> {
    self.args.next().ok_or("InternalError: Invalid argument reference".into())
  }
  pub(crate) fn sched_free_tmp(&mut self, label: &Label) {
    if let Label { kind: Tmp { offset }, size } = label {
      self.free_list.push((*offset, *size));
    }
  }
}
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
    f.write_str(match self.size {
      1 => "byte",
      2 => "word",
      4 => "dword",
      8 => "qword",
      _ => return Err(fmt::Error),
    })?;
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
