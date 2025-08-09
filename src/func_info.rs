use crate::{ErrOR, FuncInfo, Json, Label, VarKind::Tmp, WithPos};
impl FuncInfo {
  pub fn arg(&mut self) -> ErrOR<WithPos<Json>> {
    self.args.next().ok_or("InternalError: Invalid argument reference".into())
  }
  pub fn sched_free_tmp(&mut self, label: &Label) {
    if label.kind == Tmp {
      self.free_list.push(*label);
    }
  }
}
