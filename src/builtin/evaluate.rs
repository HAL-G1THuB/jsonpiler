use super::super::{ArgLen::{Any, Exactly}, ErrOR, FuncInfo, Json, Jsonpiler, ScopeInfo,Bind::Lit};
use core::mem::take;
impl Jsonpiler {
  pub(crate) fn evaluate(&mut self) {
    let common = (false, false);
    let special = (false, true);
    self.register("'", special, Jsonpiler::quote, Exactly(1));
    self.register("eval", common, Jsonpiler::f_eval, Exactly(1));
    self.register("list", common, Jsonpiler::list, Any);
    self.register("value", common, Jsonpiler::value, Exactly(1));
  }
}
#[expect(clippy::single_call_fn, clippy::unused_self, reason = "")]
impl Jsonpiler {
  fn f_eval(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.eval(func.arg()?.value, scope)
  }
  #[expect(clippy::unnecessary_wraps, reason = "")]
  fn list(&mut self, func: &mut FuncInfo, _: &mut ScopeInfo) -> ErrOR<Json> {
    Ok(Json::Array(Lit(Vec::from(take(&mut func.args)))))
  }
  fn quote(&mut self, func: &mut FuncInfo, _: &mut ScopeInfo) -> ErrOR<Json> {
    Ok(func.arg()?.value)
  }
  fn value(&mut self, func: &mut FuncInfo, _: &mut ScopeInfo) -> ErrOR<Json> {
    func.arg().map(|x| x.value)
  }
}
