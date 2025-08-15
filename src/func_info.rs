use crate::{ErrOR, FuncInfo, Json, Label, VarKind::Tmp, WithPos};
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
