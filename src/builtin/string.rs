use crate::{
  Arity::AtLeast, Bind::Lit, ErrOR, FuncInfo, Json, Jsonpiler, ScopeInfo, built_in, take_arg,
};
built_in! {self, func, _scope, string;
  f_concat =>{"concat", COMMON, AtLeast(1), {
    let mut result = take_arg!(self, func, 1, "String (Literal)", Json::String(Lit(x)) => x).0;
    for nth in 1..func.len {
      result.push_str(&take_arg!(self, func, nth, "String (Literal)", Json::String(Lit(x)) => x).0);
    }
    Ok(Json::String(Lit(result)))
  }}
}
