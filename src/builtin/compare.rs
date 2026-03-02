use crate::prelude::*;
built_in! {self, func, scope, compare;
  eq => {"==", COMMON, AtLeast(2), {cmp(E, func, scope)}},
  grater => {">", COMMON, AtLeast(2), {cmp(G, func, scope)}},
  grater_eq => {">=", COMMON, AtLeast(2), {cmp(Ge, func, scope)}},
  less => {"<", COMMON, AtLeast(2), {cmp(L, func, scope)}},
  less_eq => {"<=", COMMON, AtLeast(2), {cmp(Le, func, scope)}},
  not_eq => {"!=", COMMON, AtLeast(2), {cmp(Ne, func, scope)}},
}
fn cmp(cc: ConditionCode, func: &mut Function, scope: &mut Scope) -> ErrOR<Json> {
  scope.extend(&mov_int(Rax, arg!(self, func, (Int(x)) => x).val));
  scope.push(mov_b(Rdx, 1));
  for nth in 1..func.len {
    let (old, new) = if nth % 2 == 1 { (Rax, Rcx) } else { (Rcx, Rax) };
    scope.extend(&mov_int(new, arg!(self, func, (Int(x)) => x).val));
    scope.extend(&[LogicRR(Cmp, old, new), SetCc(cc, old), LogicRbRb(And, Rdx, old)]);
  }
  scope.push(NegRb(Rdx));
  scope.ret_bool(Rdx)
}
