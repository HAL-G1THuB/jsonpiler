use crate::prelude::*;
built_in! {self, func, scope, string;
  f_concat =>{"concat", COMMON, AtLeast(1), {
    let heap_alloc = self.import(KERNEL32, "HeapAlloc");
    let str_len = self.str_len(scope.id)?;
    let tmp_s = Local(Tmp, scope.alloc(8, 8)?);
    func.push_free_tmp(Memory(tmp_s, Size(8)));
    let tmp_d = Local(Tmp, scope.alloc(8, 8)?);
    func.push_free_tmp(Memory(tmp_d, Size(8)));
    let acc_len = Local(Tmp, scope.alloc(8, 8)?);
    func.push_free_tmp(Memory(acc_len, Size(8)));
    let buffer = Local(Tmp, scope.alloc(8, 8)?);
    let first_string = arg!(self, func, (Str(x)) => x).val;
    let first_len = Local(Tmp, scope.alloc(8, 8)?);
    func.push_free_tmp(Memory(first_len, Size(8)));
    let mut string_vec = vec![(first_string.clone(), first_len)];
    scope.extend(&[
      mov_q(tmp_s, Rsi),
      mov_q(tmp_d, Rdi),
      self.mov_str(Rcx, first_string),
      Call(str_len),
      mov_q(first_len, Rax),
      mov_q(acc_len, Rax),
    ]);
    for _ in 2..=func.len {
      let string = arg!(self, func, (Str(x)) => x).val;
      let len = Local(Tmp, scope.alloc(8, 8)?);
      func.push_free_tmp(Memory(len, Size(8)));
      string_vec.push((string.clone(), len));
      scope.extend(&[
        self.mov_str(Rcx, string),
        Call(str_len),
        mov_q(len, Rax),
        mov_q(Rcx, acc_len),
        AddRR(Rax, Rcx),
        mov_q(acc_len, Rax),
      ]);
    }
    let leak = Global(self.symbols[LEAK_CNT]);
    scope.extend(&[
      mov_q(Rcx, Global(self.symbols[HEAP])),
      mov_d(Rdx, 8),
      mov_q(R8, acc_len),
      IncR(R8),
      CallApi(heap_alloc),
      IncMd(leak),
      mov_q(buffer, Rax),
      mov_q(Rdi, Rax)
    ]);
    for (string, len) in string_vec {
      scope.extend(&[
        mov_q(Rcx, len),
        self.mov_str(Rsi, string),
        Custom(CLD_REP_MOVSB),
      ]);
    }
    scope.extend(&[//dil is not allowed
      mov_q(Rcx, acc_len),
      AddRR(Rax, Rcx),
      Clear(Rcx),
      mov_b(Ref(Rax), Rcx),
      mov_q(Rsi, tmp_s),
      mov_q(Rdi, tmp_d),
    ]);
    Ok(Str(Var(Memory(buffer, Heap(None)))))
  }},
  int_to_string => {"Str", COMMON, Exact(1), {
    scope.extend(&mov_int(Rcx, arg!(self, func, (Int(x)) => x).val));
    scope.push(Call(self.get_int_to_str(scope.id)?));
    scope.ret_str(Rax)
  }},
  len => {"len", COMMON, Exact(1), {
    let str_chars_len = self.str_chars_len(scope.id)?;
    scope.extend(&[self.mov_str(Rcx, arg!(self, func, (Str(x)) => x).val), Call(str_chars_len)]);
    Ok(Int(Var(scope.ret(Rax)?)))
  }},
  slice => {"slice", COMMON, Range(2, 3), {
    let str_chars_len = self.str_chars_len(scope.id)?;
    let string = arg!(self, func, (Str(x)) => x).val;
    if func.len == 3 {
      scope.extend(&mov_int(Rdx, arg!(self, func, (Int(x)) => x).val));
      scope.extend(&mov_int(R8, arg!(self, func, (Int(x)) => x).val));
    }else {
      scope.extend(&[self.mov_str(Rcx, string.clone()), Call(str_chars_len), mov_q(R8, Rax)]);
      scope.extend(&mov_int(Rdx, arg!(self, func, (Int(x)) => x).val));
    }
    scope.push(self.mov_str(Rcx, string));
    scope.push(Call(self.get_utf8_slice(scope.id)?));
    scope.ret_str(Rax)
  }}
}
