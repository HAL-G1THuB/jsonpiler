use crate::prelude::*;
built_in! {self, _func, scope, arithmetic;
  abs => {"abs", COMMON, Exact(1), {
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
      other => Err(args_type_err(1, &_func.name, vec![IntT, BoolT], other.map_ref(Json::as_type)))
    }
  }},
  calc_add => {"+", COMMON, AtLeast(2), {
    self.arithmetic_op(
      &(AddRR(Rax, Rcx), Add),
      (&i64::checked_add, &i64::checked_add),
      _func, scope, 0, (&|| Ok(None), &|| Some(IncR(Rax))), None
    )
  }},
  calc_div => {"/", COMMON, AtLeast(2), {
    let func_pos = _func.pos;
    self.arithmetic_op(
      &(IDivR(Rcx), Div),
      (&i64::checked_div, &i64::checked_mul),
      _func, scope, 1, (&|| err!(func_pos, ZeroDivision), &|| None),
      Some(&Jsonpiler::check_zero_cqo)
    )
  }},
  calc_minus => {"-", COMMON, AtLeast(1), {
    if _func.len == 1 {
      match _func.arg()? {
        WithPos { val: Int(int), .. } => {
          scope.extend(&mov_int(Rax, int));
          scope.push(UnaryR(Neg, Rax));
          Ok(Int(Var(scope.ret(Rax)?)))
        }
        WithPos { val: Float(float), .. } => {
          scope.extend(&mov_float_reg(Rax, float));
          scope.push(Custom(BTC_RAX_63));
          Ok(Float(Var(scope.ret(Rax)?)))
        }
        other => Err(args_type_err(1, &_func.name, vec![IntT, BoolT], other.map_ref(Json::as_type)))
      }
    } else {
      self.arithmetic_op(
        &(SubRR(Rax, Rcx), Sub),
        (&i64::checked_sub, &i64::checked_add),
        _func, scope, 0, (&|| Ok(None), &|| Some(DecR(Rax))), None
      )
    }
  }},
  calc_mul => {"*", COMMON, AtLeast(2), {
    self.arithmetic_op(
      &(IMulRR(Rax, Rcx), Mul),
      (&i64::checked_mul, &i64::checked_mul),
      _func, scope, 1, (&|| Ok(Some(0)), &|| None), None
    )
  }},
  float => {"Float", COMMON, Exact(1), {
    scope.extend(&mov_int(Rax, arg!(self, _func, (Int(x)) => x).val));
    scope.push(CvtSi2Sd(Rax, Rax));
    scope.ret_xmm(Rax)
  }},
  int => {"Int", COMMON, Exact(1), {
    scope.extend(&self.mov_float_xmm(Rax, Rax, arg!(self, _func, (Float(x)) => x).val));
    scope.push(CvtTSd2Si(Rax, Rax));
    Ok(Int(Var(scope.ret(Rax)?)))
  }},
  random => {"random", COMMON, Exact(0), {
    scope.push(Call(self.get_random(scope.id)?));
    Ok(Int(Var(scope.ret(Rax)?)))
  }},
  rem => {"%", COMMON, Exact(2), {
    let lhs = arg!(self, _func, (Int(x)) => x).val;
    let WithPos { val: rhs, pos } = arg!(self, _func, (Int(x)) => x);
    if matches!(rhs, Lit(0)) {
      return err!(pos, ZeroDivision);
    }
    if let (Lit(lit1), Lit(lit2)) = (&lhs, &rhs) {
      return Ok(Int(Lit(lit1.wrapping_rem(*lit2))));
    }
    scope.extend(&mov_int(Rax, lhs));
    scope.extend(&mov_int(Rcx, rhs));
    scope.extend(&self.check_zero_cqo(pos, scope.id)?);
    scope.push(IDivR(Rcx));
    Ok(Int(Var(scope.ret(Rdx)?)))
  }},
  shift_left => {"<<", COMMON, Exact(2), { self.shift(Shl, _func, scope) }},
  shift_right => {">>", COMMON, Exact(2), { self.shift(Shr, _func, scope) }},
  sqrt => {"sqrt", COMMON, Exact(1), {
    scope.extend(&self.mov_float_xmm(Rax, Rax, arg!(self, _func, (Float(x)) => x).val));
    scope.push(SqrtSd(Rax, Rax));
    scope.ret_xmm(Rax)
  }},
}
type Op = dyn Fn(i64, i64) -> Option<i64>;
type CheckFn = dyn Fn(&mut Jsonpiler, Position, LabelId) -> ErrOR<Vec<Inst>>;
impl Jsonpiler {
  #[expect(clippy::too_many_arguments)]
  fn arithmetic_op(
    &mut self,
    op_inst: &(Inst, ArithSdKind),
    ops: (&Op, &Op),
    func: &mut BuiltIn,
    scope: &mut Scope,
    ident_elem: i64,
    when: (&impl Fn() -> ErrOR<Option<i64>>, &impl Fn() -> Option<Inst>),
    check_opt: Option<&CheckFn>,
  ) -> ErrOR<Json> {
    match func.arg()? {
      WithPos { val: Int(int), pos } => {
        let mut rest = vec![];
        for _ in 1..func.len {
          rest.push(arg!(self, func, (Int(x)) => x));
        }
        let (first, vars, acc) = constant_fold(pos.with(int), rest, ops, ident_elem, &when.0)?;
        if first.is_none() && vars.is_empty() {
          return Ok(Int(Lit(acc)));
        }
        if let Some(memory) = first {
          if acc == 0
            && let Some(ret_val) = when.0()?
          {
            return Ok(Int(Lit(ret_val)));
          }
          scope.extend(&mov_int(Rax, Var(memory)));
          if acc != 0 {
            if acc == 1 {
              if let Some(inst) = when.1() {
                scope.push(inst);
                if !self.release {
                  scope.push(JCc(O, self.custom_err(RuntimeOverflow, None, func.pos, scope.id)?));
                }
              }
            } else {
              self.rest_op(Lit(acc), check_opt, op_inst.0, func, scope)?;
            }
          }
        } else {
          scope.extend(&mov_int(Rax, Lit(acc)));
        }
        for memory in vars {
          self.rest_op(Var(memory.val), check_opt, op_inst.0, func, scope)?;
        }
        Ok(Int(Var(scope.ret(Rax)?)))
      }
      WithPos { val: Float(float), .. } => {
        scope.extend(&self.mov_float_xmm(Rax, Rax, float));
        for _ in 1..func.len {
          scope.extend(&self.mov_float_xmm(Rcx, Rax, arg!(self, func, (Float(x)) => x).val));
          scope.push(ArithSd(op_inst.1, Rax, Rcx));
        }
        scope.ret_xmm(Rax)
      }
      other => Err(args_type_err(1, &func.name, vec![IntT, BoolT], other.map_ref(Json::as_type))),
    }
  }
  pub(crate) fn check_zero_cqo(&mut self, pos: Position, caller: LabelId) -> ErrOR<Vec<Inst>> {
    let zero_division = self.custom_err(RuntimeZeroDivision, None, pos, caller)?;
    Ok(vec![LogicRR(Test, Rcx, Rcx), JCc(E, zero_division), Custom(CQO)])
  }
  fn rest_op(
    &mut self,
    bind: Bind<i64>,
    check_opt: Option<&CheckFn>,
    int_inst: Inst,
    func: &mut BuiltIn,
    scope: &mut Scope,
  ) -> ErrOR<()> {
    scope.extend(&mov_int(Rcx, bind));
    if let Some(check) = check_opt {
      scope.extend(&check(self, func.pos, scope.id)?);
    }
    scope.push(int_inst);
    if !self.release {
      scope.push(JCc(O, self.custom_err(RuntimeOverflow, None, func.pos, scope.id)?));
    }
    Ok(())
  }
  fn shift(
    &mut self,
    direction: ShiftDirection,
    func: &mut BuiltIn,
    scope: &mut Scope,
  ) -> ErrOR<Json> {
    let lhs = arg!(self, func, (Int(x)) => x).val;
    let WithPos { val: rhs, pos } = arg!(self, func, (Int(x)) => x);
    if let (Lit(lit1), Lit(lit2)) = (&lhs, &rhs) {
      let Ok(rhs_u32) = u32::try_from(*lit2) else { return err!(pos, TooLargeShift) };
      return Ok(Int(Lit(
        if direction == Shl { lit1.checked_shl(rhs_u32) } else { lit1.checked_shr(rhs_u32) }
          .ok_or(Compilation(TooLargeShift, vec![pos]))?,
      )));
    }
    scope.extend(&mov_int(Rax, lhs));
    if let Lit(lit2) = rhs {
      if let Ok(rhs_u8) = u8::try_from(lit2)
        && lit2 < 64
      {
        scope.push(ShiftR(direction, Rax, Shift::Ib(rhs_u8)));
      } else {
        return err!(pos, TooLargeShift);
      }
    } else {
      let too_large_shift = self.custom_err(RuntimeTooLargeShift, None, pos, scope.id)?;
      scope.extend(&mov_int(Rcx, rhs));
      scope.extend(&[mov_d(Rdx, 64), LogicRR(Cmp, Rcx, Rdx), JCc(Ge, too_large_shift)]);
      scope.push(ShiftR(direction, Rax, Shift::Cl));
    }
    Ok(Int(Var(scope.ret(Rax)?)))
  }
}
fn constant_fold(
  first: WithPos<Bind<i64>>,
  rest: Vec<WithPos<Bind<i64>>>,
  ops: (&Op, &Op),
  ident_elem: i64,
  when0: &impl Fn() -> ErrOR<Option<i64>>,
) -> ErrOR<(Option<Memory>, Vec<WithPos<Memory>>, i64)> {
  let mut vars = vec![];
  let mut acc = ident_elem;
  for bind in rest {
    match bind.val {
      Lit(lit) => acc = ops.1(acc, lit).ok_or(Compilation(Overflow, vec![bind.pos]))?,
      Var(memory) => vars.push(bind.pos.with(memory)),
    }
  }
  match first.val {
    Lit(lit) => Ok((
      None,
      vars,
      if acc == 0
        && let Some(ret_val) = when0()?
      {
        ret_val
      } else {
        ops.0(lit, acc).ok_or(Compilation(Overflow, vec![first.pos]))?
      },
    )),
    Var(memory) => Ok((Some(memory), vars, acc)),
  }
}
