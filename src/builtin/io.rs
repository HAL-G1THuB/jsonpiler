use crate::prelude::*;
built_in! {self, _func, scope, output;
  input => {"input", COMMON, Zero, {
    scope.push(Call(self.get_input()?));
    scope.ret_str(Rax)
  }},
  message => {"message", COMMON, Exactly(2), {
    scope.extend(&[
      self.mov_str(Rcx, arg!(self, _func, (Str(x)) => x).val),
      self.mov_str(Rdx, arg!(self, _func, (Str(x)) => x).val),
      Clear(R8),
      Call(self.get_msg_box()?)
    ]);
    Ok(Null)
  }},
  print => {"print", COMMON, Exactly(1), {
    scope.extend(&[
      self.mov_str(Rcx, arg!(self, _func, (Str(x)) => x).val),
      Call(self.get_print()?)
    ]);
    Ok(Null)
  }},
}
