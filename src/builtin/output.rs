use super::super::{
  ArgLen::Exactly,
  Bind::{Lit, Var},
  ErrOR, FuncInfo, Json, Jsonpiler, ScopeInfo, include_once,
};
use crate::validate_type;
impl Jsonpiler {
  pub(crate) fn register_output(&mut self) {
    let common = (false, false);
    self.register("message", common, Jsonpiler::message, Exactly(2));
  }
}
#[expect(clippy::single_call_fn, reason = "")]
impl Jsonpiler {
  fn message(&mut self, mut func: FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    scope.use_reg("rdi");
    scope.use_reg("rsi");
    let title_json = func.arg()?;
    let title = match validate_type!(self, func, 1, title_json, Json::String(x) => x, "String") {
      Lit(l_str) => self.get_global_str(&l_str)?,
      Var(label) => label,
    };
    let msg_json = func.arg()?;
    let msg = match validate_type!(self, func, 1, msg_json, Json::String(x) => x, "String") {
      Lit(l_str) => self.get_global_str(&l_str)?,
      Var(label) => label,
    };
    let ret = scope.get_tmp(8)?;
    include_once!(self, self.text, "func/U8TO16");
    scope.body.push(format!(
      include_str!("../asm/caller/message.s"),
      title = title,
      msg = msg,
      ret = ret
    ));
    Ok(Json::Int(Var(ret)))
  }
}
