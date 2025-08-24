use crate::{
  Arity::AtLeast, Bind::Var, ConditionCode::*, ErrOR, FuncInfo, Inst::*, Json, Jsonpiler, OpQ::Rq,
  Reg::*, ScopeInfo, built_in,
};
built_in! {self, func, scope, compare;
  eq => {"==", COMMON, AtLeast(2), {
    self.mov_int(Rax, func, scope)?;
    let false_label = self.gen_id();
    for _ in 1..func.len {
      self.mov_int(Rcx, func, scope)?;
      scope.push(CmpRR(Rax, Rcx));
      scope.push(Jcc(Ne, false_label));
      scope.push(MovQQ(Rq(Rax), Rq(Rcx)));
    }
    let end_label = self.gen_id();
    let return_value = scope.tmp(1)?;
    scope.push(MovMbIb(return_value.kind, 0xFF));
    scope.push(JmpSh(end_label));
    scope.push(Lbl(false_label));
    scope.push(MovMbIb(return_value.kind, 0));
    scope.push(Lbl(end_label));
    Ok(Json::Bool(Var(return_value)))
  }},
  less => {"<", COMMON, AtLeast(2), {
    self.mov_int(Rax, func, scope)?;
    let false_label = self.gen_id();
    for _ in 1..func.len {
      self.mov_int(Rcx, func, scope)?;
      scope.push(CmpRR(Rax, Rcx));
      scope.push(Jcc(Ge, false_label));
      scope.push(MovQQ(Rq(Rax), Rq(Rcx)));
    }
    let end_label = self.gen_id();
    let return_value = scope.tmp(1)?;
    scope.push(MovMbIb(return_value.kind, 0xFF));
    scope.push(JmpSh(end_label));
    scope.push(Lbl(false_label));
    scope.push(MovMbIb(return_value.kind, 0));
    scope.push(Lbl(end_label));
    Ok(Json::Bool(Var(return_value)))
  }},
  less_eq => {"<=", COMMON, AtLeast(2), {
    self.mov_int(Rax, func, scope)?;
    let false_label = self.gen_id();
    for _ in 1..func.len {
      self.mov_int(Rcx, func, scope)?;
      scope.push(CmpRR(Rax, Rcx));
      scope.push(Jcc(G, false_label));
      scope.push(MovQQ(Rq(Rax), Rq(Rcx)));
    }
    let end_label = self.gen_id();
    let return_value = scope.tmp(1)?;
    scope.push(MovMbIb(return_value.kind, 0xFF));
    scope.push(JmpSh(end_label));
    scope.push(Lbl(false_label));
    scope.push(MovMbIb(return_value.kind, 0));
    scope.push(Lbl(end_label));
    Ok(Json::Bool(Var(return_value)))
  }},
}
