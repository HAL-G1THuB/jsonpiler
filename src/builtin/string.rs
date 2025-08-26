use crate::{
  Arity::{AtLeast, Exactly},
  Bind::*,
  ErrOR, FuncInfo,
  Inst::*,
  Json, Jsonpiler,
  OpQ::Rq,
  Reg::*,
  ScopeInfo, built_in, take_arg,
};
built_in! {self, func, _scope, string;
  f_concat =>{"concat", COMMON, AtLeast(1), {
    let mut result = take_arg!(self, func, "String (Literal)", Json::String(Lit(x)) => x).0;
    for _ in 1..func.len {
      result.push_str(&take_arg!(self, func, "String (Literal)", Json::String(Lit(x)) => x).0);
    }
    Ok(Json::String(Lit(result)))
  }},
  len =>{"len", COMMON, Exactly(1), {
    const CLD_REPNE_SCASB: [u8; 3] = [0xFC, 0xF2, 0xAE];
    self.take_str(Rsi, func, _scope)?;
    _scope.extend(&[
      MovQQ(Rq(Rdi), Rq(Rsi)),
      Clear(Rcx),
      DecQ(Rcx),
      Clear(Rax),
      Custom(CLD_REPNE_SCASB.to_vec()),
      SubRR(Rdi, Rsi),
      DecQ(Rdi)
    ]);
    Ok(Json::Int(Var(_scope.mov_tmp(Rdi)?)))
  }}
}
