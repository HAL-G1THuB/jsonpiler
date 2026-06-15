use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn eval(&mut self, json: Pos<Json>, scope: &mut Scope) -> ErrOR<Pos<Json>> {
    Ok(if let Array(_, Lit(array)) = json.val {
      json.pos.with(Array(ArrayType::new(None, None), Lit(self.eval_args(array, scope)?)))
    } else if let Object(Lit(object)) = json.val {
      self.eval_object(json.pos.with(object), scope)?
    } else {
      json
    })
  }
  fn eval_args(&mut self, mut args: Vec<Pos<Json>>, scope: &mut Scope) -> ErrOR<Vec<Pos<Json>>> {
    for arg in &mut args {
      *arg = self.eval(take(arg), scope)?;
    }
    Ok(args)
  }
  fn eval_func(&mut self, (name, args): KeyVal, scope: &mut Scope) -> ErrOR<Json> {
    if let Some(builtin) = self.builtin.get_mut(&name.val) {
      builtin.refs.push(name.pos);
      let BuiltInInfo { scoped, skip_eval, builtin_x64, builtin_a64, arity, .. } = builtin.clone();
      let builtin_ptr = if self.flags.a64 {
        let Some(builtin_ptr) = builtin_a64 else {
          return Err(A64NotImplemented(name.val.clone()).into());
        };
        builtin_ptr
      } else {
        builtin_x64
      };
      if scoped {
        scope.locals.push(BTreeMap::new());
      }
      let mut func = self.func_info((name, args), skip_eval, scope)?;
      func.validate_args(arity)?;
      let result = builtin_ptr(self, &mut func, scope)?;
      if scoped {
        self.drop_scope(scope)?;
      }
      self.free_tmp_list(&mut func, scope)?;
      return Ok(result);
    }
    let Some(u_d) = self.user_defined.get(&name.val).cloned() else {
      return err!(name.pos, UndefinedFunc(name.val.clone()));
    };
    self.use_func(scope.id, u_d.val.dep.id);
    self.use_u_d(scope.id, u_d.val.dep.id, name.pos)?;
    let ret_type = name.pos.with(u_d.val.sig.ret_type);
    let mut func = self.func_info((name, args), false, scope)?;
    let stack_args_count = if self.flags.a64 {
      let float_stack = u_d
        .val
        .sig
        .params
        .iter()
        .filter(|(_, json_type)| *json_type == FloatT)
        .count()
        .saturating_sub(A64_ARG_REGS.len());
      let int_stack =
        u_d.val.sig.params.len().saturating_sub(float_stack).saturating_sub(A64_ARG_REGS.len());
      float_stack + int_stack
    } else {
      u_d.val.sig.params.len().saturating_sub(X64_ARG_REGS.len())
    } as u32;
    scope.update_stack_args_count(stack_args_count);
    func.validate_args(Exact(len_u32(&u_d.val.sig.params)?))?;
    let ret_json = if self.flags.a64 {
      let mut a_args = vec![];
      for (_, param_type) in u_d.val.sig.params {
        let arg = func.arg()?;
        if arg.val.as_type() != param_type {
          return Err(func.args_err(vec![param_type], &arg));
        }
        a_args.push(arg);
      }
      self.store_args_a(a_args, true, scope)?;
      scope.p_a(Bl(u_d.val.dep.id))?;
      scope.ret_json_take_a(&ret_type, X9, X0)?
    } else {
      let mut x_args = vec![];
      for (_, param_type) in u_d.val.sig.params {
        let arg = func.arg()?;
        if arg.val.as_type() != param_type {
          return Err(func.args_err(vec![param_type], &arg));
        }
        x_args.push(arg);
      }
      self.store_args_x(x_args, true, scope)?;
      scope.p_x(Call(u_d.val.dep.id))?;
      scope.ret_json_take_x(&ret_type, Rax)?
    };
    self.free_tmp_list(&mut func, scope)?;
    Ok(ret_json)
  }
  fn eval_object(&mut self, object: Pos<Vec<KeyVal>>, scope: &mut Scope) -> ErrOR<Pos<Json>> {
    let mut tmp_json = object.pos.with(Null(Lit(())));
    for key_val in object.val {
      self.drop_json(&tmp_json.val, false, scope)?;
      tmp_json.val = self.eval_func(key_val, scope)?;
    }
    Ok(tmp_json)
  }
  pub(crate) fn free_tmp_list(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<()> {
    for mem in &take(&mut func.val.tmp_list) {
      self.heap_free(*mem, scope)?;
      if let Memory(Local(Tmp, start), mem_type) = mem {
        scope.free(*start, mem_type.size()?)?;
      }
    }
    Ok(())
  }
  pub(crate) fn func_info(
    &mut self,
    (Pos { val: name, pos }, val): KeyVal,
    skip_eval: bool,
    scope: &mut Scope,
  ) -> ErrOR<Pos<BuiltIn>> {
    let args_vec = if let Array(_, Lit(args)) = val.val { args } else { vec![val] };
    let args = if skip_eval { args_vec } else { self.eval_args(args_vec, scope)? };
    let mut func = pos.with(BuiltIn {
      len: len_u32(&args)?,
      name,
      args: vec![].into_iter(),
      tmp_list: BTreeSet::new(),
      nth: 0,
    });
    if !skip_eval {
      for arg in &args {
        func.push_free_tmp(arg.val.mem());
      }
    }
    func.val.args = args.into_iter();
    Ok(func)
  }
}
