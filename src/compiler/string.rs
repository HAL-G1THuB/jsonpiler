use crate::prelude::*;
built_in! {self, func, scope, string;
  int_to_string => {"Str", COMMON, Exact(1), {
    scope.extend(&mov_int(Rcx, arg!(func, (Int(x)) => x).val));
    scope.push(Call(self.get_int_to_str(scope.id)?));
    scope.ret_str(Rax, HeapPtr)
  }},
  len => {"len", COMMON, Exact(1), {
    let str_chars_len = self.str_chars_len(scope.id)?;
    scope.extend(&[self.mov_str(Rcx, arg!(func, (Str(x)) => x).val), Call(str_chars_len)]);
    Ok(Int(Var(scope.ret(Rax)?)))
  }},
  slice => {"slice", COMMON, Range(2, 3), {
    let str_chars_len = self.str_chars_len(scope.id)?;
    let utf8_slice = self.get_utf8_slice(scope.id)?;
    let string = arg!(func, (Str(x)) => x).val;
    if func.val.len == 3 {
      scope.extend(&mov_int(Rdx, arg!(func, (Int(x)) => x).val));
      scope.extend(&mov_int(R8, arg!(func, (Int(x)) => x).val));
    } else {
      scope.extend(&[self.mov_str(Rcx, string.clone()), Call(str_chars_len), mov_q(R8, Rax)]);
      scope.extend(&mov_int(Rdx, arg!(func, (Int(x)) => x).val));
    }
    scope.extend(&[self.mov_str(Rcx, string), Call(utf8_slice)]);
    scope.ret_str(Rax, HeapPtr)
  }}
}
impl Jsonpiler {
  pub(crate) fn concat_strings(
    &mut self,
    mut string: Bind<String>,
    func: &mut Pos<BuiltIn>,
    scope: &mut Scope,
  ) -> ErrOR<Json> {
    let str_len = self.str_len(scope.id)?;
    let tmp_s = scope.tmp(8, 8, func)?;
    let tmp_d = scope.tmp(8, 8, func)?;
    let acc_len = scope.tmp(8, 8, func)?;
    let buffer = Local(Tmp, scope.alloc(8, 8)?);
    scope.extend(&[mov_q(tmp_s, Rsi), mov_q(tmp_d, Rdi), Clear(Rax), mov_q(acc_len, Rax)]);
    let mut string_vec = vec![];
    for nth in 1..=func.val.len {
      let len = scope.tmp(8, 8, func)?;
      string_vec.push((string.clone(), len));
      scope.extend(&[
        self.mov_str(Rcx, take(&mut string)),
        Call(str_len),
        mov_q(len, Rax),
        mov_q(Rcx, acc_len),
        AddRR(Rax, Rcx),
        mov_q(acc_len, Rax),
      ]);
      if nth != func.val.len {
        string = arg!(func, (Str(x)) => x).val;
      }
    }
    let leak = Global(self.symbols[LEAK_CNT]);
    scope.extend(&[
      mov_q(Rcx, Global(self.symbols[HEAP])),
      mov_d(Rdx, 8),
      mov_q(R8, acc_len),
      IncR(R8),
      CallApi(self.api(KERNEL32, "HeapAlloc")),
      IncMd(leak),
      mov_q(buffer, Rax),
      mov_q(Rdi, Rax),
    ]);
    for (st, len) in string_vec {
      scope.extend(&[mov_q(Rcx, len), self.mov_str(Rsi, st), Custom(CLD_REP_MOVSB)]);
    }
    scope.extend(&[
      //dil is not allowed
      mov_q(Rcx, acc_len),
      AddRR(Rax, Rcx),
      Clear(Rcx),
      mov_b(Ref(Rax), Rcx),
      mov_q(Rsi, tmp_s),
      mov_q(Rdi, tmp_d),
    ]);
    Ok(Str(Var(Memory(buffer, MemoryType { heap: HeapPtr, size: Dynamic }))))
  }
}
