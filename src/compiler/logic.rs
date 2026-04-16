use crate::prelude::*;
built_in! {self, func, scope, logic;
  and => {"and", COMMON, AtLeast(2), { logic_op(And, func, scope) }},
  assert => {"assert", COMMON, Exact(2), {
    let boolean = arg!(self, func, (Bool(x)) => x);
    let string = arg!(self, func, (Str(x)) => x);
    let assertion_err = self.custom_err(AssertionErr, Some(string.val), boolean.pos, scope.id)?;
    scope.extend(&mov_bool(Rax, boolean.val));
    scope.extend(&[LogicRbRb(Test, Rax, Rax), JCc(E, assertion_err)]);
    Ok(Null(Lit(())))
  }},
  not => {"not", COMMON, Exact(1), {
    match func.arg()? {
      Pos { val: Bool(boolean), .. } => {
        scope.extend(&mov_bool(Rax, boolean));
        scope.push(UnaryRb(Not, Rax));
        scope.ret_bool(Rax)
      }
      Pos { val: Int(int), ..} => {
        scope.extend(&mov_int(Rax, int));
        scope.push(UnaryR(Not, Rax));
        Ok(Int(Var(scope.ret(Rax)?)))
      }
      other => Err(func.args_err(vec![IntT, BoolT], other.map_ref(Json::as_type)))
    }
  }},
  or => {"or", COMMON, AtLeast(2), { logic_op(Or, func, scope) }},
  xor => {"xor", COMMON, AtLeast(2), { logic_op(Xor, func, scope) }},
}
fn logic_op(lo: Logic, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
  match func.arg()? {
    Pos { val: Bool(boolean), .. } => {
      scope.extend(&mov_bool(Rax, boolean));
      for _ in 1..func.val.len {
        scope.extend(&mov_bool(Rcx, arg!(self, func, (Bool(x)) => x).val));
        scope.push(LogicRbRb(lo, Rax, Rcx));
      }
      scope.ret_bool(Rax)
    }
    Pos { val: Int(int), .. } => {
      scope.extend(&mov_int(Rax, int));
      for _ in 1..func.val.len {
        scope.extend(&mov_int(Rcx, arg!(self, func, (Int(x)) => x).val));
        scope.push(LogicRR(lo, Rax, Rcx));
      }
      Ok(Int(Var(scope.ret(Rax)?)))
    }
    other => Err(func.args_err(vec![IntT, BoolT], other.map_ref(Json::as_type))),
  }
}
