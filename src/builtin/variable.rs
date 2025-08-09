use crate::{
  Arity::Exactly,
  Bind::{self, Lit, Var},
  ErrOR, FuncInfo, Json, Jsonpiler, ScopeInfo, built_in, err, mn, take_arg,
};
impl Jsonpiler {
  fn assign(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo, is_global: bool) -> ErrOR<Json> {
    let (variable, pos) = take_arg!(self, func, 1, "String", Json::String(Lit(x)) => x);
    let json2 = func.arg()?;
    let value = match json2.value {
      Json::Function(asm_func) => {
        if self.builtin.contains_key(&variable) {
          return err!(self, pos, "Name conflict with a built-in function.");
        }
        Json::Function(asm_func)
      }
      Json::String(Lit(st)) => Json::String(Var(self.global_str(&st)?)),
      Json::String(Var(str_label)) => {
        let label = if is_global { self.get_bss(8) } else { scope.local(8) }?;
        scope.body.push(mn!("lea", label, str_label));
        Json::Int(Var(label))
      }
      Json::Null => Json::Null,
      Json::Int(Lit(int)) if is_global => Json::Int(Var(self.global_num(int)?)),
      Json::Int(int) => {
        let label = if is_global { self.get_bss(8) } else { scope.local(8) }?;
        let int_str = match int {
          Lit(l_int) => l_int.to_string(),
          Var(int_label) => format!("{int_label}"),
        };
        scope.body.push(mn!("mov", label, int_str));
        Json::Int(Var(label))
      }
      Json::Bool(Lit(boolean)) if is_global => Json::Bool(Var(self.global_bool(boolean)?)),
      Json::Bool(boolean) => {
        let label = if is_global { self.get_bss(1) } else { scope.local(1) }?;
        let bool_str = match boolean {
          Lit(l_bool) => if l_bool { "0xFF" } else { "0x00" }.to_owned(),
          Var(bool_label) => format!("{bool_label}"),
        };
        scope.body.push(mn!("mov", label, bool_str));
        Json::Bool(Var(label))
      }
      Json::Float(Lit(float)) if is_global => Json::Float(Var(self.global_num(float.to_bits())?)),
      Json::Float(float) => {
        let label = if is_global { self.get_bss(8) } else { scope.local(8) }?;
        let float_str = match float {
          Bind::Lit(l_float) => format!("{:#016x}", l_float.to_bits()),
          Bind::Var(float_label) => float_label.sched_free_2str(func),
        };
        scope.body.push(mn!("mov", label, float_str));
        Json::Float(Var(label))
      }
      Json::Array(_) | Json::Object(_) => {
        return Err(
          self
            .parser
            .type_err(
              2,
              &func.name,
              "that supports assignment (excluding Array and Object)",
              &json2,
            )
            .into(),
        );
      }
    };
    if if is_global {
      &mut self.globals
    } else {
      scope.locals.last_mut().ok_or("InternalError: Invalid scope.")?
    }
    .insert(variable, value)
    .is_some()
    {
      return err!(self, pos, "Reassignment may not be possible in some scope.");
    }
    Ok(Json::Null)
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
    let (var_name, pos) = take_arg!(self, func, 1, "String(Literal)", Json::String(Lit(x)) => x);
    match self.get_var(&var_name, scope) {
      Some(var) => Ok(var),
      None => err!(self, pos, "Undefined variables: `{var_name}`"),
    }
  }},
  scope => {"scope", SP_SCOPE, Exactly(1), {
    let (object, object_pos) = take_arg!(self, func, 1, "Sequence", Json::Object(Lit(x)) => x);
    self.eval_object(object, object_pos, scope)
  }}
}
