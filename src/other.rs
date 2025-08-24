use crate::{
  Bind::{self, Var},
  ErrOR, FuncInfo, Json, Label,
  VarKind::{self, *},
  WithPos,
};
impl<T> Bind<T> {
  pub(crate) fn describe(&self, ty: &str) -> String {
    format!("{ty} ({})", if let Var(label) = self { label.describe() } else { "Literal" })
  }
}
impl FuncInfo {
  pub(crate) fn arg(&mut self) -> ErrOR<WithPos<Json>> {
    self.nth += 1;
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
impl VarKind {
  pub(crate) fn size_of_mo_si_di(&self) -> u32 {
    match self {
      Global { .. } => 5,
      Local { .. } | Tmp { .. } => 6,
    }
  }
}
