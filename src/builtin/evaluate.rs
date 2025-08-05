use super::super::{ArgLen::Exactly, ErrOR, FuncInfo, Json, Jsonpiler, ScopeInfo};
impl Jsonpiler {
  pub(crate) fn register_evaluate(&mut self) {
    let common = (false, false);
    let special = (false, true);
    self.register("'", special, Jsonpiler::quote, Exactly(1));
    self.register("eval", common, Jsonpiler::f_eval, Exactly(1));
  }
}
#[expect(clippy::single_call_fn, reason = "")]
impl Jsonpiler {
  fn f_eval(&mut self, mut func: FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.eval(func.arg()?.value, scope)
  }
  #[expect(clippy::unused_self, reason = "")]
  fn quote(&mut self, mut func: FuncInfo, _: &mut ScopeInfo) -> ErrOR<Json> {
    Ok(func.arg()?.value)
  }
}
