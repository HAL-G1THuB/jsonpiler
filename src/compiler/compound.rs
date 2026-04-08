use crate::prelude::*;
built_in! {self, func, scope, compound;
  assign_add => {"+=", SPECIAL, Exact(2), {
    self.assign_normal(func, scope, None, AddRR(Rax, Rcx), Add)
  }},
  assign_div => {"/=", SPECIAL, Exact(2), {
    self.assign_normal(func, scope, Some(&Jsonpiler::check_zero_cqo), IDivR(Rcx), Div)
  }},
  assign_mul => {"*=", SPECIAL, Exact(2), {
    self.assign_normal(func, scope, None, IMulRR(Rax, Rcx), Mul)
  }},
  assign_sub => {"-=", SPECIAL, Exact(2), {
    self.assign_normal(func, scope, None, SubRR(Rax, Rcx), Sub)
  }}
}
type CheckFn = dyn Fn(&mut Jsonpiler, Position, LabelId) -> ErrOR<Vec<Inst>>;
impl Jsonpiler {
  fn assign_normal(
    &mut self,
    func: &mut BuiltIn,
    scope: &mut Scope,
    check_opt: Option<&CheckFn>,
    int_inst: Inst,
    float_inst: ArithSdKind,
  ) -> ErrOR<Json> {
    let var = func.arg()?.into_ident("Variable name")?;
    let val = self.get_var(&var, scope)?;
    let Some(memory) = val.memory() else {
      return err!(var.pos, UndefinedVar(var.val));
    };
    let num = self.eval(func.arg()?, scope)?;
    match num {
      WithPos { val: Int(int), .. } => {
        if val.as_type() != IntT {
          return Err(type_err(
            format!("Variable `{}`", var.val),
            vec![IntT],
            var.pos.with(val.as_type()),
          ));
        }
        scope.extend(&mov_memory(Rax, memory));
        scope.extend(&mov_int(Rcx, int));
        if let Some(check) = check_opt {
          scope.extend(&check(self, var.pos, scope.id)?);
        }
        scope.push(int_inst);
        if !self.release {
          scope.extend(&[
            LogicRR(Test, Rax, Rax),
            JCc(O, self.custom_err(RuntimeOverflow, None, var.pos, scope.id)?),
          ]);
        }
        scope.extend(&ret_memory(memory, Rcx, Rax));
      }
      WithPos { val: Float(float), .. } => {
        if val.as_type() != FloatT {
          return Err(type_err(
            format!("Variable `{}`", var.val),
            vec![FloatT],
            var.pos.with(val.as_type()),
          ));
        }
        scope.extend(&mov_memory_xmm(Rax, Rax, memory));
        scope.extend(&self.mov_float_xmm(Rcx, Rax, float));
        scope.push(ArithSd(float_inst, Rax, Rcx));
        scope.extend(&ret_memory_xmm(memory, Rax, Rax));
      }
      other => {
        return Err(args_type_err(2, &func.name, vec![IntT, BoolT], other.map_ref(Json::as_type)));
      }
    }
    self.drop_json(num.val, scope, false);
    Ok(Null(Lit(())))
  }
}
