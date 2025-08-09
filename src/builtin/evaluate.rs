use crate::{
  Arity::{Any, Exactly},
  Bind::Lit,
  ErrOR, FuncInfo, Json, Jsonpiler, ScopeInfo, built_in,
};
use core::mem::take;
built_in! {self, func, _scope, evaluate;
  f_eval => {"eval", COMMON, Exactly(1), {
    self.eval(func.arg()?, _scope)
  }},
  list => {"list", COMMON, Any, {
    Ok(Json::Array(Lit(take(&mut func.args).collect())))
  }},
  quote => {"'", SPECIAL, Exactly(1), {
    Ok(func.arg()?.value)
  }},
  value => {"value", COMMON, Exactly(1), {
    func.arg().map(|x| x.value)
  }}
}
