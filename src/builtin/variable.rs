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
    let variable = take_arg!(self, func, "String", Json::String(Lit(x)) => x);
    let json2 = func.arg()?;
    let critical_section = Global { id: self.sym_table["CRITICAL_SECTION"], disp: 0i32 };
    if is_global {
      let enter_c_s = self.import(Jsonpiler::KERNEL32, "EnterCriticalSection", 0x138);
      scope.push(LeaRM(Rcx, critical_section));
      scope.push(CallApi(enter_c_s));
    }
    let ref_label = if is_global {
      self.globals.get(&variable.value).cloned()
    } else {
      scope.get_var_local(&variable.value)
    };
    if let Some(json) = &ref_label {
      if discriminant(json) != discriminant(&json2.value) {
        return Err(
          self.parser[json2.pos.file]
            .args_type_error(
              1,
              &format!("Variable `{}`", variable.value),
              &json.type_name(),
              &json2,
            )
            .into(),
        );
      }
    }
    let value = match json2.value {
      Json::String(string) => {
        let kind = get_target_kind!(
          self, scope, is_global, 8, ref_label,
          Json::String(Var(label)) => label.kind
        );
        scope.push(match string {
          Lit(l_str) => {
            let id = self.global_str(l_str);
            LeaRM(Rax, Global { id, disp: 0i32 })
          }
          Var(str_label) => MovQQ(Rq(Rax), Mq(str_label.kind)),
        });
        scope.push(MovQQ(Mq(kind), Rq(Rax)));
        Json::String(Var(Label { kind, size: 8 }))
      }
      Json::Null => Json::Null,
      Json::Int(Lit(int)) if is_global => Json::Int(Var(Label {
        kind: Global { id: self.global_num(int as u64), disp: 0i32 },
        size: 8,
      })),
      Json::Int(int) => {
        let kind = get_target_kind!(
          self, scope, is_global, 8,ref_label,
          Json::Int(Var(label )) => label.kind
        );
        scope.push(MovQQ(
          Rq(Rax),
          match int {
            Lit(l_int) => Iq(l_int as u64),
            Var(int_label) => Mq(int_label.kind),
          },
        ));
        scope.push(MovQQ(Mq(kind), Rq(Rax)));
        Json::Int(Var(Label { kind, size: 8 }))
      }
      Json::Bool(Lit(l_bool)) if is_global => Json::Bool(Var(Label {
        kind: Global { id: self.global_bool(l_bool), disp: 0i32 },
        size: 1,
      })),
      Json::Bool(boolean) => {
        let kind = get_target_kind!(
          self, scope, is_global, 1, ref_label,
          Json::Bool(Var(label )) => label.kind);
        match boolean {
          Lit(l_bool) => scope.push(MovMbIb(kind, if l_bool { 0xFF } else { 0 })),
          Var(bool_label) => {
            scope.push(MovRbMb(Rax, bool_label.kind));
            scope.push(MovMbRb(kind, Rax));
          }
        }
        Json::Bool(Var(Label { kind, size: 1 }))
      }
      Json::Float(Lit(l_float)) if is_global => Json::Float(Var(Label {
        kind: Global { id: self.global_num(l_float.to_bits()), disp: 0i32 },
        size: 8,
      })),
      Json::Float(float) => {
        let kind = get_target_kind!(
          self, scope, is_global, 8, ref_label,
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
          self.parser[json2.pos.file]
            .args_type_error(2, &func.name, "Types excluding arrays and objects", &json2)
            .into(),
        );
      }
    };
    if is_global {
      let leave_c_s = self.import(Jsonpiler::KERNEL32, "LeaveCriticalSection", 0x3C6);
      scope.push(LeaRM(Rcx, critical_section));
      scope.push(CallApi(leave_c_s));
      self.globals.insert(variable.value, value);
    } else if ref_label.is_none() {
      scope.innermost_scope()?.insert(variable.value, value);
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
    let var_name = take_arg!(self, func, "String (Literal)", Json::String(Lit(x)) => x);
    match self.get_var(&var_name.value, scope) {
      Some(var) => Ok(var),
      None => err!(self, var_name.pos, "Undefined variables: `{}`", var_name.value),
    }
  }},
  scope => {"scope", SP_SCOPE, Exactly(1), {
    let object = take_arg!(self, func, "Sequence", Json::Object(Lit(x)) => x);
    self.eval_object(object.value, object.pos, scope)
  }}
}
