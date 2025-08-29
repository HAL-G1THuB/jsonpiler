use crate::{
  Arity::AtLeast, Bind::Var, ConditionCode::*, ErrOR, FuncInfo, Inst::*, Json, Jsonpiler, Reg::*,
  ScopeInfo, built_in,
};
built_in! {self, func, scope, compare;
  eq => {"==", COMMON, AtLeast(2), {
    self.take_int(Rax, func, scope)?;
    let false_label = self.gen_id();
    for idx in 1..func.len {
      self.take_int(if idx % 2 == 0 { Rax } else { Rcx }, func, scope)?;
      scope.extend(&[CmpRR(Rax, Rcx), Jcc(Ne, false_label)]);
    }
    let end_label = self.gen_id();
    let return_value = scope.tmp(1, 1)?;
    scope.extend(&[
      MovMbIb(return_value.kind, 0xFF),
      JmpSh(end_label),
      Lbl(false_label),
      MovMbIb(return_value.kind, 0),
      Lbl(end_label)
    ]);
    Ok(Json::Bool(Var(return_value)))
  }},
  grater => {">", COMMON, AtLeast(2), {
    self.take_int(Rax, func, scope)?;
    let false_label = self.gen_id();
    for idx in 1..func.len {
      self.take_int(if idx % 2 == 0 { Rax } else { Rcx }, func, scope)?;
      scope.extend(&[
        CmpRR(Rax, Rcx),
        Jcc(if idx % 2 == 0 { Ge } else { L }, false_label),
      ]);
    }
    let end_label = self.gen_id();
    let return_value = scope.tmp(1, 1)?;
    scope.extend(&[
      MovMbIb(return_value.kind, 0xFF),
      JmpSh(end_label),
      Lbl(false_label),
      MovMbIb(return_value.kind, 0),
      Lbl(end_label)
    ]);
    Ok(Json::Bool(Var(return_value)))
  }},
  grater_eq => {">=", COMMON, AtLeast(2), {
    self.take_int(Rax, func, scope)?;
    let false_label = self.gen_id();
    for idx in 1..func.len {
      self.take_int(if idx % 2 == 0 { Rax } else { Rcx }, func, scope)?;
      scope.extend(&[
        CmpRR(Rax, Rcx),
        Jcc(if idx % 2 == 0 { G } else { Le }, false_label),
      ]);
    }
    let end_label = self.gen_id();
    let return_value = scope.tmp(1, 1)?;
    scope.extend(&[
      MovMbIb(return_value.kind, 0xFF),
      JmpSh(end_label),
      Lbl(false_label),
      MovMbIb(return_value.kind, 0),
      Lbl(end_label)
    ]);
    Ok(Json::Bool(Var(return_value)))
  }},
  less => {"<", COMMON, AtLeast(2), {
    self.take_int(Rax, func, scope)?;
    let false_label = self.gen_id();
    for idx in 1..func.len {
      self.take_int(if idx % 2 == 0 { Rax } else { Rcx }, func, scope)?;
      scope.extend(&[
        CmpRR(Rax, Rcx),
        Jcc(if idx % 2 == 0 { L } else { Ge }, false_label),
      ]);
    }
    let end_label = self.gen_id();
    let return_value = scope.tmp(1, 1)?;
    scope.extend(&[
      MovMbIb(return_value.kind, 0xFF),
      JmpSh(end_label),
      Lbl(false_label),
      MovMbIb(return_value.kind, 0),
      Lbl(end_label)
    ]);
    Ok(Json::Bool(Var(return_value)))
  }},
  less_eq => {"<=", COMMON, AtLeast(2), {
    self.take_int(Rax, func, scope)?;
    let false_label = self.gen_id();
    for idx in 1..func.len {
      self.take_int(if idx % 2 == 0 { Rax } else { Rcx }, func, scope)?;
      scope.extend(&[
        CmpRR(Rax, Rcx),
        Jcc(if idx % 2 == 0 { Le } else { G }, false_label),
      ]);
    }
    let end_label = self.gen_id();
    let return_value = scope.tmp(1, 1)?;
    scope.extend(&[
      MovMbIb(return_value.kind, 0xFF),
      JmpSh(end_label),
      Lbl(false_label),
      MovMbIb(return_value.kind, 0),
      Lbl(end_label)
    ]);
    Ok(Json::Bool(Var(return_value)))
  }},
  not_eq => {"!=", COMMON, AtLeast(2), {
    self.take_int(Rax, func, scope)?;
    let false_label = self.gen_id();
    for idx in 1..func.len {
      self.take_int(if idx % 2 == 0 { Rax } else { Rcx }, func, scope)?;
      scope.extend(&[CmpRR(Rax, Rcx), Jcc(E, false_label)]);
    }
    let end_label = self.gen_id();
    let return_value = scope.tmp(1, 1)?;
    scope.extend(&[
      MovMbIb(return_value.kind, 0xFF),
      JmpSh(end_label),
      Lbl(false_label),
      MovMbIb(return_value.kind, 0),
      Lbl(end_label)
    ]);
    Ok(Json::Bool(Var(return_value)))
  }},
}
