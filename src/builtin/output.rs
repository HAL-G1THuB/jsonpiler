use crate::{
  Arity::Exactly, ErrOR, FuncInfo, Inst::*, Json, Jsonpiler, Operand::Args, Register::*, ScopeInfo,
  VarKind::Global, built_in, utility::mov_q,
};
built_in! {self, func, scope, output;
  message => {"message", COMMON, Exactly(2), {
    self.take_str(Rcx, func, scope)?;
    self.take_str(Rdx, func, scope)?;
    scope.push(Call(self.get_msg_box()?));
    Ok(Json::Null)
  }},
  print => {"print", COMMON, Exactly(1), {
    scope.update_stack_args(1);
    let std_o = Global { id: self.sym_table["STDO"], disp: 0 };
    let write_file = self.import(Jsonpiler::KERNEL32, "WriteFile")?;
    self.take_str_len_c_a_d(Rdx, R8, func, scope)?;
    scope.extend(&[
      mov_q(Rcx, std_o),
      LeaRM(R9, Global { id: self.sym_table["TMP"], disp: 0 }),
      Clear(Rax),
      mov_q(Args(0x20), Rax),
    ]);
    scope.extend(&self.call_api_check_null(write_file));
    Ok(Json::Null)
  }}
}
