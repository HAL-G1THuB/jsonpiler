use crate::{
  Arity::Exactly,
  Bind::{self, Lit, Var},
  ErrOR, FuncInfo,
  Inst::*,
  Json, Jsonpiler,
  OpQ::{Iq, Mq, Rq},
  Reg::*,
  ScopeInfo,
  VarKind::Global,
  built_in, err, take_arg,
};
impl Jsonpiler {
  #[expect(clippy::cast_sign_loss)]
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
      Json::String(Lit(st)) => {
        Json::String(Var(crate::Label { kind: Global { id: self.global_str(st) }, size: 8 }))
      }
      Json::String(Var(str_label)) => {
        let mem = if is_global { Global { id: self.get_bss_id(8) } } else { scope.local(8)?.kind };
        scope.push(LeaRM(Rax, str_label.kind));
        scope.push(MovQQ(Mq(mem), Rq(Rax)));
        Json::String(Var(crate::Label { kind: mem, size: 8 }))
      }
      Json::Null => Json::Null,
      Json::Int(Lit(int)) if is_global => {
        Json::Int(Var(crate::Label { kind: Global { id: self.global_num(int as u64) }, size: 8 }))
      }
      Json::Int(int) => {
        let mem = if is_global { Global { id: self.get_bss_id(8) } } else { scope.local(8)?.kind };
        scope.push(match int {
          Lit(l_int) => MovQQ(Rq(Rax), Iq(l_int as u64)),
          Var(int_label) => MovQQ(Rq(Rax), Mq(int_label.kind)),
        });
        scope.push(MovQQ(Mq(mem), Rq(Rax)));
        Json::Int(Var(crate::Label { kind: mem, size: 8 }))
      }
      Json::Bool(Lit(boolean)) if is_global => {
        Json::Bool(Var(crate::Label { kind: Global { id: self.global_bool(boolean) }, size: 1 }))
      }
      Json::Bool(boolean) => {
        let mem = if is_global { Global { id: self.get_bss_id(1) } } else { scope.local(1)?.kind };
        match boolean {
          Lit(l_bool) => scope.push(MovMbIb(mem, if l_bool { 0xFF } else { 0 })),
          Var(bool_label) => {
            scope.push(MovRbMb(Rax, bool_label.kind));
            scope.push(MovMbRb(mem, Rax));
          }
        }
        Json::Bool(Var(crate::Label { kind: mem, size: 1 }))
      }
      Json::Float(Lit(float)) if is_global => Json::Float(Var(crate::Label {
        kind: Global { id: self.global_num(float.to_bits()) },
        size: 8,
      })),
      Json::Float(float) => {
        let mem = if is_global { Global { id: self.get_bss_id(8) } } else { scope.local(8)?.kind };
        scope.push(match float {
          Bind::Lit(l_float) => MovQQ(Rq(Rax), Iq(l_float.to_bits())),
          Bind::Var(float_label) => MovQQ(Rq(Rax), Mq(float_label.kind)),
        });
        scope.push(MovQQ(Mq(mem), Rq(Rax)));
        Json::Int(Var(crate::Label { kind: mem, size: 8 }))
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
    if if is_global { &mut self.globals } else { scope.innermost_scope()? }
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
