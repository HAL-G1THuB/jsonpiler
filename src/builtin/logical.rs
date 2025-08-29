use crate::{
  Arity::{AtLeast, Exactly},
  Bind::{Lit, Var},
  ConditionCode::*,
  ErrOR, FuncInfo,
  Inst::*,
  Json, Jsonpiler,
  OpQ::*,
  Reg::*,
  ScopeInfo, built_in, take_arg,
};
built_in! {self, func, scope, logical;
  and => {"and", COMMON, AtLeast(2), {
    self.take_bool(Rax, func, scope)?;
    for _ in 1..func.len {
    let boolean = take_arg!(self, func, "Bool", Json::Bool(x) => x).value;
      match boolean {
      Lit(l_bool) => scope.push(MovRbIb(Rcx, if l_bool { 0xFF } else { 0 })),
      Var(label) => {
        scope.push(MovRbMb(Rcx, label.kind));
      }
    }
    scope.push(AndRbRb(Rax, Rcx));
    }
    scope.mov_tmp_bool(Rax)
  }},
  assert => {"assert", COMMON, Exactly(2), {
    let message_box_w = self.import(Jsonpiler::USER32, "MessageBoxW", 0x28c);
    self.take_bool(Rax, func, scope)?;
    scope.push(TestRbRb(Rax, Rax));
    let error_label = self.gen_id();
    let end_label = self.gen_id();
    scope.push(Jcc(E, error_label));
    scope.push(Jmp(end_label));
    scope.push(Lbl(error_label));
    self.take_str(Rcx, func, scope)?;
    scope.push(Call(self.get_u8_to_16()));
    scope.push(MovQQ(Rq(Rdx), Rq(Rax)));
    scope.push(Clear(Rcx));
    scope.push(Clear(R8));
    scope.push(MovRId(R9, 0x10));
    scope.extend(&self.call_api_check_null(message_box_w));
    scope.push(MovRId(Rcx, 1));
    scope.push(CallApi(self.import(Jsonpiler::KERNEL32, "ExitProcess", 0x167)));
    scope.push(Lbl(end_label));
    Ok(Json::Null)
  }},
  not => {"not", COMMON, Exactly(1), {
    let bind = take_arg!(self, func, "Bool", Json::Bool(x) => x).value;
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
    self.take_bool(Rax, func, scope)?;
    for _ in 1..func.len {
    let boolean = take_arg!(self, func, "Bool", Json::Bool(x) => x).value;
      match boolean {
      Lit(l_bool) => scope.push(MovRbIb(Rcx, if l_bool { 0xFF } else { 0 })),
      Var(label) => {
        scope.push(MovRbMb(Rcx, label.kind));
      }
    }
    scope.push(OrRbRb(Rax, Rcx));
    }
    scope.mov_tmp_bool(Rax)
  }},
  xor => {"xor", COMMON, AtLeast(2), {
    self.take_bool(Rax, func, scope)?;
    for _ in 1..func.len {
    let boolean = take_arg!(self, func, "Bool", Json::Bool(x) => x).value;
      match boolean {
      Lit(l_bool) => scope.push(MovRbIb(Rcx, if l_bool { 0xFF } else { 0 })),
      Var(label) => {
        scope.push(MovRbMb(Rcx, label.kind));
      }
    }
    scope.push(XorRbRb(Rax, Rcx));
    }
    scope.mov_tmp_bool(Rax)
  }}
}
