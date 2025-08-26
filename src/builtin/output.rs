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
    self.take_str(Rcx, func, scope)?;
    self.take_str(Rdx, func, scope)?;
    scope.push(Call(self.get_msg_box()));
    Ok(Json::Null)
  }},
  print => {"print", COMMON, Exactly(1), {
    const CLD_REPNE_SCASB: [u8; 3] = [0xFC, 0xF2, 0xAE];
    scope.update_stack_args(1);
    let heap = Global { id: self.sym_table["HEAP"] };
    let std_o = Global { id: self.sym_table["STDO"] };
    let u8_to_16 = self.get_u8to16();
    let heap_free = self.import(Jsonpiler::KERNEL32, "HeapFree", 0x357);
    let write_console_w = self.import(Jsonpiler::KERNEL32, "WriteConsoleW", 0x627);
    let tmp = scope.tmp(8)?;
    let tmp2 = scope.tmp(8)?;
    self.take_str(Rsi, func, scope)?;
    scope.extend(&[
      MovQQ(Mq(tmp.kind), Rq(Rsi)),
      MovQQ(Rq(Rdi), Mq(tmp.kind)),
      Clear(Rcx),
      DecQ(Rcx),
      Clear(Rax),
      Custom(CLD_REPNE_SCASB.to_vec()),
      SubRR(Rdi, Rsi),
      DecQ(Rdi),
      MovQQ(Mq(tmp2.kind), Rq(Rdi)),
      MovQQ(Rq(Rcx), Mq(tmp.kind)),
      Call(u8_to_16),
      MovQQ(Mq(tmp.kind), Rq(Rax)),
      MovQQ(Rq(Rcx), Mq(std_o)),
      MovQQ(Rq(Rdx), Mq(tmp.kind)),
      MovQQ(Rq(R8), Mq(tmp2.kind)),
      Clear(R9),
      MovQQ(Args(0x20), Rq(R9)),
    ]);
    scope.extend(&self.call_api_check_null(write_console_w));
    scope.extend(&[
      MovQQ(Rq(Rcx), Mq(heap)),
      Clear(Rdx),
      MovQQ(Rq(R8), Mq(tmp.kind)),
    ]);
    scope.extend(&self.call_api_check_null(heap_free));
    Ok(Json::Null)
  }}
}
impl Jsonpiler {
  fn get_msg_box(&mut self) -> usize {
    let heap = Global { id: self.sym_table["HEAP"] };
    let u8_to_16 = self.get_u8to16();
    let heap_free = self.import(Jsonpiler::KERNEL32, "HeapFree", 0x357);
    let message_box = self.import(Jsonpiler::USER32, "MessageBoxW", 0x28c);
    let msg_box_insts = self.call_api_check_null(message_box);
    let heap_free_insts = self.call_api_check_null(heap_free);
    match self.sym_table.entry("MSG_BOX") {
      Occupied(entry) => *entry.get(),
      Vacant(entry) => {
        let id = self.label_id;
        self.label_id += 1;
        self.insts.extend_from_slice(&[
          Lbl(id),
          Push(Rdi),
          Push(Rsi),
          Push(Rbp),
          MovQQ(Rq(Rbp), Rq(Rsp)),
          SubRId(Rsp, 0x20),
          MovQQ(Rq(Rsi), Rq(Rdx)),
          Call(u8_to_16),
          MovQQ(Rq(Rdi), Rq(Rax)),
          MovQQ(Rq(Rcx), Rq(Rsi)),
          Call(u8_to_16),
          MovQQ(Rq(Rsi), Rq(Rax)),
          Clear(Rcx),
          MovQQ(Rq(Rdx), Rq(Rsi)),
          MovQQ(Rq(R8), Rq(Rdi)),
          Clear(R9),
        ]);
        self.insts.extend_from_slice(&msg_box_insts);
        self.insts.extend_from_slice(&[
          MovQQ(Rq(Rcx), Mq(heap)),
          Clear(Rdx),
          MovQQ(Rq(R8), Rq(Rdi)),
        ]);
        self.insts.extend_from_slice(&heap_free_insts);
        self.insts.extend_from_slice(&[
          MovQQ(Rq(Rcx), Mq(heap)),
          Clear(Rdx),
          MovQQ(Rq(R8), Rq(Rsi)),
        ]);
        self.insts.extend_from_slice(&heap_free_insts);
        self.insts.extend_from_slice(&[MovQQ(Rq(Rsp), Rq(Rbp)), Pop(Rbp), Pop(Rsi), Pop(Rdi), Ret]);
        entry.insert(id);
        id
      }
    }
  }
  fn get_u8to16(&mut self) -> usize {
    let heap = self.sym_table["HEAP"];
    let multi_byte_to_wide_char = self.import(Jsonpiler::KERNEL32, "MultiByteToWideChar", 0x3F8);
    let mb_t_wc = self.call_api_check_null(multi_byte_to_wide_char);
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
        ]);
        self.insts.extend_from_slice(&mb_t_wc);
        self.insts.extend_from_slice(&[
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
        ]);
        self.insts.extend_from_slice(&mb_t_wc);
        self.insts.extend_from_slice(&[
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
