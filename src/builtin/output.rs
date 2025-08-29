use crate::{
  Arity::Exactly,
  ErrOR, FuncInfo,
  Inst::*,
  Json, Jsonpiler,
  OpQ::{Args, Mq, Rq},
  Reg::*,
  ScopeInfo,
  VarKind::Global,
  built_in,
};
built_in! {self, func, scope, output;
  message => {"message", COMMON, Exactly(2), {
    self.take_str(Rcx, func, scope)?;
    self.take_str(Rdx, func, scope)?;
    scope.push(Call(self.get_msg_box()));
    Ok(Json::Null)
  }},
  print => {"print", COMMON, Exactly(1), {
    scope.update_stack_args(1);
    let std_o = Global { id: self.sym_table["STDO"], disp: 0i32 };
    let write_file = self.import(Jsonpiler::KERNEL32, "WriteFile", 0x628);
    self.take_str_len(Rdx, R8, func, scope)?;
    scope.extend(&[
      MovQQ(Rq(Rcx), Mq(std_o)),
      LeaRM(R9, Global{id: self.sym_table["TMP"], disp: 0}),
      Clear(Rax),
      MovQQ(Args(0x20), Rq(Rax)),
    ]);
    scope.extend(&self.call_api_check_null(write_file));
    Ok(Json::Null)
  }}
}
