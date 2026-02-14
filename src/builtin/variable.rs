use crate::{
  Arity::Exactly,
  Bind::{self, Lit, Var},
  CompilationErrKind::*,
  ErrOR, FuncInfo,
  Inst::*,
  InternalErrKind::*,
  Json, Jsonpiler,
  JsonpilerErr::*,
  Label,
  Memory::Global,
  Register::*,
  ScopeInfo, WithPos, built_in,
  dll::*,
  err, get_target_mem, take_arg, take_arg_custom,
  utility::{args_type_error, mov_b, mov_bool, mov_int, mov_q},
};
use core::mem::discriminant;
impl Jsonpiler {
  #[expect(clippy::cast_sign_loss, clippy::too_many_lines)]
  fn assign(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo, is_global: bool) -> ErrOR<Json> {
    let WithPos { value: variable, pos: var_pos } = take_arg!(self, func, (String(Lit(x))) => x);
    let json2 = func.arg()?;
    let ref_label = if is_global {
      if scope.get_var_local(&variable).is_some() {
        return err!(self, var_pos, ExistentVar(variable));
      }
      self.globals.get(&variable).cloned()
    } else {
      scope.get_var_local(&variable)
    };
    if let Some(json) = &ref_label
      && discriminant(json) != discriminant(&json2.value)
    {
      return Err(args_type_error(1, &format!("Variable `{variable}`"), json.type_name(), &json2));
    }
    let value = match json2.value {
      Json::String(string) => {
        if is_global {
          self.enter_c_s(scope)?;
        }
        let mem =
          get_target_mem!(self, scope, is_global, 8, ref_label, (String(Var(label))) => label);
        scope.push(match string {
          Lit(l_str) => {
            let id = self.global_str(l_str).0;
            LeaRM(Rax, Global { id })
          }
          Var(str_label) => mov_q(Rax, str_label.mem),
        });
        scope.push(mov_q(mem, Rax));
        if is_global {
          self.leave_c_s(scope)?;
        }
        Json::String(Var(Label { mem, size: 8 }))
      }
      Json::Null => Json::Null,
      Json::Int(Lit(int)) if is_global && ref_label.is_none() && scope.get_epilogue().is_none() => {
        Json::Int(Var(Label { mem: Global { id: self.global_num(int as u64) }, size: 8 }))
      }
      Json::Int(int) => {
        if is_global {
          self.enter_c_s(scope)?;
        }
        let mem = get_target_mem!(self, scope, is_global, 8, ref_label, (Int(Var(label))) => label);
        mov_int(&int, Rax, scope);
        scope.push(mov_q(mem, Rax));
        if is_global {
          self.leave_c_s(scope)?;
        }
        Json::Int(Var(Label { mem, size: 8 }))
      }
      Json::Bool(Lit(l_bool))
        if is_global && ref_label.is_none() && scope.get_epilogue().is_none() =>
      {
        Json::Bool(Var(Label { mem: Global { id: self.global_bool(l_bool) }, size: 1 }))
      }
      Json::Bool(boolean) => {
        if is_global {
          self.enter_c_s(scope)?;
        }
        let mem =
          get_target_mem!(self, scope, is_global, 1, ref_label, (Bool(Var(label))) => label);
        mov_bool(&boolean, Rax, scope);
        scope.push(mov_b(mem, Rax));
        if is_global {
          self.leave_c_s(scope)?;
        }
        Json::Bool(Var(Label { mem, size: 1 }))
      }
      Json::Float(Lit(l_float))
        if is_global && ref_label.is_none() && scope.get_epilogue().is_none() =>
      {
        Json::Float(Var(Label { mem: Global { id: self.global_num(l_float.to_bits()) }, size: 8 }))
      }
      Json::Float(float) => {
        if is_global {
          self.enter_c_s(scope)?;
        }
        let mem =
          get_target_mem!(self, scope, is_global, 8, ref_label, (Float(Var(label))) => label);
        scope.push(match float {
          Bind::Lit(l_float) => mov_q(Rax, l_float.to_bits()),
          Bind::Var(float_label) => mov_q(Rax, float_label.mem),
        });
        scope.push(mov_q(mem, Rax));
        if is_global {
          self.leave_c_s(scope)?;
        }
        Json::Float(Var(Label { mem, size: 8 }))
      }
      Json::Array(_) | Json::Object(_) => {
        return Err(args_type_error(
          2,
          &func.name,
          "Types excluding arrays and objects".into(),
          &json2,
        ));
      }
    };
    if is_global {
      self.globals.insert(variable, value);
    } else if ref_label.is_none() {
      scope.innermost_scope()?.insert(variable, value);
    }
    Ok(Json::Null)
  }
  fn enter_c_s(&mut self, scope: &mut ScopeInfo) -> ErrOR<()> {
    let critical_section = Global { id: self.get_critical_section()? };
    let enter_c_s = self.import(KERNEL32, "EnterCriticalSection")?;
    scope.extend(&[LeaRM(Rcx, critical_section), CallApi(enter_c_s)]);
    Ok(())
  }
  fn leave_c_s(&mut self, scope: &mut ScopeInfo) -> ErrOR<()> {
    let critical_section = Global { id: self.get_critical_section()? };
    let leave_c_s = self.import(KERNEL32, "LeaveCriticalSection")?;
    scope.extend(&[LeaRM(Rcx, critical_section), CallApi(leave_c_s)]);
    Ok(())
  }
}
built_in! {self, func, scope, variable;
  assign_global => {"global", COMMON, Exactly(2), {
    self.assign(func, scope, true)
  }},
  assign_local =>{ "=", COMMON, Exactly(2), {
    self.assign(func, scope, false)
  }},
  reference => {"$", COMMON, Exactly(1), {
    let var_name = take_arg!(self, func, (String(Lit(x))) => x);
    match self.get_var(&var_name.value, scope) {
      Some(var) => Ok(var),
      None => err!(self, var_name.pos, UndefinedVar(var_name.value)),
    }
  }},
  scope => {"scope", SP_SCOPE, Exactly(1), {
    let object = take_arg_custom!(self, func, "Block", (Object(Lit(x))) => x).value;
    self.eval_object(object, scope)
  }}
}
