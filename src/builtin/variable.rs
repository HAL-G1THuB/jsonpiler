use crate::{
  Arity::Exactly,
  Bind::{self, Lit, Var},
  ErrOR, FuncInfo,
  Inst::*,
  Json, Jsonpiler, Label,
  OpQ::{Iq, Mq, Rq},
  Reg::*,
  ScopeInfo,
  VarKind::Global,
  built_in, err, get_target_kind, take_arg,
};
use core::mem::discriminant;
impl Jsonpiler {
  #[expect(clippy::cast_sign_loss, clippy::too_many_lines)]
  fn assign(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo, is_global: bool) -> ErrOR<Json> {
    let (variable, pos) = take_arg!(self, func, 1, "String", Json::String(Lit(x)) => x);
    let json2 = func.arg()?;
    if is_global && self.globals.contains_key(&variable) {
      return err!(self, pos, "Reassignment is not possible in the global scope.");
    }
    let local_label = if is_global { None } else { scope.get_var_local(&variable) };
    if let Some(json) = &local_label {
      if discriminant(json) != discriminant(&json2.value) {
        return Err(
          self
            .parser
            .type_err(1, &format!("Variable `{variable}`"), &json.type_name(), &json2)
            .into(),
        );
      }
    }
    let value = match json2.value {
      Json::String(Lit(st)) if is_global => {
        Json::String(Var(Label { kind: Global { id: self.global_str(st) }, size: 8 }))
      }
      Json::String(Lit(st)) => {
        let kind = get_target_kind!(
          self, scope, is_global, 8, local_label,
          Json::String(Var(label )) =>label.kind);
        scope.push(LeaRM(Rax, Global { id: self.global_str(st) }));
        scope.push(MovQQ(Mq(kind), Rq(Rax)));
        Json::String(Var(Label { kind, size: 8 }))
      }
      Json::String(Var(string)) => {
        func.sched_free_tmp(&string);
        let kind = get_target_kind!(
          self, scope, is_global, 8, local_label,
          Json::String(Var(label )) =>label.kind);
        scope.push(LeaRM(Rax, string.kind));
        scope.push(MovQQ(Mq(kind), Rq(Rax)));
        Json::String(Var(Label { kind, size: 8 }))
      }
      Json::Null => Json::Null,
      Json::Int(Lit(int)) if is_global => {
        Json::Int(Var(Label { kind: Global { id: self.global_num(int as u64) }, size: 8 }))
      }
      Json::Int(int) => {
        if let Var(label) = int {
          func.sched_free_tmp(&label);
        }
        let kind = get_target_kind!(
          self, scope, is_global, 8, local_label,
          Json::Int(Var(label )) =>label.kind);
        scope.push(match int {
          Lit(l_int) => MovQQ(Rq(Rax), Iq(l_int as u64)),
          Var(int_label) => MovQQ(Rq(Rax), Mq(int_label.kind)),
        });
        scope.push(MovQQ(Mq(kind), Rq(Rax)));
        Json::Int(Var(Label { kind, size: 8 }))
      }
      Json::Bool(Lit(l_bool)) if is_global => {
        Json::Bool(Var(Label { kind: Global { id: self.global_bool(l_bool) }, size: 1 }))
      }
      Json::Bool(boolean) => {
        if let Var(label) = boolean {
          func.sched_free_tmp(&label);
        }
        let kind = get_target_kind!(
          self, scope, is_global, 1, local_label,
          Json::Bool(Var(label )) =>label.kind);
        match boolean {
          Lit(l_bool) => scope.push(MovMbIb(kind, if l_bool { 0xFF } else { 0 })),
          Var(bool_label) => {
            scope.push(MovRbMb(Rax, bool_label.kind));
            scope.push(MovMbRb(kind, Rax));
          }
        }
        Json::Bool(Var(Label { kind, size: 1 }))
      }
      Json::Float(Lit(l_float)) if is_global => {
        Json::Float(Var(Label { kind: Global { id: self.global_num(l_float.to_bits()) }, size: 8 }))
      }
      Json::Float(float) => {
        if let Var(label) = float {
          func.sched_free_tmp(&label);
        }
        let kind = get_target_kind!(
          self, scope, is_global, 8, local_label,
          Json::Float(Var(label )) =>label.kind);
        scope.push(match float {
          Bind::Lit(l_float) => MovQQ(Rq(Rax), Iq(l_float.to_bits())),
          Bind::Var(float_label) => MovQQ(Rq(Rax), Mq(float_label.kind)),
        });
        scope.push(MovQQ(Mq(kind), Rq(Rax)));
        Json::Float(Var(Label { kind, size: 8 }))
      }
      Json::Array(_) | Json::Object(_) => {
        return Err(
          self.parser.type_err(2, &func.name, "Types excluding arrays and objects", &json2).into(),
        );
      }
    };
    if is_global {
      self.globals.insert(variable, value);
    } else if local_label.is_none() {
      scope.innermost_scope()?.insert(variable, value);
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
