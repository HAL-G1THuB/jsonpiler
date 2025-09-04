use crate::{
  Arity::Exactly,
  Bind::{self, Lit, Var},
  ErrOR, FuncInfo,
  Inst::*,
  Json, Jsonpiler, Label,
  Register::*,
  ScopeInfo,
  VarKind::Global,
  built_in, err, get_target_mem, take_arg,
  utility::{mov_b, mov_bool, mov_q},
};
use core::mem::discriminant;
impl Jsonpiler {
  #[expect(clippy::cast_sign_loss, clippy::too_many_lines)]
  fn assign(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo, is_global: bool) -> ErrOR<Json> {
    let variable = take_arg!(self, func, "String", Json::String(Lit(x)) => x);
    let json2 = func.arg()?;
    let ref_label = if is_global {
      self.globals.get(&variable.value).cloned()
    } else {
      scope.get_var_local(&variable.value)
    };
    if let Some(json) = &ref_label
      && discriminant(json) != discriminant(&json2.value)
    {
      return Err(
        self.parser[json2.pos.file]
          .args_type_error(1, &format!("Variable `{}`", variable.value), &json.type_name(), &json2)
          .into(),
      );
    }
    let value = match json2.value {
      Json::String(string) => {
        if is_global {
          self.enter_c_s(scope)?;
        }
        let mem = get_target_mem!(
          self, scope, is_global, 8, ref_label,
          Json::String(Var(label)) => label.mem
        );
        scope.push(match string {
          Lit(l_str) => {
            let id = self.global_str(l_str).0;
            LeaRM(Rax, Global { id, disp: 0i32 })
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
      Json::Int(Lit(int)) if is_global => Json::Int(Var(Label {
        mem: Global { id: self.global_num(int as u64), disp: 0i32 },
        size: 8,
      })),
      Json::Int(int) => {
        if is_global {
          self.enter_c_s(scope)?;
        }
        let mem = get_target_mem!(
          self, scope, is_global, 8,ref_label,
          Json::Int(Var(label )) => label.mem
        );
        scope.push(match int {
          Lit(l_int) => mov_q(Rax, l_int as u64),
          Var(int_label) => mov_q(Rax, int_label.mem),
        });
        scope.push(mov_q(mem, Rax));
        if is_global {
          self.leave_c_s(scope)?;
        }
        Json::Int(Var(Label { mem, size: 8 }))
      }
      Json::Bool(Lit(l_bool)) if is_global => {
        Json::Bool(Var(Label { mem: Global { id: self.global_bool(l_bool), disp: 0i32 }, size: 1 }))
      }
      Json::Bool(boolean) => {
        if is_global {
          self.enter_c_s(scope)?;
        }
        let mem = get_target_mem!(
          self, scope, is_global, 1, ref_label,
          Json::Bool(Var(label)) => label.mem
        );
        mov_bool(&boolean, Rax, scope);
        scope.push(mov_b(mem, Rax));
        if is_global {
          self.leave_c_s(scope)?;
        }
        Json::Bool(Var(Label { mem, size: 1 }))
      }
      Json::Float(Lit(l_float)) if is_global => Json::Float(Var(Label {
        mem: Global { id: self.global_num(l_float.to_bits()), disp: 0i32 },
        size: 8,
      })),
      Json::Float(float) => {
        if is_global {
          self.enter_c_s(scope)?;
        }
        let mem = get_target_mem!(
          self, scope, is_global, 8, ref_label,
          Json::Float(Var(label )) =>label.mem);
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
        return Err(
          self.parser[json2.pos.file]
            .args_type_error(2, &func.name, "Types excluding arrays and objects", &json2)
            .into(),
        );
      }
    };
    if is_global {
      self.globals.insert(variable.value, value);
    } else if ref_label.is_none() {
      scope.innermost_scope()?.insert(variable.value, value);
    }
    Ok(Json::Null)
  }
  fn enter_c_s(&mut self, scope: &mut ScopeInfo) -> ErrOR<()> {
    let critical_section = Global { id: self.get_critical_section()?, disp: 0i32 };
    let enter_c_s = self.import(Jsonpiler::KERNEL32, "EnterCriticalSection")?;
    scope.extend(&[LeaRM(Rcx, critical_section), CallApi(enter_c_s)]);
    Ok(())
  }
  fn leave_c_s(&mut self, scope: &mut ScopeInfo) -> ErrOR<()> {
    let critical_section = Global { id: self.get_critical_section()?, disp: 0i32 };
    let leave_c_s = self.import(Jsonpiler::KERNEL32, "LeaveCriticalSection")?;
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
    let var_name = take_arg!(self, func, "String (Literal)", Json::String(Lit(x)) => x);
    match self.get_var(&var_name.value, scope) {
      Some(var) => Ok(var),
      None => err!(self, var_name.pos, "Undefined variables: `{}`", var_name.value),
    }
  }},
  scope => {"scope", SP_SCOPE, Exactly(1), {
    let object = take_arg!(self, func, "Block", Json::Object(Lit(x)) => x);
    self.eval_object(object.value, object.pos, scope)
  }}
}
