use crate::{
  Arity::{AtLeast, Exactly},
  Bind::{Lit, Var},
  ErrOR, FuncInfo,
  Inst::*,
  Json, Jsonpiler,
  OpQ::{Iq, Mq, Rq},
  Reg::*,
  ScopeInfo,
  VarKind::Global,
  built_in, err, take_arg,
};
use std::collections::hash_map::Entry::{Occupied, Vacant};
built_in! {self, func, scope, arithmetic;
  abs => {"abs", COMMON, Exactly(1), {
    self.mov_int(Rax, func, 1, scope)?;
    scope.push(Cqo);
    scope.push(XorRR(Rax, Rdx));
    scope.push(SubRR(Rax, Rdx));
    Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
  }},
  div => {"/", COMMON, AtLeast(2), {
    self.mov_int(Rax, func, 1, scope)?;
    for nth in 2..=func.len {
      self.mov_rcx_nonzero(scope, func, nth)?;
      scope.push(Cqo);
      scope.push(IDivR(Rcx));
    }
    Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
  }},
  minus => {"-", COMMON, AtLeast(1), {
    if func.len == 1 {
      self.mov_int(Rax, func, 1, scope)?;
      scope.push(NegR(Rax));
      return Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
    }
    self.mov_int(Rax, func, 1, scope)?;
    for nth in 2..=func.len {
      self.mov_int(Rcx, func, nth, scope)?;
      scope.push(SubRR(Rax, Rcx));
    }
    Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
  }},
  mul => {"*", COMMON, AtLeast(2), {
    self.mov_int(Rax, func, 1, scope)?;
    for nth in 2..=func.len {
    self.mov_int(Rcx, func, nth, scope)?;
    scope.push(IMulRR(Rax, Rcx));
    }
    Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
  }},
  plus => {"+", COMMON, AtLeast(2), {
    self.mov_int(Rax, func, 1, scope)?;
    for nth in 2..=func.len {
      self.mov_int(Rcx, func, nth, scope)?;
      scope.push(AddRR(Rax, Rcx));
    }
    Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
  }},
  rem => {"%", COMMON, Exactly(2), {
    self.mov_int(Rax, func, 1, scope)?;
    self.mov_rcx_nonzero(scope, func, 2)?;
    scope.push(Cqo);
    scope.push(IDivR(Rcx));
    Ok(Json::Int(Var(scope.mov_tmp(Rdx)?)))
  }}
}
impl Jsonpiler {
  fn mov_rcx_nonzero(
    &mut self, scope: &mut ScopeInfo, func: &mut FuncInfo, nth: usize,
  ) -> ErrOR<()> {
    let (int, pos) = take_arg!(self, func, nth, "Int", Json::Int(x) => x);
    match int {
      Lit(l_int) => {
        if l_int == 0 {
          return err!(self, pos, "ZeroDivisionError");
        }
        #[expect(clippy::cast_sign_loss)]
        scope.push(MovQQ(Rq(Rcx), Iq(l_int as u64)));
      }
      Var(label) => {
        func.sched_free_tmp(&label);
        scope.push(MovQQ(Rq(Rcx), Mq(label.kind)));
        scope.push(CmpRIb(Rcx, 0));
        let zero_division_msg = self.global_str("ZeroDivisionError".to_owned());
        let message_box = self.import(Jsonpiler::USER32, "MessageBoxA", 0x285);
        let exit_process = self.import(Jsonpiler::KERNEL32, "ExitProcess", 0x167);
        let zero_division_err = match self.sym_table.entry("ZERO_DIVISION_ERR") {
          Occupied(entry) => *entry.get(),
          Vacant(entry) => {
            let id = self.ctx.gen_id();
            self.insts.extend_from_slice(&[
              Label(id),
              Clear(Rcx),
              LeaRM(Rdx, Global { id: zero_division_msg }),
              Clear(R8),
              MovRdId(R9, 0x10),
              CallApi(message_box),
              MovRdId(Rcx, u32::MAX),
              CallApi(exit_process),
            ]);
            entry.insert(id);
            id
          }
        };
        scope.push(JzJe(zero_division_err));
      }
    }
    Ok(())
  }
}
