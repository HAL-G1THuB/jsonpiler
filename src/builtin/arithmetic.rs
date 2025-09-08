use crate::{
  Arity::{AtLeast, Exactly, NoArgs},
  Bind::{self, Lit, Var},
  ConditionCode::*,
  ErrOR, FuncInfo,
  Inst::{self, *},
  Json, Jsonpiler, Label,
  LogicByteOpcode::*,
  Register::*,
  ScopeInfo, WithPos, built_in, err, take_arg,
  utility::{mov_float_reg, mov_float_xmm, mov_int, mov_q},
};
built_in! {self, _func, scope, arithmetic;
  abs => {"abs", COMMON, Exactly(1), {
    let arg = _func.arg()?;
    if let Json::Int(int) = arg.value {
      mov_int(&int, Rax, scope);
      scope.push(Custom(&Jsonpiler::CQO));
      scope.push(LogicRR(Xor, Rax, Rdx));
      scope.push(SubRR(Rax, Rdx));
      Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
    } else if let Json::Float(float) = arg.value {
      const BTR_RAX_63: &[u8] = &[0x48, 0x0F, 0xBA, 0xF0, 0x3F];
      mov_float_reg(&float, Rax, scope);
      scope.push(Custom(&BTR_RAX_63));
      Ok(Json::Float(Var(scope.mov_tmp(Rax)?)))
    } else {
      Err(self.parser[arg.pos.file].args_type_error(1, &_func.name, "Int` or `Float", &arg).into())
  }  }},
  add => {"+", COMMON, AtLeast(2), {
    self.arithmetic_template(&(AddRR(Rax, Rcx), AddSd(Rax, Rcx)), &i64::checked_add, &i64::checked_add, _func, scope, 0)
  }},
  div => {"/", COMMON, AtLeast(2), {
    let arg = _func.arg()?;
    if let Json::Int(int) = arg.value {
      let mut int_vec = vec![];
      for _ in 1.._func.len {
        int_vec.push(take_arg!(self, _func, "Int", Json::Int(x) => x).value);
      }
      let op_one = |x, y| match i64::checked_div(x, y) {
        Some(l_int) => Ok(l_int),
        _ => err!(self, _func.pos, "ZeroDivisionError"),
      };
      let op_two = |x, y| match i64::checked_mul(x, y) {
        Some(l_int) => Ok(l_int),
        _ => err!(self, _func.pos, "Overflow"),
      };
      let (first, vars, acc) = constant_fold(&int, int_vec, op_one, op_two, 1)?;
      if first.is_none() && vars.is_empty() {
        return Ok(Json::Int(Lit(acc)));
      }
      if acc == 0 {
        return err!(self, _func.pos, "ZeroDivisionError");
      }
      #[expect(clippy::cast_sign_loss)]
      if let Some(lbl) = first {
        mov_int(&Var(lbl), Rax, scope);
        scope.push(mov_q(Rcx, acc as u64));
        scope.push(Custom(&Jsonpiler::CQO));
        scope.push(IDivR(Rcx));
      } else {
        scope.push(mov_q(Rax, acc as u64));
      }
      for label in vars {
        scope.push(mov_q(Rcx, label.mem));
        scope.push(LogicRR(Test, Rcx, Rcx));
        let zero_division_err = self.get_custom_error("ZeroDivisionError")?;
        scope.push(JCc(E, zero_division_err));
        scope.push(Custom(&Jsonpiler::CQO));
        scope.push(IDivR(Rcx));
      }
      Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
    } else if let Json::Float(float) = arg.value {
      mov_float_xmm(&float, Rax, Rax, scope)?;
        for _ in 1.._func.len {
          self.take_float(Rcx, Rax, _func, scope)?;
          scope.push(DivSd(Rax, Rcx));
        }
        scope.mov_tmp_xmm(Rax)
    } else {
      Err(self.parser[arg.pos.file].args_type_error(1, &_func.name, "Int` or `Float", &arg).into())
    }
  }},
  float => {"Float", COMMON, Exactly(1), {
    self.take_int(Rax, _func, scope)?;
    scope.push(CvtSi2Sd(Rax, Rax));
    scope.mov_tmp_xmm(Rax)
  }},
  int => {"Int", COMMON, Exactly(1), {
    self.take_float(Rax, Rax, _func, scope)?;
    scope.push(CvtTSd2Si(Rax, Rax));
    Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
  }},
  minus => {"-", COMMON, AtLeast(1), {
    const BTC_RAX_63: &[u8] = &[0x48, 0x0F, 0xBA, 0xF8, 0x3F];
    if _func.len == 1 {
      match _func.arg()? {
      WithPos { value: Json::Int(int), .. } => {
        mov_int(&int, Rax, scope);
        scope.push(NegR(Rax));
        Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
      }
      WithPos { value: Json::Float(float), .. } => {
        mov_float_reg(&float, Rax, scope);
        scope.push(Custom(&BTC_RAX_63));
        Ok(Json::Float(Var(scope.mov_tmp(Rax)?)))
      }
      other => {
        Err(self.parser[other.pos.file].args_type_error(1, &_func.name, "Int` or `Float", &other).into())
      }
    }
    } else {
      self.arithmetic_template(&(SubRR(Rax, Rcx), SubSd(Rax, Rcx)), &i64::checked_sub, &i64::checked_add, _func, scope, 0)
    }
  }},
  mul => {"*", COMMON, AtLeast(2), {
    self.arithmetic_template(&(IMulRR(Rax, Rcx), MulSd(Rax, Rcx)), &i64::checked_mul, &i64::checked_mul, _func, scope, 1)
  }},
  random => {"random", COMMON, NoArgs, {
    scope.push(Call(self.get_random()?));
    Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
  }},
  rem => {"%", COMMON, Exactly(2), {
    let int1 = take_arg!(self, _func, "Int", Json::Int(x) => x);
    let int2 = take_arg!(self, _func, "Int", Json::Int(x) => x);
    if let (Lit(l_int1), Lit(l_int2)) = (&int1.value, &int2.value) {
      let Some(ret) = l_int1.checked_rem(*l_int2) else {
        return err!(self, int2.pos, "ZeroDivisionError");
      };
      return Ok(Json::Int(Lit(ret)));
    }
    mov_int(&int1.value, Rax, scope);
    match int2.value {
      Lit(l_int) => {
        if l_int == 0 {
          return err!(self, int2.pos, "ZeroDivisionError");
        }
        #[expect(clippy::cast_sign_loss)]
        scope.push(mov_q(Rcx, l_int as u64));
      }
      Var(label) => {
        scope.push(mov_q(Rcx, label.mem));
        scope.push(LogicRR(Test, Rcx, Rcx));
        let zero_division_err = self.get_custom_error("ZeroDivisionError")?;
        scope.push(JCc(E, zero_division_err));
      }
    }
    scope.push(Custom(&Jsonpiler::CQO));
    scope.push(IDivR(Rcx));
    Ok(Json::Int(Var(scope.mov_tmp(Rdx)?)))
  }},
}
impl Jsonpiler {
  #[expect(clippy::cast_sign_loss)]
  fn arithmetic_template(
    &self, op_inst: &(Inst, Inst), op1: &dyn Fn(i64, i64) -> Option<i64>,
    op2: &dyn Fn(i64, i64) -> Option<i64>, func: &mut FuncInfo, scope: &mut ScopeInfo,
    ident_elem: i64,
  ) -> ErrOR<Json> {
    match func.arg()? {
      WithPos { value: Json::Int(int), .. } => {
        let mut int_vec = vec![];
        for _ in 1..func.len {
          int_vec.push(take_arg!(self, func, "Int", Json::Int(x) => x).value);
        }
        let op_one = |x, y| match op1(x, y) {
          Some(l_int) => Ok(l_int),
          _ => err!(self, func.pos, "Overflow"),
        };
        let op_two = |x, y| match op2(x, y) {
          Some(l_int) => Ok(l_int),
          _ => err!(self, func.pos, "Overflow"),
        };
        let (first, vars, acc) = constant_fold(&int, int_vec, op_one, op_two, ident_elem)?;
        if first.is_none() && vars.is_empty() {
          return Ok(Json::Int(Lit(acc)));
        }
        if let Some(lbl) = first {
          mov_int(&Var(lbl), Rax, scope);
          scope.push(mov_q(Rcx, acc as u64));
          scope.push(op_inst.0.clone());
        } else {
          scope.push(mov_q(Rax, acc as u64));
        }
        for var in vars {
          mov_int(&Var(var), Rcx, scope);
          scope.push(op_inst.0.clone());
        }
        Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
      }
      WithPos { value: Json::Float(float), .. } => {
        mov_float_xmm(&float, Rax, Rax, scope)?;
        for _ in 1..func.len {
          self.take_float(Rcx, Rax, func, scope)?;
          scope.push(op_inst.1.clone());
        }
        scope.mov_tmp_xmm(Rax)
      }
      other => Err(
        self.parser[other.pos.file].args_type_error(1, &func.name, "Int` or `Float", &other).into(),
      ),
    }
  }
}
fn constant_fold(
  first: &Bind<i64>, rest: Vec<Bind<i64>>, op1: impl Fn(i64, i64) -> ErrOR<i64>,
  op2: impl Fn(i64, i64) -> ErrOR<i64>, ident_elem: i64,
) -> ErrOR<(Option<Label>, Vec<Label>, i64)> {
  let mut vars = vec![];
  let mut acc = ident_elem;
  for bind in rest {
    match bind {
      Lit(l_int) => acc = op2(acc, l_int)?,
      Var(lbl) => vars.push(lbl),
    }
  }
  match first {
    Lit(l_int) => {
      acc = op1(*l_int, acc)?;
      Ok((None, vars, acc))
    }
    Var(lbl) => Ok((Some(*lbl), vars, acc)),
  }
}
