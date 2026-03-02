use crate::prelude::*;
built_in! {self, func, scope, control;
  f_break => {"break", COMMON, Exactly(0), {
    let Some(&(_, end)) = scope.loop_labels.last() else {
      return err!(func.pos, OutSideError { kind: func.name.clone(), place: "loop" });
    };
    scope.push(Jmp(end));
    Ok(Null)
  }},
  f_continue => {"continue", COMMON, Exactly(0), {
    let Some(&(start, _)) = scope.loop_labels.last() else {
      return err!(func.pos, OutSideError { kind: func.name.clone(), place: "loop" });
    };
    scope.push(Jmp(start));
    Ok(Null)
  }},
  f_if => {"if", SP_SCOPE, AtLeast(1), {
    const IF_CLAUSE: &str = "Array[Bool, Any] (Literal)";
    let mut used_true = false;
    let end = self.id();
    for nth in 1..=func.len {
      let mut cond_then = arg_custom!(self, func, IF_CLAUSE, (Array(Lit(x))) => x);
      if used_true {
        warn!(self, cond_then.pos, "Blocks in subsequent `if` clauses are not evaluated");
        scope.push(Lbl(end));
        break;
      }
      if cond_then.val.len() != 2 {
        let if_clause = cond_then.pos.with(Array(Lit(cond_then.val)));
        return Err(type_err("Each `if` clause".into(), IF_CLAUSE.into(), &if_clause));
      }
      let mut cond = cond_then.val.remove(0);
      let then = cond_then.val.remove(0);
      cond = cond.pos.with(self.eval(take(&mut cond), scope)?);
      let object = unwrap_arg!(self, then, func, "Block", (Object(Lit(x))) => x).val;
      match unwrap_arg!(self, cond, func, "Bool", (Bool(x)) => x).val {
        Lit(lit) => {
          if lit {
            self.eval_object_with_drop(object, scope)?;
            used_true = true;
          } else {
            warn!(self, then.pos, "This block is passed to `if` but not evaluated");
          }
          if nth == func.len { scope.push(Lbl(end)) }
        }
        Var(label) => {
          func.push_free_tmp(label);
          let next = if nth == func.len { end } else { self.id() };
          scope.extend(&mov_label(Rax, label, 1, false));
          scope.extend(&[LogicRbRb(Test, Rax, Rax), JCc(E, next)]);
          self.eval_object_with_drop(object, scope)?;
          if nth != func.len { scope.push(Jmp(end)) }
          scope.push(Lbl(next));
        }
      }
    }
    Ok(Null)
  }},
  f_while => {"while", SP_SCOPE, Exactly(2), {
    let mut cond = func.arg()?;
    let body = arg_custom!(self, func, "Block", (Object(Lit(x))) => x);
    let while_start = self.id();
    let end   = self.id();
    scope.loop_labels.push((while_start, end));
    scope.push(Lbl(while_start));
    cond = WithPos { val: self.eval(take(&mut cond), scope)?, ..cond };
    let cond_bool = unwrap_arg!(self, cond, func, "Bool", (Bool(x)) => x).val;
    match cond_bool {
      Lit(lit) => {
        if lit {
          self.eval_object_with_drop(body.val, scope)?;
          scope.push(Jmp(while_start));
        } else {
          warn!(self, cond.pos, "While condition is a `false`, loop body is unreachable.");
        }
      }
      Var(label) => {
        func.push_free_tmp(label);
        scope.extend(&mov_label(Rax, label, 1, false));
        scope.extend(&[LogicRbRb(Test, Rax, Rax), JCc(E, end)]);
        self.eval_object_with_drop(body.val, scope)?;
        scope.push(Jmp(while_start));
      }
    }
    scope.push(Lbl(end));
    scope.loop_labels.pop();
    Ok(Null)
  }},
}
