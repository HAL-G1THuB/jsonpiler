use super::super::{
  ArgLen::{Exactly, SomeArg},
  Bind::{Lit, Var},
  ErrOR, FuncInfo, Json, Jsonpiler, ScopeInfo, mn,
  utility::get_bool_str,
  validate_type,
};
impl Jsonpiler {
  pub(crate) fn logical(&mut self) {
    let common = (false, false);
    self.register("or", common, Jsonpiler::or, SomeArg);
    self.register("and", common, Jsonpiler::and, SomeArg);
    self.register("xor", common, Jsonpiler::xor, SomeArg);
    self.register("not", common, Jsonpiler::not, Exactly(1));
  }
}
#[expect(clippy::single_call_fn, reason = "")]
impl Jsonpiler {
  fn and(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.logical_template("and", func, scope)
  }
  fn logical_template(&mut self, mn: &str, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    let mut arg = func.arg()?;
    let mut boolean = validate_type!(self, func, 1, arg, Json::Bool(x) => x, "Bool");
    let mut bool_str = get_bool_str(&boolean, func);
    scope.body.push(mn!("mov", "al", bool_str));
    for ord in 2..=func.len {
      arg = func.arg()?;
      boolean = validate_type!(self, func, ord, arg, Json::Bool(x) => x, "Bool");
      bool_str = get_bool_str(&boolean, func);
      scope.body.push(mn!(mn, "al", bool_str));
    }
    scope.mov_tmp_bool("al")
  }
  fn not(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    let arg = func.arg()?;
    let bind = validate_type!(self, func, 1, arg, Json::Bool(x) => x, "Bool");
    match bind {
      Lit(l_bool) => Ok(Json::Bool(Lit(!l_bool))),
      Var(var) => {
        scope.body.push(mn!("mov", "al", var));
        scope.body.push(mn!("not", "al"));
        scope.mov_tmp_bool("al")
      }
    }
  }
  fn or(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.logical_template("or", func, scope)
  }
  fn xor(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.logical_template("xor", func, scope)
  }
}
