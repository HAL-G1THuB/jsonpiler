use crate::prelude::*;
built_in! {self, func, _scope, string;
  f_concat =>{"concat", COMMON, AtLeast(1), {
    let mut string = String::new();
    for _ in 1..=func.len {
      string.push_str(&arg!(self, func, (Str(Lit(x))) => x).val);
    }
    Ok(Str(Lit(string)))
  }},
  int_to_string => {"Str", COMMON, Exactly(1), {
    _scope.extend(&mov_int(Rcx, arg!(self, func, (Int(x)) => x).val));
    _scope.push(Call(self.get_int_to_str()?));
    _scope.ret_str(Rax)
  }},
  len =>{"len", COMMON, Exactly(1), {
    self.mov_len(Rax, &arg!(self, func, (Str(x)) => x).val, _scope)?;
    Ok(Int(Var(_scope.ret(Rax)?)))
  }},
}
