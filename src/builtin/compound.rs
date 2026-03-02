use crate::prelude::*;
built_in! {self, func, scope, compound;
  assign_add => {"+=", COMMON, Exactly(2), {
    self.assign_normal(func, scope, None, AddRR(Rax, Rcx), AddSd(Rax, Rcx))
  }},
  assign_div => {"/=", COMMON, Exactly(2), {
    self.assign_normal(func, scope, Some(&Jsonpiler::check_zero_cqo), IDivR(Rcx), DivSd(Rax, Rcx))
  }},
  assign_mul => {"*=", COMMON, Exactly(2), {
    self.assign_normal(func, scope, None, IMulRR(Rax, Rcx), MulSd(Rax, Rcx))
  }},
  assign_sub => {"-=", COMMON, Exactly(2), {
    self.assign_normal(func, scope, None, SubRR(Rax, Rcx), SubSd(Rax, Rcx))
  }}
}
type CheckFn = dyn Fn(&mut Jsonpiler, Position) -> ErrOR<Vec<Inst>>;
impl Jsonpiler {
  fn assign_normal(
    &mut self,
    func: &mut Function,
    scope: &mut Scope,
    check_opt: Option<&CheckFn>,
    int_inst: Inst,
    float_inst: Inst,
  ) -> ErrOR<Json> {
    let WithPos { val: var_name, pos } = arg!(self, func, (Str(Lit(x))) => x);
    let mut var = self.get_var(&var_name, pos, scope)?;
    let label = *or_err!((var.label()), pos, UndefinedVar(var_name))?;
    let arg = func.arg()?;
    if let Int(int) = arg.val {
      if !matches!(var, Int(_)) {
        let name = Int(Var(Label::default())).describe();
        return Err(type_err(format!("Variable `{var}`"), name, &pos.with(var)));
      }
      extend!(scope.body, mov_label(Rax, label, 8, false), &mov_int(Rcx, int));
      if let Some(check) = check_opt {
        scope.extend(&check(self, pos)?);
      }
      scope.push(int_inst);
      scope.extend(&ret_label(label, Rcx, Rax, 8, false));
    } else if let Float(float) = arg.val {
      if !matches!(var, Float(_)) {
        let name = Float(Var(Label::default())).describe();
        return Err(type_err(format!("Variable `{var}`"), name, &pos.with(var)));
      }
      extend!(
        scope.body,
        mov_label_xmm(Rax, Rax, label),
        self.mov_float_xmm(Rcx, Rax, float),
        [float_inst],
        ret_label_xmm(label, Rax, Rax)
      );
    } else {
      return Err(args_type_err(2, &func.name, "Int` or `Float".into(), &arg));
    }
    Ok(Null)
  }
}
