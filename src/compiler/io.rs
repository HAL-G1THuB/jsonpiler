use crate::prelude::*;
built_in! {self, _func, scope, io;
  confirm => {"confirm", COMMON, Exact(2), {
    scope.extend(&[
      self.mov_str(Rcx, arg!(_func, (Str(x)) => x).val),
      self.mov_str(Rdx, arg!(_func, (Str(x)) => x).val),
      mov_d(R8, 4),
      Call(self.get_msg_box(scope.id)?),
      mov_d(Rcx, 6),
      Clear(Rdx),
      mov_d(R8, 0xFF),
      LogicRR(Cmp, Rax, Rcx),
      CMovCc(E, Rdx, R8),
    ]);
    scope.ret_bool(Rdx)
  }},
  input => {"input", COMMON, Exact(0), {
    scope.push(Call(self.get_input(scope.id)?));
    scope.ret_str(Rax, HeapPtr)
  }},
  message => {"message", COMMON, Exact(2), {
    scope.extend(&[
      self.mov_str(Rcx, arg!(_func, (Str(x)) => x).val),
      self.mov_str(Rdx, arg!(_func, (Str(x)) => x).val),
      Clear(R8),
      Call(self.get_msg_box(scope.id)?)
    ]);
    Ok(Null(Lit(())))
  }},
  print => {"print", SPECIAL, AtLeast(1), {
    for _ in 1..=_func.val.len {
      let raw = _func.arg()?;
      let printable = self.eval(raw, scope)?;
      let Str(arg) = printable.val else {
        return Err(_func.args_err(vec![StrT], printable.map_ref(Json::as_type)));
      };
      scope.extend(&[self.mov_str(Rcx, arg.clone()), Call(self.get_print(scope.id)?)]);
      self.drop_json(Str(arg), false, scope);
    }
    Ok(Null(Lit(())))
  }},
}
