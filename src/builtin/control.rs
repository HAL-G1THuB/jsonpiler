use super::super::{
  ArgLen::{Exactly, SomeArg},AsmFunc,
  Bind::{Lit, Var},
  ErrOR, FuncInfo, Json, Jsonpiler, ScopeInfo, err, mn, validate_type, warn,
  utility::get_int_str_without_free
};
use core::mem::{replace, take};
use std::collections::HashMap;
impl Jsonpiler {
  pub(crate) fn control(&mut self) {
    let special = (false, true);
    let sp_scope = (true, true);
    self.register("lambda", special, Jsonpiler::lambda, Exactly(2));
    self.register("if", sp_scope, Jsonpiler::f_if, SomeArg);
  }
}
#[expect(clippy::single_call_fn, reason = "")]
impl Jsonpiler {
  fn f_if(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    let mut used_true = false;
    let if_end_label = self.ctx.label(8)?;
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
      let then_object = &mut validate_type!(self, func, idx, then_jwp, Json::Object(Lit(x)) => x, "Object (Literal)");
      cond_jwp.value = self.eval(take(&mut cond_jwp.value), scope)?;
      let Json::Bool(Var(cond_bool)) = cond_jwp.value else {
        let l_bool = validate_type!(self, func, idx, cond_jwp, Json::Bool(Lit(x)) => x, "Bool");
        if l_bool {
          then_jwp.value = self.eval_object(then_object, scope)?;
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
      let next_clause_label = if idx == func.len { &if_end_label } else { &self.ctx.label(8)? };
      func.sched_free_tmp(&cond_bool);
      scope.body.push(mn!("mov", "al", cond_bool));
      scope.body.push(mn!("test", "al", "al"));
      scope.body.push(mn!("jz", next_clause_label.to_ref()));
      let then_result = self.eval_object(then_object, scope)?;
      scope.drop_json(then_result)?;
      scope.body.push(mn!("jmp", if_end_label.to_ref()));
      scope.body.push(next_clause_label.to_def());
    }
    Ok(Json::Null)
  }
  fn lambda(&mut self,func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    let mut tmp_local_scope = replace(&mut scope.locals, vec![HashMap::new()]);
    let mut inner_scope = ScopeInfo::new();
    let json1 = func.arg()?;
    let params = validate_type!(self, func, 1, json1, Json::Array(Lit(x)) => x, "Array (Literal)");
    if !params.is_empty() {
      return err!(self, &json1.pos, "PARAMETERS HAS BEEN NOT IMPLEMENTED.");
    }
    let json2 = func.arg()?;
    let object = &mut validate_type!(self, func, 2, json2, Json::Object(Lit(x)) => x, "Object (Literal)");

    let ret = Box::new(self.eval_object(object, &mut inner_scope)?);
    let label = self.ctx.label(8)?;
    self.text.push(mn!(".seh_proc", label.to_ref()));
    self.text.push(label.to_def());
    let size = format!("{:#x}", inner_scope.calc_alloc((inner_scope.reg_used.len() & 1) << 3)?);
    let mut registers = inner_scope.reg_used.into_iter().collect::<Vec<String>>();
    registers.sort();
    for reg in &registers {
      self.text.push(mn!("push", reg));
      self.text.push(mn!(".seh_pushreg", reg));
    }
    self.text.push(mn!("push", "rbp"));
    self.text.push(mn!(".seh_pushreg", "rbp"));
    self.text.push(format!(include_str!("../asm/common/prologue.s"), size = size));
    for body in inner_scope.body {
      self.text.push(body);
    }
    if let Json::Int(int) = &*ret {
      let int_str = get_int_str_without_free(int);
      self.text.push(mn!("mov", "rax", int_str));
    } else {
      self.text.push(mn!("xor", "eax", "eax"));
    }
    self.text.push(mn!("mov", "rsp", "rbp"));
    self.text.push(mn!("pop", "rbp"));
    registers.reverse();
    for reg in registers {
      self.text.push(mn!("pop", reg));
    }
    self.text.push(mn!("ret"));
    self.text.push(mn!(".seh_endproc"));
    scope.locals = take(&mut tmp_local_scope);
    Ok(Json::Function(AsmFunc { label, params, ret }))
  }
}
