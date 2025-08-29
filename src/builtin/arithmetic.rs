use crate::{
  Arity::{AtLeast, Exactly, NoArgs},
  Bind::{Lit, Var},
  ConditionCode::*,
  ErrOR, FuncInfo,
  Inst::{self, *},
  Json, Jsonpiler,
  OpQ::{Iq, Mq, Rq},
  Reg::*,
  ScopeInfo, WithPos, built_in, err, take_arg,
  utility::{mov_float_reg, mov_int},
};
built_in! {self, _func, scope, arithmetic;
  abs => {"abs", COMMON, Exactly(1), {
    let arg = _func.arg()?;
    if let Json::Int(int) = arg.value {
      mov_int(&int, Rax, scope);
      scope.push(Custom(Jsonpiler::CQO.to_vec()));
      scope.push(XorRR(Rax, Rdx));
      scope.push(SubRR(Rax, Rdx));
      Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
    } else if let Json::Float(float) = arg.value {
      const BTR_RAX_63: [u8; 5] = [0x48, 0x0F, 0xBA, 0xF0, 0x3F];
      mov_float_reg(&float, Rax, scope);
      scope.push(Custom(BTR_RAX_63.to_vec()));
      Ok(Json::Float(Var(scope.mov_tmp(Rax)?)))
    } else {
      Err(self.parser[arg.pos.file].args_type_error(1, &_func.name, "Int` or `Float", &arg).into())
  }  }},
  add => {"+", COMMON, AtLeast(2), {
    self.arithmetic_template(&AddRR(Rax, Rcx), &AddSd(Rax, Rcx), _func, scope)
  }},
  div => {"/", COMMON, AtLeast(2), {
    let arg = _func.arg()?;
    if let Json::Int(int) = arg.value {
      mov_int(&int, Rax, scope);
      for _ in 1.._func.len {
        self.mov_rcx_nonzero(scope, _func)?;
        scope.push(Custom(Jsonpiler::CQO.to_vec()));
        scope.push(IDivR(Rcx));
      }
      Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
    } else if let Json::Float(float) = arg.value {
      self.mov_float_xmm(&float, Rax, Rax, scope);
        for _ in 1.._func.len {
          self.take_float(Rcx, Rax, _func, scope)?;
          scope.push(DivSd(Rax, Rcx));
        }
        let tmp = scope.tmp(8, 8)?;
        scope.push(MovSdMX(tmp.kind, Rax));
        Ok(Json::Float(Var(tmp)))
    } else {
      Err(self.parser[arg.pos.file].args_type_error(1, &_func.name, "Int` or `Float", &arg).into())
    }
  }},
  int => {"Int", COMMON, Exactly(1), {
    self.take_float(Rax, Rax, _func, scope)?;
    scope.push(CvtTSd2Si(Rax, Rax));
    Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
  }},
  minus => {"-", COMMON, AtLeast(1), {
    const BTC_RAX_63: [u8; 5] = [0x48, 0x0F, 0xBA, 0xF8, 0x3F];
    if _func.len == 1 {
      match _func.arg()? {
      WithPos { value: Json::Int(int), .. } => {
        mov_int(&int, Rax, scope);
        scope.push(NegR(Rax));
        Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
      }
      WithPos { value: Json::Float(float), .. } => {
        mov_float_reg(&float, Rax, scope);
        scope.push(Custom(BTC_RAX_63.to_vec()));
        Ok(Json::Float(Var(scope.mov_tmp(Rax)?)))
      }
      other => {
        Err(self.parser[other.pos.file].args_type_error(1, &_func.name, "Int` or `Float", &other).into())
      }
    }
    } else {
      self.arithmetic_template(&SubRR(Rax, Rcx), &SubSd(Rax, Rcx), _func, scope)
    }
  }},
  mul => {"*", COMMON, AtLeast(2), {
    self.arithmetic_template(&IMulRR(Rax, Rcx), &MulSd(Rax, Rcx), _func, scope)
  }},
  random => {"random", COMMON, NoArgs, {
    scope.push(Call(self.get_random()));
    Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
  }},
  rem => {"%", COMMON, Exactly(2), {
    self.take_int(Rax, _func, scope)?;
    self.mov_rcx_nonzero(scope, _func)?;
    scope.push(Custom(Jsonpiler::CQO.to_vec()));
    scope.push(IDivR(Rcx));
    Ok(Json::Int(Var(scope.mov_tmp(Rdx)?)))
  }},
}
impl Jsonpiler {
  fn arithmetic_template(
    &self, int_inst: &Inst, float_inst: &Inst, func: &mut FuncInfo, scope: &mut ScopeInfo,
  ) -> ErrOR<Json> {
    match func.arg()? {
      WithPos { value: Json::Int(int), .. } => {
        mov_int(&int, Rax, scope);
        for _ in 1..func.len {
          self.take_int(Rcx, func, scope)?;
          scope.push(int_inst.clone());
        }
        Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
      }
      WithPos { value: Json::Float(float), .. } => {
        self.mov_float_xmm(&float, Rax, Rax, scope);
        for _ in 1..func.len {
          self.take_float(Rcx, Rax, func, scope)?;
          scope.push(float_inst.clone());
        }
        let tmp = scope.tmp(8, 8)?;
        scope.push(MovSdMX(tmp.kind, Rax));
        Ok(Json::Float(Var(tmp)))
      }
      other => Err(
        self.parser[other.pos.file].args_type_error(1, &func.name, "Int` or `Float", &other).into(),
      ),
    }
  }
  fn mov_rcx_nonzero(&mut self, scope: &mut ScopeInfo, func: &mut FuncInfo) -> ErrOR<()> {
    let int = take_arg!(self, func, "Int", Json::Int(x) => x);
    match int.value {
      Lit(l_int) => {
        if l_int == 0 {
          return err!(self, int.pos, "ZeroDivisionError");
        }
        #[expect(clippy::cast_sign_loss)]
        scope.push(MovQQ(Rq(Rcx), Iq(l_int as u64)));
      }
      Var(label) => {
        scope.push(MovQQ(Rq(Rcx), Mq(label.kind)));
        scope.push(CmpRIb(Rcx, 0));
        let zero_division_err = self.get_custom_error("ZeroDivisionError");
        scope.push(Jcc(E, zero_division_err));
      }
    }
    Ok(())
  }
}
