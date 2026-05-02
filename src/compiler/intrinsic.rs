use crate::prelude::*;
built_in! {self, func, _scope, intrinsic;
  __alloc => {"__alloc", COMMON, Exact(1), {
    let leak = Global(self.symbols[LEAK_CNT]);
    _scope.extend(&mov_int(R8, arg!(func, (Int(x)) => x).val));
    _scope.extend(&[
      mov_q(Rcx, Global(self.symbols[HEAP])),
      mov_d(Rdx, 8),
      CallApi(self.api(KERNEL32, "HeapAlloc")),
      IncMd(leak)
    ]);
    Ok(Int(Var(_scope.ret(Rax)?)))
  }},
  __free => {"__free", COMMON, Exact(1), {
    let leak = Global(self.symbols[LEAK_CNT]);
    _scope.extend(&mov_int(R8, arg!(func, (Int(x)) => x).val));
    _scope.extend(&[
      mov_q(Rcx, Global(self.symbols[HEAP])),
      Clear(Rdx),
      CallApiCheck(self.api(KERNEL32, "HeapFree")),
      DecMd(leak)
    ]);
    Ok(Null(Lit(())))
  }},
  __win_api => {"__win_api", SPECIAL, AtLeast(3), { self.windows_api(false, func, _scope) }},
  __win_api_check => {"__win_api_check", SPECIAL, AtLeast(3), { self.windows_api(true, func, _scope) }},
  list => {"list", COMMON, AtLeast(0), { Ok(Array(Lit(take(&mut func.val.args).collect()))) }},
  name_is_main => {"main", SPECIAL, Exact(1), {
    if self.first_parser()?.val.file == self.parsers[func.pos.file as usize].val.file {
      Ok(self.eval(func.arg()?, _scope)?.val)
    } else {
      Ok(Null(Lit(())))
    }
  }},
  value => {"value", COMMON, Exact(1), { Ok(func.arg()?.val) }},
}
impl Jsonpiler {
  pub(crate) fn windows_api(
    &mut self,
    check: bool,
    func: &mut Pos<BuiltIn>,
    scope: &mut Scope,
  ) -> ErrOR<Json> {
    let mut dll = func.arg()?.into_ident("DLL NAME")?.val;
    dll.push_str(".dll");
    let api_name = func.arg()?.into_ident("API NAME")?.val;
    let ret_type =
      func.arg()?.into_ident("RET VAL")?.map(|val: String| JsonType::from_string(val.as_ref()));
    let api = self.api(&dll, &api_name);
    let args_len = func.val.len - 3;
    scope.update_args_count(args_len);
    for idx in 0..args_len {
      let arg = self.eval(func.arg()?, scope)?;
      func.push_free_tmp(arg.val.memory());
      self.mov_args_json(idx, arg, false, scope)?;
    }
    scope.push(if check { CallApiCheck(api) } else { CallApi(api) });
    scope.ret_json_take(&ret_type, Rax)
  }
}
