use super::super::{
  ArgLen::Exactly,
  Bind::{self, Lit, Var},
  ErrOR, FuncInfo, Json, Jsonpiler, ScopeInfo, err, mn,
  utility::{get_bool_str, get_int_str},
  validate_type,
};
impl Jsonpiler {
  pub(crate) fn variable(&mut self) {
    let common = (false, false);
    let sp_scope = (true, true);
    self.register("global", common, Jsonpiler::assign_global, Exactly(2));
    self.register("=", common, Jsonpiler::assign_local, Exactly(2));
    self.register("$", common, Jsonpiler::reference, Exactly(1));
    self.register("scope", sp_scope, Jsonpiler::scope, Exactly(1));
  }
}
#[expect(clippy::single_call_fn, reason = "")]
impl Jsonpiler {
  fn assign(&mut self,  func: &mut FuncInfo, scope: &mut ScopeInfo, is_global: bool) -> ErrOR<Json> {
    let json1 = func.arg()?;
    let variable = validate_type!(self, func, 1, json1, Json::String(Lit(x)) => x, "String");
    let json2 = func.arg()?;
    let value = match json2.value {
      Json::Function(asm_func) => {
        if self.builtin.contains_key(&variable) {
          return err!(self, json1.pos, "Name conflict with a built-in function.");
        }
        Json::Function(asm_func)
      }
      Json::String(Lit(st)) => Json::String(Var(self.global_str(&st)?)),
      Json::String(Var(_)) if is_global => {
        return err!(self, json2.pos, "Local string cannot be assigned to a global variable.");
      }
      var @ Json::String(Var(_)) => var,
      Json::Null => Json::Null,
      Json::Int(Lit(int)) if is_global => Json::Int(Var(self.global_num(int)?)),
      Json::Int(int) => {
        let label = if is_global { self.get_bss(8) } else { scope.get_local(8) }?;
        let int_str = get_int_str(&int, func);
        scope.body.push(mn!("mov", label, int_str));
        Json::Int(Var(label))
      }
      Json::Bool(boolean) => {
        let label = if is_global { self.get_bss(1) } else { scope.get_local(1) }?;
        let bool_str = get_bool_str(&boolean, func);
        scope.body.push(mn!("mov", label, bool_str));
        Json::Bool(Var(label))
      }
      Json::Float(Lit(float)) if is_global => {
        Json::Float(Var(self.global_num(float.to_bits())?))
      }
      Json::Float(float) => {
        let label = if is_global { self.get_bss(8) } else { scope.get_local(8) }?;
        let float_str = match float {
          Bind::Lit(l_float) => format!("{:#016x}", l_float.to_bits()),
          Bind::Var(float_label) => float_label.sched_free_2str(func),
        };
        scope.body.push(mn!("mov", label, float_str));
        Json::Float(Var(label))
      }
      Json::Array(_) | Json::Object(_) => {
        return self.parser.typ_err(
          2,
          &func.name,
          "that supports assignment (excluding Array and Object)",
          &json2,
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
      return err!(self, json1.pos, "Reassignment may not be possible in some scope.");
    }
    Ok(Json::Null)
  }
  fn assign_global(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.assign(func, scope, true)
  }
  fn assign_local(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.assign(func, scope, false)
  }
  fn reference(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    let json = func.arg()?;
    let var_name =
      validate_type!(self, func, 1, json, Json::String(Lit(x)) => x, "String(Literal)");
    match self.get_var(&var_name, scope){Some(var) => Ok(var), None => err!(self, json.pos, "Undefined variables: `{var_name}`")}
  }
  fn scope(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    let json = func.arg()?;
    let mut object =
      validate_type!(self, func, 1, json, Json::Object(Lit(x)) => x, "Object(Literal)");
    self.eval_object(&mut object, scope)
  }
}
