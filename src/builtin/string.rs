use crate::{
  Arity::{AtLeast, Exactly},
  Bind::*,
  ErrOR, FuncInfo, Json, Jsonpiler,
  Reg::*,
  ScopeInfo, built_in, take_arg,
};
built_in! {self, func, _scope, string;
  f_concat =>{"concat", COMMON, AtLeast(1), {
    let mut result = take_arg!(self, func, "String (Literal)", Json::String(Lit(x)) => x).value;
    for _ in 1..func.len {
      result.push_str(&take_arg!(self, func, "String (Literal)", Json::String(Lit(x)) => x).value);
    }
    Ok(Json::String(Lit(result)))
  }},
  len =>{"len", COMMON, Exactly(1), {
    self.take_len(Rax, func, _scope)?;
    Ok(Json::Int(Var(_scope.mov_tmp(Rax)?)))
    /*
    const CLD_REPNE_SCASB: [u8; 3] = [0xFC, 0xF2, 0xAE];
    _scope.extend(&[
      MovQQ(Rq(Rdi), Rq(Rsi)),
      Clear(Rcx),
      DecR(Rcx),
      Clear(Rax),
      Custom(CLD_REPNE_SCASB.to_vec()),
      SubRR(Rdi, Rsi),
      DecR(Rdi)
    ]);
    Ok(Json::Int(Var(_scope.mov_tmp(Rdi)?)))
    */
  }}
}
