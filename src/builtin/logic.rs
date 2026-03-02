use crate::prelude::*;
built_in! {self, func, scope, logic;
  and => {"and", COMMON, AtLeast(2), { logic_normal(And, func, scope) }},
  assert => {"assert", COMMON, Exactly(2), {
    let boolean = arg!(self, func, (Bool(x)) => x);
    let string = arg!(self, func, (Str(x)) => x);
    let assertion_err = self.custom_err(ASSERTION_ERROR, string.val, boolean.pos)?;
    scope.extend(&mov_bool(Rax, boolean.val));
    scope.extend(&[LogicRbRb(Test, Rax, Rax), JCc(E, assertion_err)]);
    Ok(Null)
  }},
  not => {"not", COMMON, Exactly(1), {
    match func.arg()? {
      WithPos { val: Bool(boolean), .. } => {
        scope.extend(&mov_bool(Rax, boolean));
        scope.push(NotRb(Rax));
        scope.ret_bool(Rax)
      }
      WithPos { val: Int(int), ..} => {
        scope.extend(&mov_int(Rax, int));
        scope.push(NotR(Rax));
        Ok(Int(Var(scope.ret(Rax)?)))
      }
      other => Err(args_type_err(1, &func.name, "Int` or `Bool".into(), &other))
      }
  }},
  or => {"or", COMMON, AtLeast(2), { logic_normal(Or, func, scope) }},
  xor => {"xor", COMMON, AtLeast(2), { logic_normal(Xor, func, scope) }},
}
fn logic_normal(lo: LogicOpcode, func: &mut Function, scope: &mut Scope) -> ErrOR<Json> {
  match func.arg()? {
    WithPos { val: Bool(boolean), .. } => {
      scope.extend(&mov_bool(Rax, boolean));
      for _ in 1..func.len {
        scope.extend(&mov_bool(Rcx, arg!(self, func, (Bool(x)) => x).val));
        scope.push(LogicRbRb(lo, Rax, Rcx));
      }
      scope.ret_bool(Rax)
    }
    WithPos { val: Int(int), .. } => {
      scope.extend(&mov_int(Rax, int));
      for _ in 1..func.len {
        scope.extend(&mov_int(Rcx, arg!(self, func, (Int(x)) => x).val));
        scope.push(LogicRR(lo, Rax, Rcx));
      }
      Ok(Int(Var(scope.ret(Rax)?)))
    }
    other => Err(args_type_err(1, &func.name, "Int` or `Bool".into(), &other)),
  }
}
