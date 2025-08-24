use crate::{
  Arity::{AtLeast, Exactly},
  Bind::{Lit, Var},
  ErrOR, FuncInfo,
  Inst::*,
  Json, Jsonpiler,
  Reg::*,
  ScopeInfo, built_in, take_arg,
};
built_in! {self, func, scope, logical;
  and => {"and", COMMON, AtLeast(2), {
    self.mov_bool(Rax, func, scope)?;
    for _ in 1..func.len {
    let boolean = take_arg!(self, func, "Bool", Json::Bool(x) => x).0;
      match boolean {
      Lit(l_bool) => scope.push(MovRbIb(Rcx, if l_bool { 0xFF } else { 0 })),
      Var(label) => {
        func.sched_free_tmp(&label);
        scope.push(MovRbMb(Rcx, label.kind));
      }
    }
    scope.push(AndRbRb(Rax, Rcx));
    }
    scope.mov_tmp_bool(Rax)
  }},
  not => {"not", COMMON, Exactly(1), {
    let bind = take_arg!(self, func, "Bool", Json::Bool(x) => x).0;
    match bind {
      Lit(l_bool) => Ok(Json::Bool(Lit(!l_bool))),
      Var(var) => {
        scope.push(MovRbMb(Rax, var.kind));
        scope.push(NotRb(Rax));
    scope.mov_tmp_bool(Rax)
      }
    }
  }},
  or => {"or", COMMON, AtLeast(2), {
    self.mov_bool(Rax, func, scope)?;
    for _ in 1..func.len {
    let boolean = take_arg!(self, func, "Bool", Json::Bool(x) => x).0;
      match boolean {
      Lit(l_bool) => scope.push(MovRbIb(Rcx, if l_bool { 0xFF } else { 0 })),
      Var(label) => {
        func.sched_free_tmp(&label);
        scope.push(MovRbMb(Rcx, label.kind));
      }
    }
    scope.push(OrRbRb(Rax, Rcx));
    }
    scope.mov_tmp_bool(Rax)
  }},
  xor => {"xor", COMMON, AtLeast(2), {
    self.mov_bool(Rax, func, scope)?;
    for _ in 1..func.len {
    let boolean = take_arg!(self, func, "Bool", Json::Bool(x) => x).0;
      match boolean {
      Lit(l_bool) => scope.push(MovRbIb(Rcx, if l_bool { 0xFF } else { 0 })),
      Var(label) => {
        func.sched_free_tmp(&label);
        scope.push(MovRbMb(Rcx, label.kind));
      }
    }
    scope.push(XorRbRb(Rax, Rcx));
    }
    scope.mov_tmp_bool(Rax)
  }}
}
