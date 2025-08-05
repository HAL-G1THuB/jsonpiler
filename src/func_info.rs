use crate::{ErrOR, FuncInfo, Json, WithPos};
impl FuncInfo {
  pub fn arg(&mut self) -> ErrOR<WithPos<Json>> {
    self.args.pop_front().ok_or("InternalError: Invalid argument reference".into())
  }
}
