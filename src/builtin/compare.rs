use crate::{
  Arity::AtLeast,
  ConditionCode::{self, *},
  ErrOR, FuncInfo,
  Inst::*,
  Json, Jsonpiler,
  LogicByteOpcode::*,
  Register::*,
  ScopeInfo, built_in,
  utility::{mov_b, take_int},
};
built_in! {self, func, scope, compare;
  eq => {"==", COMMON, AtLeast(2), {compare_template(E, func, scope)}},
  grater => {">", COMMON, AtLeast(2), {compare_template(G, func, scope)}},
  grater_eq => {">=", COMMON, AtLeast(2), {compare_template(Ge, func, scope)}},
  less => {"<", COMMON, AtLeast(2), {compare_template(L, func, scope)}},
  less_eq => {"<=", COMMON, AtLeast(2), {compare_template(Le, func, scope)}},
  not_eq => {"!=", COMMON, AtLeast(2), {compare_template(Ne, func, scope)}},
}
fn compare_template(cc: ConditionCode, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
  take_int(Rax, func, scope)?;
  scope.extend(&[mov_b(Rdx, 0xFF)]);
  for idx in 1..func.len {
    let old_reg = if idx % 2 == 0 { Rcx } else { Rax };
    let new_reg = if idx % 2 == 0 { Rax } else { Rcx };
    take_int(new_reg, func, scope)?;
    scope.extend(&[
      LogicRR(Cmp, old_reg, new_reg),
      SetCc(cc, old_reg),
      NegRb(old_reg),
      LogicRbRb(And, Rdx, old_reg),
    ]);
  }
  scope.mov_tmp_bool(Rdx)
}
