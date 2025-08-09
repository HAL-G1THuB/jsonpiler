use crate::{Arity::Exactly, ErrOR, FuncInfo, Json, Jsonpiler, ScopeInfo, built_in, include_once};
built_in! {self, func, scope, output;
  message => {"message", COMMON, Exactly(2), {
    scope.use_reg("rdi");
    scope.use_reg("rsi");
    let title = self.get_str_str(func, 1)?;
    let msg = self.get_str_str(func, 2)?;
    include_once!(self, self.text, "func/U8TO16");
    scope.body.push(format!(
      include_str!("../asm/caller/message.s"),
      title = title,
      msg = msg,
    ));
    Ok(Json::Null)
  }}
}
