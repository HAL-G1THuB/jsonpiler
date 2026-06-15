use crate::prelude::*;
built_in! {self, func, scope, logic;
  {"and", COMMON, AtLeast(2),
    and => { logic_op(And, func, scope) },
    and_a => { logic_a(And, func, scope) }
  },
  {"assert", COMMON, Exact(2),
    assert_x => { self.assert(func, scope) },
    assert_a => { self.assert(func, scope) }
  },
  {"not", COMMON, Exact(1),
    not => {
      match func.arg()? {
        Pos { val: Bool(boolean), .. } => {
          scope.ee_x(vec![
            load_bool_x(Rax, boolean)?,
            vec![Unary(S1, Not, Rax)]
          ])?;
          Ok(Bool(Var(scope.ret_x(S1, Rax)?)))
        }
        Pos { val: Int(int), ..} => {
          scope.ee_x(vec![
            load_int_x(Rax, int)?,
            vec![Unary(S8, Not, Rax)]
          ])?;
          Ok(Int(Var(scope.ret_x(S8, Rax)?)))
        }
        other => Err(func.args_err(vec![IntT, BoolT], &other))
      }
    },
    not_a => {
      match func.arg()? {
        Pos { val: Bool(boolean), .. } => {
          scope.ee_a(vec![
            load_bool_a(X0, boolean)?,
            vec![OrnR3(X0, Xzr, X0)]
          ])?;
          Ok(Bool(Var(scope.ret_a(S1, X1, X0)?)))
        }
        Pos { val: Int(int), ..} => {
          scope.ee_a(vec![
            load_int_a(X0, int)?,
            vec![OrnR3(X0, Xzr, X0)]
          ])?;
          Ok(Int(Var(scope.ret_a(S8, X1, X0)?)))
        }
        other => Err(func.args_err(vec![IntT, BoolT], &other))
      }
    }
  },
  {"or", COMMON, AtLeast(2),
    or => { logic_op(Or, func, scope) },
    or_a => { logic_a(Or, func, scope) }
  },
  {"xor", COMMON, AtLeast(2),
    xor => { logic_op(Xor, func, scope) },
    xor_a => { logic_a(Xor, func, scope) }
  },
}
impl Jsonpiler {
  fn assert(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    let boolean = arg!(func, (Bool(x)) => x);
    let string = arg!(func, (Str(x)) => x);
    if self.flags.a64 {
      scope.ee_a(vec![load_bool_a(X0, boolean.val)?, vec![TstRb(X0)]])?;
    } else {
      scope.ee_x(vec![load_bool_x(Rax, boolean.val)?, vec![TestRR(S1, Rax)]])?;
    }
    let assertion_err = self.runtime_err(AssertionErr, Some(string.val), boolean.pos, scope.id)?;
    scope.push_jcc(E, assertion_err);
    Ok(Null(Lit(())))
  }
}
fn logic_op(lo: Group1, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
  match func.arg()? {
    Pos { val: Bool(boolean), .. } => {
      scope.e_x(load_bool_x(Rax, boolean)?)?;
      for _ in 1..func.val.len {
        scope.ee_x(vec![
          load_bool_x(Rcx, arg!(func, (Bool(x)) => x).val)?,
          vec![RR(S1, lo, Rax, Rcx)],
        ])?;
      }
      Ok(Bool(Var(scope.ret_x(S1, Rax)?)))
    }
    Pos { val: Int(int), .. } => {
      scope.e_x(load_int_x(Rax, int)?)?;
      for _ in 1..func.val.len {
        scope.ee_x(vec![
          load_int_x(Rcx, arg!(func, (Int(x)) => x).val)?,
          vec![RR(S8, lo, Rax, Rcx)],
        ])?;
      }
      Ok(Int(Var(scope.ret_x(S8, Rax)?)))
    }
    other => Err(func.args_err(vec![IntT, BoolT], &other)),
  }
}
fn logic_a(lo: Group1, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
  let op = match lo {
    And => AndR3(X0, X0, X1),
    Or => OrrR3(X0, X0, X1),
    Xor => EorR3(X0, X0, X1),
    Add | Sub | Cmp => return Err(InvalidInst("invalid Logic".into()).into()),
  };
  match func.arg()? {
    Pos { val: Bool(boolean), .. } => {
      scope.e_a(load_bool_a(X0, boolean)?)?;
      for _ in 1..func.val.len {
        scope.ee_a(vec![load_bool_a(X1, arg!(func, (Bool(x)) => x).val)?, vec![op]])?;
      }
      Ok(Bool(Var(scope.ret_a(S1, X1, X0)?)))
    }
    Pos { val: Int(int), .. } => {
      scope.e_a(load_int_a(X0, int)?)?;
      for _ in 1..func.val.len {
        scope.ee_a(vec![load_int_a(X1, arg!(func, (Int(x)) => x).val)?, vec![op]])?;
      }
      Ok(Int(Var(scope.ret_a(S8, X1, X0)?)))
    }
    other => Err(func.args_err(vec![IntT, BoolT], &other)),
  }
}
