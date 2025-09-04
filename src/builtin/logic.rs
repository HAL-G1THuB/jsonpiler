use crate::{
  Arity::{AtLeast, Exactly},
  Bind::Var,
  ConditionCode::*,
  ErrOR, FuncInfo,
  Inst::*,
  Json, Jsonpiler,
  LogicByteOpcode::{self, *},
  Register::*,
  ScopeInfo, built_in,
  utility::{mov_bool, mov_d, mov_int, mov_q},
};
built_in! {self, func, scope, logic;
  and => {"and", COMMON, AtLeast(2), {
    self.logic_template(And, func, scope)
  }},
  assert => {"assert", COMMON, Exactly(2), {
    let message_box_w = self.import(Jsonpiler::USER32, "MessageBoxW")?;
    self.take_bool(Rax, func, scope)?;
    scope.push(LogicRbRb(Test, Rax, Rax));
    let error_label = self.gen_id();
    let end_label = self.gen_id();
    scope.push(Jcc(E, error_label));
    scope.push(Jmp(end_label));
    scope.push(Lbl(error_label));
    self.take_str(Rcx, func, scope)?;
    scope.push(Call(self.get_u8_to_16()?));
    scope.push(mov_q(Rdx, Rax));
    scope.push(Clear(Rcx));
    scope.push(Clear(R8));
    scope.push(mov_d(R9, 0x10));
    scope.extend(&self.call_api_check_null(message_box_w));
    scope.push(mov_d(Rcx, 1));
    scope.push(CallApi(self.import(Jsonpiler::KERNEL32, "ExitProcess")?));
    scope.push(Lbl(end_label));
    Ok(Json::Null)
  }},
  not => {"not", COMMON, Exactly(1), {
    let arg = func.arg()?;
    if let Json::Bool(boolean) = arg.value {
      mov_bool(&boolean, Rax, scope);
      scope.push(NotRb(Rax));
      scope.mov_tmp_bool(Rax)
    } else if let Json::Int(int) = arg.value {
      mov_int(&int, Rax, scope);
      scope.push(NotR(Rax));
      Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
    } else {
      Err(self.parser[arg.pos.file].args_type_error(1, &func.name, "Int` or `Bool", &arg).into())
    }
  }},
  or => {"or", COMMON, AtLeast(2), {
    self.logic_template(Or, func, scope)
  }},
  xor => {"xor", COMMON, AtLeast(2), {
    self.logic_template(Xor, func, scope)
  }},
}
impl Jsonpiler {
  pub(crate) fn logic_template(
    &self, logic_op: LogicByteOpcode, func: &mut FuncInfo, scope: &mut ScopeInfo,
  ) -> ErrOR<Json> {
    let arg = func.arg()?;
    if let Json::Bool(boolean) = arg.value {
      mov_bool(&boolean, Rax, scope);
      for _ in 1..func.len {
        self.take_bool(Rcx, func, scope)?;
        scope.push(LogicRbRb(logic_op, Rax, Rcx));
      }
      scope.mov_tmp_bool(Rax)
    } else if let Json::Int(int) = arg.value {
      mov_int(&int, Rax, scope);
      for _ in 1..func.len {
        self.take_int(Rcx, func, scope)?;
        scope.push(LogicRR(logic_op, Rax, Rcx));
      }
      Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
    } else {
      Err(self.parser[arg.pos.file].args_type_error(1, &func.name, "Int` or `Bool", &arg).into())
    }
  }
}
