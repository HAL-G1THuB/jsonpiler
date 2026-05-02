use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn eval(&mut self, json: Pos<Json>, scope: &mut Scope) -> ErrOR<Pos<Json>> {
    Ok(if let Array(Lit(array)) = json.val {
      json.pos.with(Array(Lit(self.eval_args(array, scope)?)))
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
    if let Some(builtin) = self.builtin.get(&name.val.as_ref()) {
      let BuiltInInfo { scoped, skip_eval, builtin_ptr, arity } = *builtin;
      if let Some(symbol) = self.analysis.as_mut().and_then(|analysis| {
        analysis
          .symbols
          .iter_mut()
          .find(|symbol| symbol.name == name.val && symbol.kind == BuiltInFunc)
      }) {
        symbol.refs.push(name.pos);
      }
      if scoped {
        scope.locals.push(BTreeMap::new());
      }
      let mut func = self.func_info((name, args), skip_eval, scope)?;
      func.validate_args(arity)?;
      let result = builtin_ptr(self, &mut func, scope)?;
      if scoped {
        self.drop_scope(scope)?;
      }
      self.free_all(&mut func, scope);
      return Ok(result);
    }
    let Some(u_d) = self.user_defined.get_mut(&name.val) else {
      return err!(name.pos, UndefinedFunc(name.val.clone()));
    };
    u_d.val.refs.push(name.pos);
    let UserDefinedInfo { dep, sig, .. } = u_d.val.clone();
    self.use_function(scope.id, dep.id);
    self.use_u_d(scope.id, dep.id)?;
    let ret = name.pos.with(sig.ret_type);
    let mut func = self.func_info((name, args), false, scope)?;
    let params_len = len_u32(&sig.params)?;
    scope.update_args_count(params_len);
    func.validate_args(Exact(params_len))?;
    for (_, param_type) in sig.params {
      let arg = func.arg()?;
      if arg.val.as_type() != param_type {
        return Err(func.args_err(vec![param_type], arg.map_ref(Json::as_type)));
      }
      self.mov_args_json(func.val.nth - 1, arg, true, scope)?;
    }
    scope.push(Call(dep.id));
    let ret_json = scope.ret_json_take(&ret, Rax)?;
    self.free_all(&mut func, scope);
    Ok(ret_json)
  }
  fn eval_object(&mut self, object: Pos<Vec<KeyVal>>, scope: &mut Scope) -> ErrOR<Pos<Json>> {
    let mut tmp_json = object.pos.with(Null(Lit(())));
    for key_val in object.val {
      self.drop_json(tmp_json.val, false, scope);
      tmp_json.val = self.eval_func(key_val, scope)?;
    }
    Ok(tmp_json)
  }
  pub(crate) fn free_all(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) {
    for memory in &take(&mut func.val.free_list) {
      self.heap_free(*memory, scope);
      if let Memory(Local(Tmp, start), mem_type) = memory {
        scope.free(*start, *mem_type);
      }
    }
  }
  pub(crate) fn func_info(
    &mut self,
    (Pos { val: name, pos }, val): KeyVal,
    skip_eval: bool,
    scope: &mut Scope,
  ) -> ErrOR<Pos<BuiltIn>> {
    let args_vec = if let Array(Lit(args)) = val.val { args } else { vec![val] };
    let args = if skip_eval { args_vec } else { self.eval_args(args_vec, scope)? };
    let mut func = pos.with(BuiltIn {
      len: len_u32(&args)?,
      name,
      args: vec![].into_iter(),
      free_list: BTreeSet::new(),
      nth: 0,
    });
    if !skip_eval {
      for arg in &args {
        func.push_free_tmp(arg.val.memory());
      }
    }
    func.val.args = args.into_iter();
    Ok(func)
  }
}
