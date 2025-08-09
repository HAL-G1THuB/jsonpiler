use crate::{
  Arity::{AtLeast, Exactly},
  Bind::{Lit, Var},
  ErrOR, FuncInfo, Json, Jsonpiler, ScopeInfo, built_in, mn, take_arg,
};
built_in! {self, func, scope, logical;
  and => {"and", COMMON, AtLeast(2), {
        self.logical_template("and", func, scope)
  }},
  not => {"not", COMMON, Exactly(1), {
    let bind = take_arg!(self, func, 1,"Bool", Json::Bool(x) => x).0;
    match bind {
      Lit(l_bool) => Ok(Json::Bool(Lit(!l_bool))),
      Var(var) => {
        scope.body.push(mn!("mov", "al", var));
        scope.body.push(mn!("not", "al"));
        scope.mov_tmp_bool("al")
      }
    }
  }},
  or => {"or", COMMON, AtLeast(2), {
        self.logical_template("or", func, scope)
  }},
  xor => {"xor", COMMON, AtLeast(2), {
        self.logical_template("xor", func, scope)
  }}
}
impl Jsonpiler {
  fn logical_template(
    &mut self, mn: &str, func: &mut FuncInfo, scope: &mut ScopeInfo,
  ) -> ErrOR<Json> {
    let mut bool_str = self.get_bool_str(func, 1)?;
    scope.body.push(mn!("mov", "al", bool_str));
    for nth in 2..=func.len {
      bool_str = self.get_bool_str(func, nth)?;
      scope.body.push(mn!(mn, "al", bool_str));
    }
    scope.mov_tmp_bool("al")
  }
}
