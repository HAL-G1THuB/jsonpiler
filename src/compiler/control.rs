use crate::prelude::*;
built_in! {self, func, scope, control;
  f_break => {"break", COMMON, Exact(0), {self.loop_control(false, func, scope) }},
  f_continue => {"continue", COMMON, Exact(0), { self.loop_control(true, func, scope) }},
  f_if => {"if", SPECIAL, AtLeast(1), {
    let if_expr_t = vec![CustomT("Array[Bool, Any] (Literal)".into())];
    let end = self.id();
    let mut arg = func.arg()?;
    let mut if_expr = if let Array(Lit(x)) = arg.val { arg.pos.with(x) } else {
      let cond = self.eval_with_scope(take(&mut arg), scope)?;
      let Bool(condition) = cond.val else {
        return Err(func.args_err(if_expr_t, cond.map_ref(Json::as_type)))
      };
      func.validate_args(Exact(2))?;
      scope.extend(&mov_bool(Rax, condition));
      scope.extend(&[LogicRbRb(Test, Rax, Rax), JCc(E, end)]);
      let memory_opt = match condition {
        Lit(reachable) => {
          self.warn(cond.pos, if reachable { UselessIfTrue } else { UnreachableIf })?;
          None
        },
        Var(memory)=> Some(memory)
      };
      self.if_expr(memory_opt, true, end, func.arg()?, func, scope)?;
      return Ok(Null(Lit(())))
    };
    let mut then_vec = vec![];
    for _ in 1..=func.val.len {
      if if_expr.val.len() != 2 {
        return Err(type_err(
          "`if` expression".into(),
          if_expr_t,
          if_expr.pos.with(ArrayT)
        ));
      }
      let mut cond = if_expr.val.remove(0);
      let then_label = self.id();
      cond = self.eval_with_scope(take(&mut cond), scope)?;
      let condition = unwrap_arg!(cond, "`if` condition", vec![BoolT], (Bool(x)) => x);
      let memory_opt = match condition.val {
        Lit(reachable) => {
          if reachable {
            if func.val.len == 1 {
              self.warn(condition.pos, UselessIfTrue)?;
            }
            if func.val.nth != func.val.len {
              self.warn(condition.pos, EarlyElse)?;
            }
          } else {
            self.warn(condition.pos, UnreachableIf)?;
          }
          None
        }
        Var(memory) => Some(memory)
      };
      then_vec.push((then_label, if_expr.val.remove(0), memory_opt));
      scope.extend(&mov_bool(Rax, condition.val));
      scope.extend(&[LogicRbRb(Test, Rax, Rax), JCc(Ne, then_label)]);
      if func.val.nth != func.val.len {
        if_expr = arg!(func, (Array(Lit(x))) => x);
      }
    }
    scope.push(Jmp(end));
    for (idx, (then_label, expr, memory_opt)) in then_vec.into_iter().enumerate() {
      scope.push(Lbl(then_label));
      self.if_expr(memory_opt, idx + 1 == func.val.len as usize, end, expr, func, scope)?;
    }
    Ok(Null(Lit(())))
  }},
  f_while => {"while", SP_SCOPE, Exact(2), {
    let mut cond = func.arg()?;
    let body = func.arg()?;
    let start = self.id();
    let end = self.id();
    scope.loop_labels.push((start, end, scope.locals.len()));
    scope.push(Lbl(start));
    cond = self.eval(take(&mut cond), scope)?;
    func.push_free_tmp(cond.val.memory());
    let condition = unwrap_arg!(cond, "`while` condition", vec![BoolT], (Bool(x)) => x);
    match condition.val {
      Lit(reachable) => {
        if !reachable {
          self.warn(condition.pos, UnreachableWhile)?;
        }
      }
      Var(memory) => {
        scope.extend(&mov_memory(Rax, memory));
        scope.extend(&[LogicRbRb(Test, Rax, Rax), JCc(E, end)]);
      }
    }
    let json = self.eval_with_scope(body, scope)?.val;
    self.drop_json(json, false, scope);
    self.free_all(func, scope);
    scope.extend(&[Jmp(start), Lbl(end)]);
    scope.loop_labels.pop();
    Ok(Null(Lit(())))
  }},
}
impl Jsonpiler {
  pub(crate) fn eval_with_scope(&mut self, expr: Pos<Json>, scope: &mut Scope) -> ErrOR<Pos<Json>> {
    scope.locals.push(BTreeMap::new());
    let value = self.eval(expr, scope)?;
    self.drop_scope(scope)?;
    Ok(value)
  }
  pub(crate) fn if_expr(
    &mut self,
    memory_opt: Option<Memory>,
    is_end: bool,
    end: u32,
    expr: Pos<Json>,
    func: &mut Pos<BuiltIn>,
    scope: &mut Scope,
  ) -> ErrOR<()> {
    func.push_free_tmp(memory_opt);
    let json = self.eval_with_scope(expr, scope)?.val;
    self.drop_json(json, false, scope);
    self.free_all(func, scope);
    scope.push(if is_end { Lbl(end) } else { Jmp(end) });
    Ok(())
  }
  pub(crate) fn loop_control(
    &mut self,
    is_continue: bool,
    func: &mut Pos<BuiltIn>,
    scope: &mut Scope,
  ) -> ErrOR<Json> {
    let Some(&(start, end, idx)) = scope.loop_labels.last() else {
      return err!(func.pos, OutSideError { name: func.val.name.clone(), place: "loop" });
    };
    for locals in scope.locals.get(idx..).unwrap_or_default().to_owned() {
      for local in locals.into_values() {
        if let Some(memory) = local.val.val.memory() {
          self.heap_free(memory, scope);
        }
      }
    }
    scope.push(Jmp(if is_continue { start } else { end }));
    Ok(Null(Lit(())))
  }
}
