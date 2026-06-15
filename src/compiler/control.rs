use crate::prelude::*;
built_in! {self, func, scope, control;
  {"break", COMMON, Exact(0),
    f_break => {self.loop_control(false, func, scope) },
    f_break_a => {self.loop_control(false, func, scope) }
  },
  {"continue", COMMON, Exact(0),
    f_continue => { self.loop_control(true, func, scope) },
    f_continue_a => { self.loop_control(true, func, scope) }
  },
  {"if", SPECIAL, AtLeast(1),
    f_if => { self.eval_if_branches(func, scope) },
    f_if_a => { self.eval_if_branches(func, scope) }
  },
  {"while", SP_SCOPE, Exact(2),
    f_while => { self.while_loop(func, scope) },
    f_while_a => { self.while_loop(func, scope) }
  }
}
impl Jsonpiler {
  fn check_multi_if(
    &mut self,
    bool: Pos<Bind<bool>>,
    found_true: &mut bool,
    func: &Pos<BuiltIn>,
  ) -> ErrOR<Option<Memory>> {
    match bool.val {
      Lit(reachable) => {
        if reachable {
          if func.val.len == 1 {
            self.warn(bool.pos.with(UselessIfTrue))?;
          }
          if func.val.nth != func.val.len {
            self.warn(bool.pos.with(EarlyElse))?;
          }
          *found_true = true;
        } else {
          self.warn(bool.pos.with(UnreachableIf))?;
        }
        Ok(None)
      }
      Var(mem) => Ok(Some(mem)),
    }
  }
  fn check_single_if(&mut self, bool: Pos<Bind<bool>>) -> ErrOR<Option<Memory>> {
    match bool.val {
      Lit(reachable) => {
        if reachable {
          self.warn(bool.pos.with(UselessIfTrue))?;
        } else {
          self.warn(bool.pos.with(UnreachableIf))?;
        }
        Ok(None)
      }
      Var(mem) => Ok(Some(mem)),
    }
  }
  fn eval_if_branches(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    let end = self.id();
    let mut found_true = false;
    let mut arg = func.arg()?;
    let mut if_expr = if let Array(_, Lit(x)) = arg.val {
      arg.pos.with(x)
    } else {
      let cond = self.eval_with_scope(take(&mut arg), scope)?;
      let condition = unwrap_arg!(cond, "`if` condition", vec![BoolT], (Bool(x)) => x);
      func.validate_args(Exact(2))?;
      let mem_opt = self.check_single_if(condition)?;
      let then_lbl = self.id();
      self.eval_if_cond(condition, then_lbl, scope)?;
      let then = (then_lbl, func.arg()?, mem_opt);
      let (result_type, result_json, _) = self.eval_if_expr(then, end, None, func, scope)?;
      scope.push_lbl(end);
      if result_type != NullT && !found_true {
        return err!(func.pos, IfNoTrueBranch);
      }
      return Ok(result_json);
    };
    let mut then_vec = vec![];
    for _ in 1..=func.val.len {
      if if_expr.val.len() != 2 {
        return err!(
          if_expr.pos,
          ArityErr {
            name: "`if` expression".into(),
            expected: Exact(2),
            actual: len_u32(&if_expr.val)?
          }
        );
      }
      let mut cond = if_expr.val.remove(0);
      cond = self.eval_with_scope(take(&mut cond), scope)?;
      let condition = unwrap_arg!(cond, "`if` condition", vec![BoolT], (Bool(x)) => x);
      let mem_opt = self.check_multi_if(condition, &mut found_true, func)?;
      let then_lbl = self.id();
      self.eval_if_cond(condition, then_lbl, scope)?;
      then_vec.push((then_lbl, if_expr.val.remove(0), mem_opt));
      if func.val.nth != func.val.len {
        if_expr = array_arg!(func, None, Some(2), (Lit(x)) => x);
      }
    }
    let Some(before) = then_vec
      .into_iter()
      .try_fold(None, |before, then| self.eval_if_expr(then, end, before, func, scope).map(Some))?
    else {
      return Err(ArgNotFound(func.val.name.clone(), 1).into());
    };
    scope.push_lbl(end);
    if before.0 != NullT && !found_true {
      return err!(func.pos, IfNoTrueBranch);
    }
    Ok(before.1)
  }
  fn eval_if_cond(
    &mut self,
    cond: Pos<Bind<bool>>,
    then_lbl: LabelId,
    scope: &mut Scope,
  ) -> ErrOR<()> {
    if self.flags.a64 {
      scope.ee_a(vec![load_bool_a(X0, cond.val)?, vec![TstRb(X0)]])
    } else {
      scope.ee_x(vec![load_bool_x(Rax, cond.val)?, vec![TestRR(S1, Rax)]])
    }?;
    scope.push_jcc(Ne, then_lbl);
    Ok(())
  }
  fn eval_if_expr(
    &mut self,
    (then_lbl, expr, mem_opt): (LabelId, Pos<Json>, Option<Memory>),
    end: LabelId,
    before: Option<(JsonType, Json, Option<Memory>)>,
    func: &mut Pos<BuiltIn>,
    scope: &mut Scope,
  ) -> ErrOR<(JsonType, Json, Option<Memory>)> {
    func.push_free_tmp(mem_opt);
    scope.push_jmp(end);
    scope.push_lbl(then_lbl);
    scope.locals.push(BTreeMap::new());
    let json = self.eval(expr, scope)?;
    let json_type = json.val.as_type();
    let (result_type, result, result_mem_opt) = match before {
      Some((expected, e_json, e_mem)) => {
        if expected != json_type {
          let actual = json.map_ref(Json::as_type);
          return Err(type_err(fmt_ret_val("if"), vec![expected], actual));
        }
        (expected, e_json, e_mem)
      }
      None if json_type == NullT => (NullT, Null(Lit(())), None),
      None => {
        let mem_type = json_type.mem_type(json.pos)?;
        let size = mem_type.reg_size();
        let mem = Memory(scope.alloc_n(size)?.0, mem_type);
        let result = json_type.to_json(json.pos, mem.0)?;
        (json_type, result, Some(mem))
      }
    };
    let Some(mem) = result_mem_opt else {
      self.drop_json(&json.val, false, scope)?;
      self.drop_scope(scope)?;
      return Ok((result_type, result, None));
    };
    if self.flags.a64 {
      scope.ee_a(vec![self.load_json_a(X0, &json, Some(scope.id))?, store_a(mem, X1, X0)?])?;
    } else {
      scope.ee_x(vec![self.load_json_x(Rax, &json, Some(scope.id))?, store_x(mem, Rcx, Rax)?])?;
    }
    self.drop_json(&json.val, false, scope)?;
    self.drop_scope(scope)?;
    Ok((result_type, result, Some(mem)))
  }
  fn eval_with_scope(&mut self, expr: Pos<Json>, scope: &mut Scope) -> ErrOR<Pos<Json>> {
    scope.locals.push(BTreeMap::new());
    let value = self.eval(expr, scope)?;
    self.drop_scope(scope)?;
    Ok(value)
  }
  fn loop_control(
    &mut self,
    is_continue: bool,
    func: &mut Pos<BuiltIn>,
    scope: &mut Scope,
  ) -> ErrOR<Json> {
    let Some(&(start, end, idx)) = scope.loop_labels.last() else {
      return err!(func.pos, OutSideErr { name: func.val.name.clone(), place: "loop" });
    };
    let mems: Vec<_> = scope.locals[idx..]
      .iter()
      .flat_map(BTreeMap::values)
      .filter_map(|local| local.val.val.mem())
      .collect();
    for mem in mems {
      self.heap_free(mem, scope)?;
    }
    let lbl = if is_continue { start } else { end };
    scope.push_jmp(lbl);
    Ok(Null(Lit(())))
  }
  fn while_loop(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    let mut arg = func.arg()?;
    let start = self.id();
    let end = self.id();
    scope.loop_labels.push((start, end, scope.locals.len()));
    scope.push_lbl(start);
    arg = self.eval(take(&mut arg), scope)?;
    func.push_free_tmp(arg.val.mem());
    let cond = unwrap_arg!(arg, "`while` condition", vec![BoolT], (Bool(x)) => x);
    match cond.val {
      Lit(reachable) => {
        if !reachable {
          self.warn(cond.pos.with(UnreachableWhile))?;
        }
      }
      Var(mem) => {
        if self.flags.a64 {
          scope.ee_a(vec![load_a(X0, mem)?, vec![TstRb(X0)]])?;
        } else {
          scope.ee_x(vec![load_x(Rax, mem)?, vec![TestRR(S1, Rax)]])?;
        }
        scope.push_jcc(E, end);
      }
    }
    let json = self.eval_with_scope(func.arg()?, scope)?.val;
    self.drop_json(&json, false, scope)?;
    scope.push_jmp(start);
    scope.push_lbl(end);
    scope.loop_labels.pop();
    Ok(Null(Lit(())))
  }
}
