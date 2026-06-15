use crate::prelude::*;
built_in! {self, func, scope, compare;
  {"==", COMMON, AtLeast(2),
    eq => { self.compare_x(E, E, func, scope) },
    eq_a => { self.compare_a(E, None, func, scope) }
  },
  {">", COMMON, AtLeast(2),
    greater => { self.compare_x(G, A, func, scope) },
    greater_a => { self.compare_a(G, None, func, scope) }
  },
  {">=", COMMON, AtLeast(2),
    greater_eq => { self.compare_x(Ge, Ae, func, scope) },
    greater_eq_a => { self.compare_a(Ge, None, func, scope) }
  },
  {"<", COMMON, AtLeast(2),
    less => { self.compare_x(L, B, func, scope) },
    less_a => { self.compare_a(L, Some(S.into()), func, scope) }
  },
  {"<=", COMMON, AtLeast(2),
    less_eq => { self.compare_x(Le, Be, func, scope) },
    less_eq_a => { self.compare_a(Le, Some(Be.into()), func, scope) }
  },
  {"!=", COMMON, AtLeast(2),
    not_eq => { self.compare_x(Ne, Ne, func, scope) },
    not_eq_a => { self.compare_a(Ne, None, func, scope) }
  },
}
impl Jsonpiler {
  fn compare_a(
    &mut self,
    cc: X64Cc,
    f_cc: Option<A64Cc>,
    func: &mut Pos<BuiltIn>,
    scope: &mut Scope,
  ) -> ErrOR<Json> {
    match func.arg()? {
      Pos { val: Int(int), .. } => {
        scope.p_a(MovZ(X0, 1, Lsl0))?;
        scope.e_a(load_int_a(X1, int)?)?;
        for nth in 1..func.val.len {
          let (old, new) = if nth & 1 == 1 { (X1, X2) } else { (X2, X1) };
          scope.e_a(load_int_a(new, arg!(func, (Int(x)) => x).val)?)?;
          scope.e_a(vec![CmpRR(old, new), CSet(old, cc.into()), AndR3(X0, X0, old)])?;
        }
        scope.p_a(SubR3(X0, Xzr, X0))?;
      }
      Pos { val: Float(float), .. } => {
        scope.p_a(MovZ(X0, 1, Lsl0))?;
        scope.e_a(self.load_dn(X0, X3, float)?)?;
        for nth in 1..func.val.len {
          let (old, new) = if nth & 1 == 1 { (X0, X1) } else { (X1, X0) };
          scope.ee_a(vec![
            self.load_dn(new, X3, arg!(func, (Float(x)) => x).val)?,
            vec![FCmpD(old, new), CSet(X2, f_cc.unwrap_or(cc.into())), AndR3(X0, X0, X2)],
          ])?;
        }
        scope.p_a(SubR3(X0, Xzr, X0))?;
      }
      Pos { val: Str(_string), .. } if matches!(func.val.name.as_ref(), "==" | "!=") => {
        func.validate_args(Exact(2))?;
        let str_eq = self.str_eq_a(scope.id)?;
        scope.ee_a(vec![
          self.load_str_a(X0, &_string)?,
          self.load_str_a(X1, &arg!(func, (Str(x)) => x).val)?,
          vec![Bl(str_eq)],
        ])?;
        if func.val.name == "!=" {
          scope.p_a(OrnR3(X0, Xzr, X0))?;
        }
      }
      other => {
        let mut expected = vec![IntT, FloatT];
        if matches!(func.val.name.as_ref(), "==" | "!=") {
          expected.push(StrT);
        }
        return Err(func.args_err(expected, &other));
      }
    }
    Ok(Bool(Var(scope.ret_a(S1, X1, X0)?)))
  }
  fn compare_x(
    &mut self,
    cc: X64Cc,
    f_cc: X64Cc,
    func: &mut Pos<BuiltIn>,
    scope: &mut Scope,
  ) -> ErrOR<Json> {
    match func.arg()? {
      Pos { val: Int(int), .. } => {
        scope.p_x(MovMIb(Reg(Rdx), 1))?;
        scope.e_x(load_int_x(Rax, int)?)?;
        for nth in 1..func.val.len {
          let (old, new) = if nth & 1 == 1 { (Rax, Rcx) } else { (Rcx, Rax) };
          scope.ee_x(vec![
            load_int_x(new, arg!(func, (Int(x)) => x).val)?,
            vec![RR(S8, Cmp, old, new), SetCc(old, cc), RR(S1, And, Rdx, old)],
          ])?;
        }
        scope.p_x(Unary(S1, Neg, Rdx))?;
        Ok(Bool(Var(scope.ret_x(S1, Rdx)?)))
      }
      Pos { val: Float(float), .. } => {
        scope.p_x(MovMIb(Reg(Rdx), 1))?;
        scope.e_x(self.load_xmm(Rax, Rax, float)?)?;
        for nth in 1..func.val.len {
          let (old, new) = if nth & 1 == 1 { (Rax, Rcx) } else { (Rcx, Rax) };
          scope.ee_x(vec![
            self.load_xmm(new, Rax, arg!(func, (Float(x)) => x).val)?,
            vec![UComISd(old, new), SetCc(Rax, f_cc), RR(S1, And, Rdx, Rax)],
          ])?;
        }
        scope.p_x(Unary(S1, Neg, Rdx))?;
        Ok(Bool(Var(scope.ret_x(S1, Rdx)?)))
      }
      Pos { val: Str(string), .. } if matches!(func.val.name.as_ref(), "==" | "!=") => {
        func.validate_args(Exact(2))?;
        let str_eq = self.str_eq_x(scope.id)?;
        scope.ee_x(vec![
          self.load_str_x(Rcx, &string)?,
          self.load_str_x(Rdx, &arg!(func, (Str(x)) => x).val)?,
          vec![Call(str_eq)],
        ])?;
        if func.val.name == "!=" {
          scope.p_x(Unary(S1, Not, Rax))?;
        }
        Ok(Bool(Var(scope.ret_x(S1, Rax)?)))
      }
      other => {
        let mut expected = vec![IntT, FloatT];
        if matches!(func.val.name.as_ref(), "==" | "!=") {
          expected.push(StrT);
        }
        Err(func.args_err(expected, &other))
      }
    }
  }
}
