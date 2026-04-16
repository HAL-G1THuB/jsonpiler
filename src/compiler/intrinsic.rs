use crate::prelude::*;
built_in! {self, func, _scope, intrinsic;
  __alloc => {"__alloc", COMMON, Exact(1), {
    let heap_alloc = self.import(KERNEL32, "HeapAlloc");
    let leak = Global(self.symbols[LEAK_CNT]);
    _scope.extend(&[mov_q(Rcx, Global(self.symbols[HEAP])), mov_d(Rdx, 8)]);
    _scope.extend(&mov_int(R8, arg!(self, func, (Int(x)) => x).val));
    _scope.extend(&[CallApi(heap_alloc), IncMd(leak)]);
    Ok(Int(Var(_scope.ret(Rax)?)))
  }},
  __free => {"__free", COMMON, Exact(1), {
    let heap_free = self.import(KERNEL32, "HeapFree");
    let leak = Global(self.symbols[LEAK_CNT]);
    _scope.extend(&[mov_q(Rcx, Global(self.symbols[HEAP])), Clear(Rdx)]);
    _scope.extend(&mov_int(R8, arg!(self, func, (Int(x)) => x).val));
    _scope.extend(&[CallApiCheck(heap_free), DecMd(leak)]);
    Ok(Null(Lit(())))
  }},
  __win_api => {"__win_api", SPECIAL, AtLeast(3), { self.windows_api(false, func, _scope) }},
  __win_api_check => {"__win_api_check", SPECIAL, AtLeast(3), { self.windows_api(true, func, _scope) }},
  list => {"list", COMMON, AtLeast(0), { Ok(Array(Lit(take(&mut func.val.args).collect()))) }},
  name_is_main => {"main", SPECIAL, Exact(1), {
    if self.parsers[0].val.file == self.parsers[func.pos.file as usize].val.file {
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
    let api = self.import(&dll, &api_name);
    scope.update_args_count(func.val.len - 3);
    for idx in 0..func.val.len - 3 {
      let arg = self.eval(func.arg()?, scope)?;
      func.push_free_tmp(arg.val.memory());
      self.mov_args_json(idx, scope, arg, false)?;
    }
    scope.push(if check { CallApiCheck(api) } else { CallApi(api) });
    scope.ret_json_take(&ret_type, Rax)
  }
}
