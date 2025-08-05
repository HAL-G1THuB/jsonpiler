use super::super::{
  ArgLen::SomeArg,
  Bind::{Lit, Var},
  ErrOR, FuncInfo, Json, Jsonpiler, ScopeInfo, err, mn, validate_type, warn,
};
use core::mem::take;
impl Jsonpiler {
  pub(crate) fn register_control(&mut self) {
    let sp_scope = (true, true);
    self.register("if", sp_scope, Jsonpiler::f_if, SomeArg);
    self.register("scope", sp_scope, Jsonpiler::scope, SomeArg);
  }
}
#[expect(clippy::single_call_fn, reason = "")]
impl Jsonpiler {
  fn f_if(&mut self, mut func: FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    let mut used_true = false;
    let if_end_label = self.get_label(8)?;
    for idx in 1..=func.len {
      let arg = func.arg()?;
      if used_true {
        warn!(
          self,
          &arg.pos,
          concat!(
            "Expressions in clauses following a clause ",
            "with a literal `true` condition are not evaluated at runtime, ",
            "but they are still present and parsed."
          )
        );
        break;
      }
      let cond_then_pair = &mut validate_type!(self, func, idx, arg, Json::Array(Lit(x)) => x, "Array[Bool, Any] (Literal)");
      if cond_then_pair.len() != 2 {
        return err!(
          self,
          arg.pos,
          "Each 'if' clause must have exactly two elements: a condition and a then expression."
        );
      }
      let mut cond_jwp = cond_then_pair.remove(0);
      let mut then_jwp = cond_then_pair.remove(0);
      cond_jwp.value = self.eval(take(&mut cond_jwp.value), scope)?;
      let Json::Bool(Var(cond_bool)) = cond_jwp.value else {
        let l_bool = validate_type!(self, func, idx, cond_jwp, Json::Bool(Lit(x)) => x, "Bool");
        if l_bool {
          then_jwp.value = self.eval(then_jwp.value, scope)?;
          scope.drop_json(then_jwp.value)?;
          used_true = true;
          scope.body.push(if_end_label.to_def());
          continue;
        }
        warn!(
          self,
          then_jwp.pos,
          "Expressions in clauses with a literal `false` condition are not evaluated at runtime, but they are still passed as arguments to the `if` function."
        );
        continue;
      };
      let next_clause_label = if idx == func.len { &if_end_label } else { &self.get_label(8)? };
      scope.free_if_tmp(&cond_bool)?;
      scope.body.push(mn!("mov", "al", cond_bool));
      scope.body.push(mn!("test", "al", "al"));
      scope.body.push(mn!("jz", next_clause_label.to_ref()));
      let then_result = self.eval(then_jwp.value, scope)?;
      scope.drop_json(then_result)?;
      scope.body.push(mn!("jmp", if_end_label.to_ref()));
      scope.body.push(next_clause_label.to_def());
    }
    Ok(Json::Null)
  }
  fn scope(&mut self, mut func: FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    for _ in 1..func.len {
      let val = self.eval(func.arg()?.value, scope)?;
      scope.drop_json(val)?;
    }
    self.eval(func.arg()?.value, scope)
  }
}
