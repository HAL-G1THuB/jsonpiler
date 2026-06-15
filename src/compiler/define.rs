use crate::prelude::*;
built_in! {self, func, scope, define;
  {"define", SPECIAL, Exact(4),
    f_define => { self.define_func(func, scope) },
    f_define_a => { self.define_func(func, scope) }
  },
  {"ret", COMMON, Exact(1),
    ret => { self.ret_func(func, scope) },
    ret_a => { self.ret_func(func, scope) }
  }
}
impl Jsonpiler {
  fn define_func(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    let id = self.id();
    let old_scope = scope.change(id);
    let name = func.arg()?.into_ident("Function name")?;
    self.check_defined(&name, name.pos, UserDefinedFunc, scope)?;
    let type_annotations = arg_custom!(
      func, vec![CustomT("TypeAnnotations".into())], (Object(Lit(x))) => x
    );
    let mut params = vec![];
    let mut args = vec![];
    let mut names = vec![];
    for (var_name, param) in type_annotations.val {
      let param_type_str = param.into_ident("Type annotation")?;
      let json_type = JsonType::from_str(&param_type_str.val);
      let mem_type = json_type.mem_type(param_type_str.pos)?;
      let size = mem_type.size()?;
      let arg = Local(Long, scope.alloc(size, size)?);
      let json = json_type.to_json(param_type_str.pos, arg)?;
      args.push((Memory(arg, mem_type), json_type == FloatT));
      params.push((var_name.val.clone(), json_type));
      names.push((var_name, json));
    }
    let stack_args_count = if self.flags.a64 {
      let float_stack = params
        .iter()
        .filter(|(_, json_type)| *json_type == FloatT)
        .count()
        .saturating_sub(A64_ARG_REGS.len());
      let int_stack = params.len().saturating_sub(float_stack).saturating_sub(A64_ARG_REGS.len());
      float_stack + int_stack
    } else {
      params.len().saturating_sub(X64_ARG_REGS.len())
    } as u32;
    scope.update_stack_args_count(stack_args_count);
    let ret_type = JsonType::from_str(&func.arg()?.into_ident("Type annotation")?.val);
    let epilogue = self.id();
    scope.epilogue = Some((epilogue, ret_type.clone()));
    let u_d_info = UserDefinedInfo::new(id, params, ret_type.clone());
    self.user_defined.insert(name.val.clone(), name.pos.with(u_d_info));
    for (var_name, json) in names {
      self.check_defined(&var_name, var_name.pos, Argument, scope)?;
      scope.innermost().insert(var_name.val, var_name.pos.with(Variable::new(json, Argument)));
    }
    let ret = self.eval(func.arg()?, scope)?;
    if ret_type != ret.val.as_type() {
      let actual = ret.map_ref(Json::as_type);
      return Err(type_err(fmt_ret_val(&name.val), vec![ret_type], actual));
    }
    let tmp = scope.alloc(8, 8)?;
    if self.flags.a64 {
      scope.e_a(self.load_json_a(X0, &ret, Some(scope.id))?)?;
      scope.e_a(store_a(Local(Tmp, tmp).v_rq(), X1, X0)?)?;
    } else {
      scope.e_x(self.load_json_x(Rax, &ret, Some(scope.id))?)?;
      scope.p_x(store(S8, Local(Tmp, tmp), Rax))?;
    }
    self.drop_json(&ret.val, false, scope)?;
    self.drop_all_local(scope)?;
    if self.flags.a64 {
      scope.e_a(load_a(X0, Local(Tmp, tmp).v_rq())?)?;
    } else {
      scope.p_x(load(S8, Rax, Local(Tmp, tmp)))?;
    }
    scope.push_lbl(epilogue);
    scope.free(tmp, 8)?;
    scope.check_free()?;
    if self.flags.a64 {
      let stack_size = scope.resolve_a64_stack_size()?;
      let mut insts = vec![];
      let mut float_idx = 0;
      for (idx, (mem, is_float)) in args.into_iter().enumerate() {
        insts.push(self.load_args_a(u32::try_from(idx)?, &mut float_idx, mem, is_float)?);
      }
      insts.extend(scope.replace(old_scope).a64()?);
      self.link_func_a(id, insts, stack_size);
    } else {
      let stack_size = scope.resolve_x64_stack_size()?;
      let mut insts = vec![];
      for (idx, (mem, is_float)) in args.into_iter().enumerate() {
        insts.push(self.load_args_x(u32::try_from(idx)?, mem, is_float)?);
      }
      insts.extend(scope.replace(old_scope).x64()?);
      self.link_func_x(id, insts, stack_size);
    }
    Ok(Null(Lit(())))
  }
  fn ret_func(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    let ret = func.arg()?;
    let Some((epilogue, ret_type)) = scope.epilogue.as_ref() else {
      return err!(ret.pos, OutSideErr { name: func.val.name.clone(), place: "function" });
    };
    let epi = *epilogue;
    if *ret_type != ret.val.as_type() {
      let actual = ret.map_ref(Json::as_type);
      return Err(type_err(fmt_ret_val(&func.val.name), vec![ret_type.clone()], actual));
    }
    let mems: Vec<_> = scope
      .iter_locals()
      .flat_map(|locals| locals.values())
      .filter_map(|local| local.val.val.mem())
      .collect();
    for mem in mems {
      if Some(mem) != ret.val.mem() {
        self.heap_free(mem, scope)?;
      }
    }
    if self.flags.a64 {
      scope.e_a(self.load_json_a(X0, &ret, Some(scope.id))?)?;
    } else {
      scope.e_x(self.load_json_x(Rax, &ret, Some(scope.id))?)?;
    }
    if let Some(mem) = ret.val.mem()
      && mem.1.pass_by == HeapPtr
    {
      let tmp = scope.alloc(8, 8)?;
      let tmp_mem = Local(Tmp, tmp).v_rq();
      if self.flags.a64 {
        scope.ee_a(vec![
          store_a(tmp_mem, X1, X0)?,
          self.heap_free_a(mem)?,
          load_a(X0, tmp_mem)?,
        ])?;
      } else {
        scope.ee_x(vec![
          store_x(tmp_mem, Rcx, Rax)?,
          self.heap_free_x(mem)?,
          load_x(Rax, tmp_mem)?,
        ])?;
      }
      scope.free(tmp, 8)?;
    }
    scope.push_jmp(epi);
    Ok(Null(Lit(())))
  }
}
