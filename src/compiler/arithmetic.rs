use crate::prelude::*;
pub(crate) const CHECK_X: Option<CheckFn<X64Inst>> = Some(&Jsonpiler::nz_cqo);
pub(crate) const CHECK_A: Option<CheckFn<A64Inst>> = Some(&Jsonpiler::nz_a);
built_in! {self, _func, scope, arithmetic;
  {"abs", COMMON, Exact(1),
    abs => {
      match _func.arg()? {
        Pos { val: Int(int), .. } => {
          scope.ee_x(vec![
            load_int_x(Rax, int)?,
            vec![Cqo, RR(S8, Xor, Rax, Rdx), RR(S8, Sub, Rax, Rdx)]
          ])?;
          Ok(Int(Var(scope.ret_x(S8, Rax)?)))
        }
        Pos { val: Float(float), .. } => {
          scope.ee_x(vec![load_float_reg_x(Rax, float)?, vec![BitTest(Btr, Rax, 63)]])?;
          Ok(Float(Var(scope.ret_x(S8, Rax)?)))
        }
        other => Err(_func.args_err(vec![IntT, FloatT], &other))
      }
    },
  abs_a => {
      match _func.arg()? {
        Pos { val: Int(int), .. } => {
          scope.ee_a(vec![
            load_int_a(X0, int)?,
            vec![Asr(X1, X0, 63), EorR3(X0, X0, X1), SubR3(X0, X0, X1)]
          ])?;
          Ok(Int(Var(scope.ret_a(S8, X1, X0)?)))
        }
        Pos { val: Float(float), .. } => {
          scope.ee_a(vec![
            self.load_dn(X0, X0, float)?,
            vec![FAbsD(X0, X0)]
          ])?;
          scope.ret_dn(X0, X0)
        }
        other => Err(_func.args_err(vec![IntT, FloatT], &other))
      }
    }
  },
  {"+", COMMON, AtLeast(2),
    calc_add => { self.arith_add(_func, scope) },
    calc_add_a => { self.arith_add(_func, scope) }
  },
  {"/", COMMON, AtLeast(2),
    calc_div => { self.arith_div(_func, scope) },
    calc_div_a => { self.arith_div(_func, scope) }
  },
  {"-", COMMON, AtLeast(1),
    calc_minus => { self.minus(_func, scope) },
    calc_minus_a => { self.minus(_func, scope) }
  },
  {"*", COMMON, AtLeast(2),
    calc_mul => { self.arith_mul(_func, scope) },
    calc_mul_a => { self.arith_mul(_func, scope) }
  },
  {"Float", COMMON, Exact(1),
    float => {
      scope.ee_x(vec![
        load_int_x(Rax, arg!(_func, (Int(x)) => x).val)?,
        vec![CvtSi2Sd(Rax, Rax)]
      ])?;
      scope.ret_xmm(Rax)
    },
    float_a => {
      scope.ee_a(vec![
        load_int_a(X0, arg!(_func, (Int(x)) => x).val)?,
        vec![SCvtFD(X0, X0)]
      ])?;
      scope.ret_dn(X0, X0)
    }
  },
  {"Int", COMMON, Exact(1),
    int => {
      scope.ee_x(vec![
        self.load_xmm(Rax, Rax, arg!(_func, (Float(x)) => x).val)?,
        vec![CvtTSd2Si(Rax, Rax)]
      ])?;
      Ok(Int(Var(scope.ret_x(S8, Rax)?)))
    },
    int_a => {
      scope.ee_a(vec![
        self.load_dn(X0, X0, arg!(_func, (Float(x)) => x).val)?,
        vec![FCvtZSD(X0, X0)]
      ])?;
      Ok(Int(Var(scope.ret_a(S8, X1, X0)?)))
    }
  },
  {"random", COMMON, Exact(0),
    random => {
      scope.p_x(Call(self.get_random_x(scope.id)?))?;
      Ok(Int(Var(scope.ret_x(S8, Rax)?)))
    },
    random_a => {
      scope.p_a(Bl(self.get_random_a(scope.id)?))?;
      Ok(Int(Var(scope.ret_a(S8, X1, X0)?)))
    }
  },
  {"%", COMMON, Exact(2),
    rem => { self.reminder(_func, scope) },
    rem_a => { self.reminder(_func, scope) }
  },
  {"<<", COMMON, Exact(2),
    shift_left => { self.shift(Shl, _func, scope) },
    shift_left_a => { self.shift(Shl, _func, scope) }
  },
  {">>", COMMON, Exact(2),
    shift_right => { self.shift(Sar, _func, scope) },
    shift_right_a => { self.shift(Sar, _func, scope) }
  },
  {"sqrt", COMMON, Exact(1),
    sqrt => {
      scope.ee_x(vec![
        self.load_xmm(Rax, Rax, arg!(_func, (Float(x)) => x).val)?,
        vec![SqrtSd(Rax, Rax)]
      ])?;
      scope.ret_xmm(Rax)
    },
    sqrt_a => {
      scope.ee_a(vec![
        self.load_dn(X0, X0, arg!(_func, (Float(x)) => x).val)?,
        vec![FSqrtD(X0, X0)]
      ])?;
      scope.ret_dn(X0, X0)
    }
},
}
pub(crate) type Op = dyn Fn(i64, i64) -> Option<i64>;
pub(crate) type CheckFn<'a, T> = &'a dyn Fn(&mut Jsonpiler, Position, LabelId) -> ErrOR<Vec<T>>;
pub(crate) type Arith<'a> = (
  Result<Group1, (X64Inst, Option<CheckFn<'a, X64Inst>>)>,
  ArithSdKind,
  &'a dyn Fn(A64Reg, A64Reg, A64Reg) -> A64Inst,
  Option<CheckFn<'a, A64Inst>>,
);
impl Jsonpiler {
  fn arith_add(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    const CHECKED: (&Op, &Op) = (&i64::checked_add, &i64::checked_add);
    let when = (&|| Ok(None), Some(IncR(Rax)));
    if self.flags.debug {
      self.arith((Ok(Add), AddSd, &AddSR3, None), CHECKED, 0, when, func, scope)
    } else {
      self.arith((Ok(Add), AddSd, &AddR3, None), CHECKED, 0, when, func, scope)
    }
  }
  fn arith_div(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    const CHECKED: (&Op, &Op) = (&i64::checked_div, &i64::checked_mul);
    let func_pos = func.pos;
    let when = (&|| err!(func_pos, ZeroDivision), None);
    self.arith((Err((IDivR(Rcx), CHECK_X)), Div, &SDivR3, CHECK_A), CHECKED, 1, when, func, scope)
  }
  fn arith_mul(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    const CHECKED: (&Op, &Op) = (&i64::checked_mul, &i64::checked_mul);
    let when = (&|| Ok(Some(0)), None);
    self.arith((Err((IMulR2(Rax, Rcx), None)), Mul, &MulR3, None), CHECKED, 1, when, func, scope)
  }
  fn arith_sub(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    const CHECKED: (&Op, &Op) = (&i64::checked_sub, &i64::checked_add);
    let when = (&|| Ok(None), Some(DecR(Rax)));
    if self.flags.debug {
      self.arith((Ok(Sub), SubSd, &SubSR3, None), CHECKED, 0, when, func, scope)
    } else {
      self.arith((Ok(Sub), SubSd, &SubR3, None), CHECKED, 0, when, func, scope)
    }
  }
  fn minus(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    if func.val.len == 1 { self.neg(func, scope) } else { self.arith_sub(func, scope) }
  }
  fn neg(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    match func.arg()? {
      Pos { val: Int(int), .. } => {
        if self.flags.a64 {
          scope.e_a(load_int_a(X0, int)?)?;
          scope.p_a(SubR3(X0, Xzr, X0))?;
          Ok(Int(Var(scope.ret_a(S8, X1, X0)?)))
        } else {
          scope.e_x(load_int_x(Rax, int)?)?;
          scope.p_x(Unary(S8, Neg, Rax))?;
          Ok(Int(Var(scope.ret_x(S8, Rax)?)))
        }
      }
      Pos { val: Float(float), .. } => {
        if self.flags.a64 {
          scope.e_a(self.load_dn(X0, X0, float)?)?;
          scope.p_a(FNegD(X0, X0))?;
          scope.ret_dn(X0, X0)
        } else {
          scope.e_x(load_float_reg_x(Rax, float)?)?;
          scope.p_x(BitTest(Btc, Rax, 63))?;
          Ok(Float(Var(scope.ret_x(S8, Rax)?)))
        }
      }
      other => Err(func.args_err(vec![IntT, FloatT], &other)),
    }
  }
  fn reminder(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    let lhs = arg!(func, (Int(x)) => x).val;
    let Pos { val: rhs, pos } = arg!(func, (Int(x)) => x);
    if matches!(rhs, Lit(0)) {
      return err!(pos, ZeroDivision);
    }
    if let (Lit(lit1), Lit(lit2)) = (&lhs, &rhs) {
      return Ok(Int(Lit(lit1.wrapping_rem(*lit2))));
    }
    self.mov_first_operand(lhs, scope)?;
    if self.flags.a64 {
      scope.p_a(MovRR(X2, X0))?;
    }
    self.rest_op(rhs, (Err((IDivR(Rcx), CHECK_X)), Div, &SDivR3, CHECK_A), func, scope)?;
    if self.flags.a64 {
      scope.e_a(vec![MulR3(X0, X0, X1), SubR3(X0, X2, X0)])?;
      Ok(Int(Var(scope.ret_a(S8, X1, X0)?)))
    } else {
      Ok(Int(Var(scope.ret_x(S8, Rdx)?)))
    }
  }
}
impl Jsonpiler {
  fn arith(
    &mut self,
    (int_x, float_kind, int_a, check_a): Arith,
    ops: (&Op, &Op),
    ident_elem: i64,
    when: (&impl Fn() -> ErrOR<Option<i64>>, Option<X64Inst>),
    func: &mut Pos<BuiltIn>,
    scope: &mut Scope,
  ) -> ErrOR<Json> {
    match func.arg()? {
      Pos { val: Int(int), pos } => {
        let mut rest = vec![];
        for _ in 1..func.val.len {
          rest.push(arg!(func, (Int(x)) => x));
        }
        let (first, vars, acc) = constant_fold(pos.with(int), rest, ops, ident_elem, &when.0)?;
        match first {
          Lit(lit) if vars.is_empty() => return Ok(Int(Lit(lit))),
          Lit(lit) => self.mov_first_operand(Lit(lit), scope)?,
          Var(mem) => {
            self.mov_first_operand(Var(mem), scope)?;
            match acc {
              0 => (),
              1 if !self.flags.a64 => {
                if let Some(int_inst_x) = when.1 {
                  let arith = (Err((int_inst_x, None)), float_kind, int_a, None);
                  self.op_and_check_overflow(arith, func, scope)?;
                }
              }
              _ => self.rest_op(Lit(acc), (int_x, float_kind, int_a, check_a), func, scope)?,
            }
          }
        }
        for mem in vars {
          self.rest_op(Var(mem.val), (int_x, float_kind, int_a, check_a), func, scope)?;
        }
        let mem = if self.flags.a64 { scope.ret_a(S8, X1, X0)? } else { scope.ret_x(S8, Rax)? };
        Ok(Int(Var(mem)))
      }
      Pos { val: Float(float), .. } => {
        self.mov_first_float(float, scope)?;
        for _ in 1..func.val.len {
          self.rest_float(arg!(func, (Float(x)) => x).val, float_kind, scope)?;
        }
        if self.flags.a64 { scope.ret_dn(X0, X0) } else { scope.ret_xmm(Rax) }
      }
      Pos { val: Str(string), .. } if func.val.name == "+" => self.cat_str(string, func, scope),
      other => {
        let mut expected = vec![IntT, FloatT];
        if func.val.name == "+" {
          expected.push(StrT);
        }
        Err(func.args_err(expected, &other))
      }
    }
  }
  pub(crate) fn mov_first_float(&mut self, float: Bind<f64>, scope: &mut Scope) -> ErrOR<()> {
    if self.flags.a64 {
      scope.e_a(self.load_dn(X0, X0, float)?)
    } else {
      scope.e_x(self.load_xmm(Rax, Rax, float)?)
    }
  }
  pub(crate) fn mov_first_operand(&mut self, bind: Bind<i64>, scope: &mut Scope) -> ErrOR<()> {
    if self.flags.a64 {
      scope.e_a(load_int_a(X0, bind)?)
    } else {
      scope.e_x(load_int_x(Rax, bind)?)
    }
  }
  pub(crate) fn nz_a(&mut self, pos: Position, caller: LabelId) -> ErrOR<Vec<A64Inst>> {
    let zero_div = self.runtime_err(RuntimeZeroDivision, None, pos, caller)?;
    Ok(vec![CmpRR(X1, Xzr), BCc(E.into(), zero_div)])
  }
  pub(crate) fn nz_cqo(&mut self, pos: Position, caller: LabelId) -> ErrOR<Vec<X64Inst>> {
    let zero_div = self.runtime_err(RuntimeZeroDivision, None, pos, caller)?;
    Ok(vec![TestRR(S8, Rcx), JCc(E, zero_div), Cqo])
  }
  fn op_and_check_overflow(
    &mut self,
    (int_1_x, float_kind, int_1_a, _): Arith,
    func: &mut Pos<BuiltIn>,
    scope: &mut Scope,
  ) -> ErrOR<()> {
    if self.flags.debug && float_kind != Div {
      let cc = if self.flags.a64 {
        if float_kind == Mul {
          scope.e_a(vec![
            SMulH(X3, X0, X1),
            int_1_a(X0, X0, X1),
            Asr(X4, X0, 63),
            CmpRR(X3, X4),
          ])?;
          Ne
        } else {
          scope.e_a(vec![int_1_a(X0, X0, X1)])?;
          O
        }
      } else {
        match int_1_x {
          Ok(g1) => scope.e_x(vec![RR(S8, g1, Rax, Rcx)]),
          Err((inst, _)) => scope.e_x(vec![inst]),
        }?;
        O
      };
      let overflow = self.runtime_err(RuntimeOverflow, None, func.pos, scope.id)?;
      scope.push_jcc(cc, overflow);
    } else {
      if self.flags.a64 {
        scope.p_a(int_1_a(X0, X0, X1))?;
      } else {
        match int_1_x {
          Ok(g1) => scope.e_x(vec![RR(S8, g1, Rax, Rcx)]),
          Err((inst, _)) => scope.e_x(vec![inst]),
        }?;
      }
    }
    Ok(())
  }
  pub(crate) fn rest_float(
    &mut self,
    float: Bind<f64>,
    float_kind: ArithSdKind,
    scope: &mut Scope,
  ) -> ErrOR<()> {
    if self.flags.a64 {
      scope.e_a(self.load_dn(X1, X0, float)?)?;
      scope.p_a(FArithD(float_kind, X0, X0, X1))
    } else {
      scope.e_x(self.load_xmm(Rcx, Rax, float)?)?;
      scope.p_x(ArithSd(float_kind, Rax, Rcx))
    }
  }
  pub(crate) fn rest_op(
    &mut self,
    bind: Bind<i64>,
    (mut int_x, float_kind, int_a, check_a): Arith,
    func: &mut Pos<BuiltIn>,
    scope: &mut Scope,
  ) -> ErrOR<()> {
    if self.flags.a64 {
      scope.e_a(load_int_a(X1, bind)?)?;
      if let Some(check) = check_a {
        scope.e_a(check(self, func.pos, scope.id)?)?;
      }
    } else {
      if let Ok(g1) = int_x {
        int_x = match bind {
          Lit(lit) if i32::try_from(lit).is_ok() => Err((m8i(g1, Rax, i32::try_from(lit)?), None)),
          Var(mem) => Err((r_m(S8, g1, Rax, mem.0), None)),
          _ => {
            scope.e_x(load_int_x(Rcx, bind)?)?;
            int_x
          }
        };
      } else {
        scope.e_x(load_int_x(Rcx, bind)?)?;
      }
      if let Err((_, Some(check))) = int_x {
        scope.e_x(check(self, func.pos, scope.id)?)?;
      }
    }
    self.op_and_check_overflow((int_x, float_kind, int_a, check_a), func, scope)?;
    Ok(())
  }
  fn shift(
    &mut self,
    direction: ShiftDirection,
    func: &mut Pos<BuiltIn>,
    scope: &mut Scope,
  ) -> ErrOR<Json> {
    let lhs = arg!(func, (Int(x)) => x).val;
    let Pos { val: rhs, pos } = arg!(func, (Int(x)) => x);
    if let (Lit(lit1), Lit(lit2)) = (&lhs, &rhs) {
      let Ok(rhs_u32) = u32::try_from(*lit2) else { return err!(pos, TooLargeShift) };
      let checked = if direction == Shl { i64::checked_shl } else { i64::checked_shr };
      let Some(result) = checked(*lit1, rhs_u32) else {
        return err!(pos, TooLargeShift);
      };
      return Ok(Int(Lit(result)));
    }
    if self.flags.a64 {
      scope.e_a(load_int_a(X0, lhs)?)?;
    } else {
      scope.e_x(load_int_x(Rax, lhs)?)?;
    }
    if let Lit(lit2) = rhs {
      if let Ok(rhs_u8) = u8::try_from(lit2)
        && lit2 < 64
      {
        if self.flags.a64 {
          scope.p_a(if direction == Shl { Lsl(X0, X0, rhs_u8) } else { Asr(X0, X0, rhs_u8) })?;
        } else {
          scope.p_x(ShiftR(direction, Rax, Shift::Ib(rhs_u8)))?;
        }
      } else {
        return err!(pos, TooLargeShift);
      }
    } else {
      if self.flags.a64 {
        scope.ee_a(vec![load_int_a(X1, rhs)?, load_imm_a(X2, 64), vec![CmpRR(X1, X2)]])?;
      } else {
        scope.ee_x(vec![load_int_x(Rcx, rhs)?, vec![m8i(Cmp, Rcx, 64)]])?;
      }
      let too_large_shift = self.runtime_err(RuntimeTooLargeShift, None, pos, scope.id)?;
      scope.push_jcc(Ge, too_large_shift);
      if self.flags.a64 {
        scope.p_a(if direction == Shl { LslR3(X0, X0, X1) } else { AsrR3(X0, X0, X1) })?;
      } else {
        scope.p_x(ShiftR(direction, Rax, Shift::Cl))?;
      }
    }
    Ok(Int(Var(if self.flags.a64 { scope.ret_a(S8, X1, X0)? } else { scope.ret_x(S8, Rax)? })))
  }
}
fn constant_fold(
  first: Pos<Bind<i64>>,
  rest: Vec<Pos<Bind<i64>>>,
  (first_op, rest_op): (&Op, &Op),
  ident_elem: i64,
  when0: &impl Fn() -> ErrOR<Option<i64>>,
) -> ErrOR<(Bind<i64>, Vec<Pos<Memory>>, i64)> {
  let mut vars = vec![];
  let mut acc = ident_elem;
  for bind in rest {
    match bind.val {
      Lit(lit) => acc = rest_op(acc, lit).ok_or(compilation!(bind.pos, Overflow))?,
      Var(mem) => vars.push(bind.pos.with(mem)),
    }
  }
  let folded_first = if acc == 0
    && let Some(ret_val) = when0()?
  {
    Lit(ret_val)
  } else {
    match first.val {
      Var(mem) => Var(mem),
      Lit(lit) => Lit(first_op(lit, acc).ok_or(compilation!(first.pos, Overflow))?),
    }
  };
  Ok((folded_first, vars, acc))
}
