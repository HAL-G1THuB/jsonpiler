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
use std::collections::hash_map::Entry::{Occupied, Vacant};
built_in! {self, func, scope, output;
  message => {"message", COMMON, Exactly(2), {
    scope.use_reg(Rdi);
    scope.use_reg(Rsi);
    let title = self.mov_str(Rcx, func, 1)?;
    let msg = self.mov_str(Rcx, func, 2)?;
    let u8_to_16  = self.get_u8to16();
    let heap_free = self.import(Jsonpiler::KERNEL32, "HeapFree", 0x357);
    let message_box = self.import(Jsonpiler::USER32, "MessageBoxW", 0x28c);
    let heap = Global{id: self.sym_table["HEAP"]};
    let win_handler = self.sym_table["WIN_HANDLER"];
    scope.extend(&[
      msg,
      Call(u8_to_16),
      MovQQ(Rq(Rdi), Rq(Rax)),
      title,
      Call(u8_to_16),
      MovQQ(Rq(Rsi), Rq(Rax)),
      Clear(Rcx),
      MovQQ(Rq(Rdx), Rq(Rdi)),
      MovQQ(Rq(R8), Rq(Rsi)),
      Clear(R9),
      CallApi(message_box),
      TestRR(Rax, Rax),
      Jze(win_handler),
      MovQQ(Rq(Rcx), Mq(heap)),
      Clear(Rdx),
      MovQQ(Rq(R8), Rq(Rdi)),
      CallApi(heap_free),
      TestRR(Rax, Rax),
      Jze(win_handler),
      MovQQ(Rq(Rcx), Mq(heap)),
      Clear(Rdx),
      MovQQ(Rq(R8), Rq(Rsi)),
      CallApi(heap_free),
      TestRR(Rax, Rax),
      Jze(win_handler),
    ]);
    Ok(Json::Null)
  }}
}
impl Jsonpiler {
  fn get_u8to16(&mut self) -> usize {
    let heap = self.sym_table["HEAP"];
    let win_handler = self.sym_table["WIN_HANDLER"];
    let multi_byte_to_wide_char = self.import(Jsonpiler::KERNEL32, "MultiByteToWideChar", 0x3F8);
    let heap_alloc = self.import(Jsonpiler::KERNEL32, "HeapAlloc", 0x353);
    match self.sym_table.entry("U8TO16") {
      Occupied(entry) => *entry.get(),
      Vacant(entry) => {
        let id = self.label_id;
        self.label_id += 1;
        self.insts.extend_from_slice(&[
          Lbl(id),
          Push(Rdi),
          Push(Rsi),
          Push(Rbx),
          Push(Rbp),
          MovQQ(Rq(Rbp), Rq(Rsp)),
          SubRId(Rsp, 0x48),
          MovQQ(Rq(Rdi), Rq(Rcx)),
          MovRId(Rcx, 65001),
          Clear(Rdx),
          MovQQ(Rq(R8), Rq(Rdi)),
          MovRId(R9, u32::MAX),
          Clear(Rax),
          MovQQ(Args(0x20), Rq(Rax)),
          MovQQ(Args(0x28), Rq(Rax)),
          CallApi(multi_byte_to_wide_char),
          TestRR(Rax, Rax),
          Jze(win_handler),
          Shl1R(Rax),
          MovQQ(Rq(Rsi), Rq(Rax)),
          MovQQ(Rq(Rcx), Mq(Global { id: heap })),
          Clear(Rdx),
          MovQQ(Rq(R8), Rq(Rsi)),
          CallApi(heap_alloc),
          MovQQ(Rq(Rbx), Rq(Rax)),
          MovRId(Rcx, 65001),
          Clear(Rdx),
          MovQQ(Rq(R8), Rq(Rdi)),
          MovRId(R9, u32::MAX),
          MovQQ(Args(0x20), Rq(Rbx)),
          MovQQ(Args(0x28), Rq(Rsi)),
          CallApi(multi_byte_to_wide_char),
          TestRR(Rax, Rax),
          Jze(win_handler),
          MovQQ(Rq(Rax), Rq(Rbx)),
          MovQQ(Rq(Rsp), Rq(Rbp)),
          Pop(Rbp),
          Pop(Rbx),
          Pop(Rsi),
          Pop(Rdi),
          Ret,
        ]);
        entry.insert(id);
        id
      }
    }
  }
}
