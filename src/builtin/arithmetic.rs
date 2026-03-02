use crate::prelude::*;
built_in! {self, _func, scope, arithmetic;
  abs => {"abs", COMMON, Exactly(1), {
    match _func.arg()? {
      WithPos { val: Int(int), .. } => {
        scope.extend(&mov_int(Rax, int));
        scope.extend(&[Custom(CQO), LogicRR(Xor, Rax, Rdx), SubRR(Rax, Rdx)]);
        Ok(Int(Var(scope.ret(Rax)?)))
      }
      WithPos { val: Float(float), .. } => {
        scope.extend(&mov_float_reg(Rax, float));
        scope.push(Custom(BTR_RAX_63));
        Ok(Float(Var(scope.ret(Rax)?)))
      }
      other => Err(args_type_err(1, &_func.name, "Int` or `Float".into(), &other))
    }
  }},
  calc_add => {"+", COMMON, AtLeast(2), {
    self.calc_normal(
      &(AddRR(Rax, Rcx), AddSd(Rax, Rcx)),
      (&i64::wrapping_add, &i64::wrapping_add),
      _func, scope, 0, (&|| Ok(None), &|| Some(IncR(Rax))), None
    )
  }},
  calc_div => {"/", COMMON, AtLeast(2), {
    let func_pos = _func.pos;
    self.calc_normal(
      &(IDivR(Rcx), DivSd(Rax, Rcx)),
      (&i64::wrapping_div, &i64::wrapping_mul),
      _func, scope, 1, (&|| err!(func_pos, ZeroDivision), &|| None),
      Some(&Jsonpiler::check_zero_cqo)
    )
  }},
  calc_minus => {"-", COMMON, AtLeast(1), {
    if _func.len == 1 {
      match _func.arg()? {
        WithPos { val: Int(int), .. } => {
          scope.extend(&mov_int(Rax, int));
          scope.push(NegR(Rax));
          Ok(Int(Var(scope.ret(Rax)?)))
        }
        WithPos { val: Float(float), .. } => {
          scope.extend(&mov_float_reg(Rax, float));
          scope.push(Custom(BTC_RAX_63));
          Ok(Float(Var(scope.ret(Rax)?)))
        }
        other => Err(args_type_err(1, &_func.name, "Int` or `Float".into(), &other))
      }
    } else {
      self.calc_normal(
        &(SubRR(Rax, Rcx), SubSd(Rax, Rcx)),
        (&i64::wrapping_sub, &i64::wrapping_add),
        _func, scope, 0, (&|| Ok(None), &|| Some(DecR(Rax))), None
      )
    }
  }},
  calc_mul => {"*", COMMON, AtLeast(2), {
    self.calc_normal(
      &(IMulRR(Rax, Rcx), MulSd(Rax, Rcx)),
      (&i64::wrapping_mul, &i64::wrapping_mul),
      _func, scope, 1, (&|| Ok(Some(0)), &|| None), None
    )
  }},
  float => {"Float", COMMON, Exactly(1), {
    scope.extend(&mov_int(Rax, arg!(self, _func, (Int(x)) => x).val));
    scope.push(CvtSi2Sd(Rax, Rax));
    scope.ret_xmm(Rax)
  }},
  int => {"Int", COMMON, Exactly(1), {
          scope.extend(&self.mov_float_xmm(Rax, Rax, arg!(self, _func, (Float(x)) => x).val));
    scope.push(CvtTSd2Si(Rax, Rax));
    Ok(Int(Var(scope.ret(Rax)?)))
  }},
  random => {"random", COMMON, Zero, {
    scope.push(Call(self.get_random()?));
    Ok(Int(Var(scope.ret(Rax)?)))
  }},
  rem => {"%", COMMON, Exactly(2), {
    let lhs = arg!(self, _func, (Int(x)) => x).val;
    let WithPos { val: rhs, pos } = arg!(self, _func, (Int(x)) => x);
    if matches!(rhs, Lit(0)) {
      return err!(pos, ZeroDivision);
    }
    if let (Lit(lit1), Lit(lit2)) = (&lhs, &rhs) {
      return Ok(Int(Lit(lit1.wrapping_rem(*lit2))));
    }
    extend!(
      scope.body,
      mov_int(Rax, lhs),
      mov_int(Rcx, rhs),
      self.check_zero_cqo(pos)?,
      [IDivR(Rcx)]
    );
    Ok(Int(Var(scope.ret(Rdx)?)))
  }},
  sqrt => {"sqrt", COMMON, Exactly(1), {
    scope.extend(&self.mov_float_xmm(Rax, Rax, arg!(self, _func, (Float(x)) => x).val));
    scope.push(SqrtSd(Rax, Rax));
    scope.ret_xmm(Rax)
  }},
}
type Op = dyn Fn(i64, i64) -> i64;
type CheckFn = dyn Fn(&mut Jsonpiler, Position) -> ErrOR<Vec<Inst>>;
impl Jsonpiler {
  #[expect(clippy::too_many_arguments)]
  fn calc_normal(
    &mut self,
    op_inst: &(Inst, Inst),
    ops: (&Op, &Op),
    func: &mut Function,
    scope: &mut Scope,
    ident_elem: i64,
    when: (&impl Fn() -> ErrOR<Option<i64>>, &impl Fn() -> Option<Inst>),
    check_opt: Option<&CheckFn>,
  ) -> ErrOR<Json> {
    match func.arg()? {
      WithPos { val: Int(int), .. } => {
        let mut rest = vec![];
        for _ in 1..func.len {
          rest.push(arg!(self, func, (Int(x)) => x));
        }
        let (first, vars, acc) = constant_fold(int, rest, ops, ident_elem, &when.0)?;
        if first.is_none() && vars.is_empty() {
          return Ok(Int(Lit(acc)));
        }
        if let Some(label) = first {
          if acc == 0
            && let Some(ret_val) = when.0()?
          {
            return Ok(Int(Lit(ret_val)));
          }
          scope.extend(&mov_int(Rax, Var(label)));
          if acc != 0 {
            if acc == 1
              && let Some(inst) = when.1()
            {
              scope.push(inst);
            } else {
              scope.extend(&mov_int(Rcx, Lit(acc)));
              if let Some(check) = check_opt {
                scope.extend(&check(self, func.pos)?);
              }
              scope.push(op_inst.0);
            }
          }
        } else {
          scope.extend(&mov_int(Rax, Lit(acc)));
        }
        for label_wp in vars {
          scope.extend(&mov_int(Rcx, Var(label_wp.val)));
          if let Some(check) = check_opt {
            scope.extend(&check(self, label_wp.pos)?);
          }
          scope.push(op_inst.0);
        }
        Ok(Int(Var(scope.ret(Rax)?)))
      }
      WithPos { val: Float(float), .. } => {
        scope.extend(&self.mov_float_xmm(Rax, Rax, float));
        for _ in 1..func.len {
          scope.extend(&self.mov_float_xmm(Rcx, Rax, arg!(self, func, (Float(x)) => x).val));
          scope.push(op_inst.1);
        }
        scope.ret_xmm(Rax)
      }
      other => Err(args_type_err(1, &func.name, "Int` or `Float".into(), &other)),
    }
  }
  pub(crate) fn check_zero_cqo(&mut self, pos: Position) -> ErrOR<Vec<Inst>> {
    let zero_division = self.custom_err(ZERO_DIVISION, Lit(String::new()), pos)?;
    Ok(vec![LogicRR(Test, Rcx, Rcx), JCc(E, zero_division), Custom(CQO)])
  }
}
fn constant_fold(
  first: Bind<i64>,
  rest: Vec<WithPos<Bind<i64>>>,
  ops: (&Op, &Op),
  ident_elem: i64,
  when0: &impl Fn() -> ErrOR<Option<i64>>,
) -> ErrOR<(Option<Label>, Vec<WithPos<Label>>, i64)> {
  let mut vars = vec![];
  let mut acc = ident_elem;
  for bind_wp in rest {
    match bind_wp.val {
      Lit(lit) => acc = ops.1(acc, lit),
      Var(label) => vars.push(bind_wp.pos.with(label)),
    }
  }
  match first {
    Lit(lit) => {
      if acc == 0
        && let Some(ret_val) = when0()?
      {
        return Ok((None, vars, ret_val));
      }
      Ok((None, vars, ops.0(lit, acc)))
    }
    Var(label) => Ok((Some(label), vars, acc)),
  }
}
