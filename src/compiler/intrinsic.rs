use crate::prelude::*;
built_in! {self, func, _scope, intrinsic;
  {"__alloc", COMMON, Exact(1), __alloc => {
    _scope.ee_x(vec![load_int_x(R8, arg!(func, (Int(x)) => x).val)?, self.call_alloc_r8_x()?])?;
    Ok(Int(Var(_scope.ret_x(S8, Rax)?)))
  }},
  {"__free", COMMON, Exact(1), __free => {
    _scope.ee_x(vec![load_int_x(R8, arg!(func, (Int(x)) => x).val)?, self.call_free_r8_x()?])?;
    Ok(Null(Lit(())))
  }},
  {"__load", SPECIAL, Exact(2), __load => {
    let arg = self.eval(func.arg()?, _scope)?;
    let src = unwrap_arg!(arg, "SRC", vec![IntT], (Int(x)) => x).val;
    let json_type = func.arg()?.into_ident("TYPE")?.map(|val: String| JsonType::from_str(val.as_ref()));
    let mem_type = json_type.val.mem_type(json_type.pos)?;
    let size = mem_type.size()?;
    let addr = Local(Tmp, _scope.alloc(size, size)?);
    _scope.ee_x(vec![load_int_x(Rax, src)?, store_x(Memory(addr, mem_type), Rcx, Rax)?])?;
    json_type.val.to_json(json_type.pos, addr)
  }},
  {"__store_b", COMMON, Exact(2), __store_b => {
    let dst = arg!(func, (Int(x)) => x).val;
    let src = arg!(func, (Int(x)) => x).val;
    _scope.ee_x(vec![
      load_int_x(Rax, dst)?,
      load_int_x(Rcx, src)?,
      vec![store(S1, Ref(Rax), Rcx)]
    ])?;
    Ok(Null(Lit(())))
  }},
  {"__store_q", COMMON, Exact(2), __store_q => {
    let dst = arg!(func, (Int(x)) => x).val;
    let src = func.arg()?;
    _scope.ee_x(vec![
      load_int_x(Rax, dst)?,
      self.load_json_x(Rcx, &src, Some(_scope.id))?,
      vec![store(S8, Ref(Rax), Rcx)]
    ])?;
    Ok(Null(Lit(())))
  }},
  {"__win_api", SPECIAL, AtLeast(3),
    __win_api => { self.windows_api(false, func, _scope) }
  },
  {"__win_api_check", SPECIAL, AtLeast(3),
    __win_api_check => { self.windows_api(true, func, _scope) }
  },
  {"list", COMMON, AtLeast(0),
    list => { self.create_list(func) },
    list_a => { self.create_list(func) }
  },
  {"main", SPECIAL, Exact(1),
    n_is_main => { self.name_is_main(func, _scope) },
    n_is_main_a => { self.name_is_main(func, _scope) }
  },
  {"value", COMMON, Exact(1),
    value => { Ok(func.arg()?.val) },
    value_a => { Ok(func.arg()?.val) }
  },
}
impl Jsonpiler {
  fn create_list(&mut self, func: &mut Pos<BuiltIn>) -> ErrOR<Json> {
    let args = take(&mut func.val.args).collect();
    Ok(Array(ArrayType::new(None, None), Lit(args)))
  }
  fn name_is_main(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    if self.first_file()?.path == self.files[func.pos.file].path {
      Ok(self.eval(func.arg()?, scope)?.val)
    } else {
      Ok(Null(Lit(())))
    }
  }
  pub(crate) fn windows_api(
    &mut self,
    check: bool,
    func: &mut Pos<BuiltIn>,
    scope: &mut Scope,
  ) -> ErrOR<Json> {
    let mut dll = func.arg()?.into_ident("DLL NAME")?.val;
    dll.push_str(".dll");
    let api_name = func.arg()?.into_ident("API NAME")?.val;
    let ret_type = func.arg()?.into_ident("RET VAL")?.map(|val| JsonType::from_str(val.as_ref()));
    let api = self.api(&dll, &api_name);
    let args_len = func.val.len - 3;
    scope.update_stack_args_count(args_len.saturating_sub(len_u32(&X64_ARG_REGS)?));
    let mut args = vec![];
    for _ in 0..args_len {
      let arg = self.eval(func.arg()?, scope)?;
      func.push_free_tmp(arg.val.mem());
      args.push(arg);
    }
    self.store_args_x(args, false, scope)?;
    scope.p_x(if check { CallApiCheck(api) } else { CallApi(api) })?;
    scope.ret_json_take_x(&ret_type, Rax)
  }
}
