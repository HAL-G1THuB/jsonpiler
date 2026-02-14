use crate::{
  Arity::{AtLeast, Exactly, NoArgs},
  Bind::{self, Lit, Var},
  CompilationErrKind::*,
  ErrOR, FuncInfo,
  Inst::{self, *},
  Json, Jsonpiler,
  JsonpilerErr::*,
  Label,
  LogicByteOpcode::*,
  Position,
  Register::*,
  ScopeInfo, WithPos, built_in, err, take_arg,
  utility::{args_type_error, mov_float_reg, mov_float_xmm, mov_int, mov_q, take_float, take_int},
};
built_in! {self, _func, scope, arithmetic;
  abs => {"abs", COMMON, Exactly(1), {
    let arg = _func.arg()?;
    if let Json::Int(int) = arg.value {
      mov_int(&int, Rax, scope);
      scope.extend(&[
        Custom(&Jsonpiler::CQO),
        LogicRR(Xor, Rax, Rdx),
        SubRR(Rax, Rdx)
      ]);
      Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
    } else if let Json::Float(float) = arg.value {
      const BTR_RAX_63: &[u8] = &[0x48, 0x0F, 0xBA, 0xF0, 0x3F];
      mov_float_reg(&float, Rax, scope);
      scope.push(Custom(&BTR_RAX_63));
      Ok(Json::Float(Var(scope.mov_tmp(Rax)?)))
    } else {
      Err(args_type_error(1, &_func.name, "Int` or `Float".into(), &arg))
  }  }},
  add => {"+", COMMON, AtLeast(2), {
    arithmetic_template(
      &(vec![AddRR(Rax, Rcx)], AddSd(Rax, Rcx)),
      (&i64::wrapping_add, &i64::wrapping_add),
      _func, scope, 0, &|_| Ok(None), &||Some(IncR(Rax))
    )
  }},
  div => {"/", COMMON, AtLeast(2), {
    arithmetic_template(
      &(vec![Custom(&Jsonpiler::CQO), IDivR(Rcx)], DivSd(Rax, Rcx)),
      (&i64::wrapping_div, &i64::wrapping_mul),
      _func, scope, 1, &|pos| err!(self, pos, ZeroDivisionError), &||None)
  }},
  float => {"Float", COMMON, Exactly(1), {
    take_int(Rax, _func, scope)?;
    scope.push(CvtSi2Sd(Rax, Rax));
    scope.mov_tmp_xmm(Rax)
  }},
  int => {"Int", COMMON, Exactly(1), {
    take_float(Rax, Rax, _func, scope)?;
    scope.push(CvtTSd2Si(Rax, Rax));
    Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
  }},
  minus => {"-", COMMON, AtLeast(1), {
    if _func.len == 1 {
      match _func.arg()? {
      WithPos { value: Json::Int(int), .. } => {
        mov_int(&int, Rax, scope);
        scope.push(NegR(Rax));
        Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
      }
      WithPos { value: Json::Float(float), .. } => {
        const BTC_RAX_63: &[u8] = &[0x48, 0x0F, 0xBA, 0xF8, 0x3F];
        mov_float_reg(&float, Rax, scope);
        scope.push(Custom(&BTC_RAX_63));
        Ok(Json::Float(Var(scope.mov_tmp(Rax)?)))
      }
      other => {
        Err(args_type_error(1, &_func.name, "Int` or `Float".into(), &other))
      }
    }
    } else {
      arithmetic_template(&(vec![SubRR(Rax, Rcx)], SubSd(Rax, Rcx)), (&i64::wrapping_sub, &i64::wrapping_add), _func, scope, 0, &|_| Ok(None), &||Some(DecR(Rax)))
    }
  }},
  mul => {"*", COMMON, AtLeast(2), {
    arithmetic_template(&(vec![IMulRR(Rax, Rcx)], MulSd(Rax, Rcx)), (&i64::wrapping_mul, &i64::wrapping_mul), _func, scope, 1, &|_| Ok(Some(0)), &||None)
  }},
  random => {"random", COMMON, NoArgs, {
    scope.push(Call(self.get_random()?));
    Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
  }},
  rem => {"%", COMMON, Exactly(2), {
    let int1 = take_arg!(self, _func, (Int(x)) => x);
    let int2 = take_arg!(self, _func, (Int(x)) => x);
    if let (Lit(l_int1), Lit(l_int2)) = (&int1.value, &int2.value) {
      if *l_int2 == 0 {
        return err!(self, int2.pos, ZeroDivisionError);
      }
      return Ok(Json::Int(Lit(l_int1.wrapping_rem(*l_int2))));
    }
    mov_int(&int1.value, Rax, scope);
    match int2.value {
      Lit(l_int) => {
        if l_int == 0 {
          return err!(self, int2.pos, ZeroDivisionError);
        }
        mov_int(&Lit(l_int), Rcx, scope);
      }
      Var(label) => {
        scope.push(mov_q(Rcx, label.mem));
      }
    }
    scope.push(Custom(&Jsonpiler::CQO));
    scope.push(IDivR(Rcx));
    Ok(Json::Int(Var(scope.mov_tmp(Rdx)?)))
  }},
}
type Op = dyn Fn(i64, i64) -> i64;
fn arithmetic_template(
  op_inst: &(Vec<Inst>, Inst), ops: (&Op, &Op), func: &mut FuncInfo, scope: &mut ScopeInfo,
  ident_elem: i64, case_zero: &impl Fn(Position) -> ErrOR<Option<i64>>,
  case_one: &impl Fn() -> Option<Inst>,
) -> ErrOR<Json> {
  match func.arg()? {
    WithPos { value: Json::Int(int), .. } => {
      let mut int_vec = vec![];
      for _ in 1..func.len {
        int_vec.push(take_arg!(self, func, (Int(x)) => x).value);
      }
      let (first, vars, acc) =
        constant_fold(&int, int_vec, ops, ident_elem, &|| case_zero(func.pos))?;
      if first.is_none() && vars.is_empty() {
        return Ok(Json::Int(Lit(acc)));
      }
      if let Some(lbl) = first {
        if acc == 0 {
          if let Some(ret_val) = case_zero(func.pos)? {
            return Ok(Json::Int(Lit(ret_val)));
          }
          mov_int(&Var(lbl), Rax, scope);
        } else if acc == 1 {
          mov_int(&Var(lbl), Rax, scope);
          if let Some(inst) = case_one() {
            scope.push(inst);
          }
        } else {
          mov_int(&Var(lbl), Rax, scope);
          mov_int(&Lit(acc), Rcx, scope);
          scope.extend(&op_inst.0);
        }
      } else {
        mov_int(&Lit(acc), Rax, scope);
      }
      for var in vars {
        mov_int(&Var(var), Rcx, scope);
        scope.extend(&op_inst.0);
      }
      Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
    }
    WithPos { value: Json::Float(float), .. } => {
      mov_float_xmm(&float, Rax, Rax, scope)?;
      for _ in 1..func.len {
        take_float(Rcx, Rax, func, scope)?;
        scope.push(op_inst.1.clone());
      }
      scope.mov_tmp_xmm(Rax)
    }
    other => Err(args_type_error(1, &func.name, "Int` or `Float".into(), &other)),
  }
}
fn constant_fold(
  first: &Bind<i64>, rest: Vec<Bind<i64>>, ops: (&Op, &Op), ident_elem: i64,
  case_zero: &impl Fn() -> ErrOR<Option<i64>>,
) -> ErrOR<(Option<Label>, Vec<Label>, i64)> {
  let mut vars = vec![];
  let mut acc = ident_elem;
  for bind in rest {
    match bind {
      Lit(l_int) => acc = ops.1(acc, l_int),
      Var(lbl) => vars.push(lbl),
    }
  }
  match first {
    Lit(l_int) => {
      if acc == 0
        && let Some(ret_val) = case_zero()?
      {
        return Ok((None, vars, ret_val));
      }
      acc = ops.0(*l_int, acc);
      Ok((None, vars, acc))
    }
    Var(lbl) => Ok((Some(*lbl), vars, acc)),
  }
}
