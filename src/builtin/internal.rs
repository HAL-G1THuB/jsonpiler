use crate::{Inst::*, Jsonpiler, OpQ::*, Reg::*, VarKind::Global};
use std::collections::hash_map::Entry::{Occupied, Vacant};
impl Jsonpiler {
  pub(crate) fn get_custom_error(&mut self, err_msg: &'static str) -> usize {
    let err_msg_id = self.global_str(err_msg.to_owned());
    let message_box = self.import(Jsonpiler::USER32, "MessageBoxA", 0x285);
    let mb_a = self.call_api_check_null(message_box);
    let exit_process = self.import(Jsonpiler::KERNEL32, "ExitProcess", 0x167);
    match self.sym_table.entry(err_msg) {
      Occupied(entry) => *entry.get(),
      Vacant(entry) => {
        let id = self.label_id;
        self.label_id += 1;
        self.insts.extend_from_slice(&[
          Lbl(id),
          Clear(Rcx),
          LeaRM(Rdx, Global { id: err_msg_id, disp: 8i32 }),
          Clear(R8),
          MovRId(R9, 0x10),
        ]);
        self.insts.extend_from_slice(&mb_a);
        self.insts.extend_from_slice(&[MovRId(Rcx, u32::MAX), CallApi(exit_process)]);
        entry.insert(id);
        id
      }
    }
  }
  pub(crate) fn get_msg_box(&mut self) -> usize {
    let heap = Global { id: self.sym_table["HEAP"], disp: 0i32 };
    let u8_to_16 = self.get_u8_to_16();
    let heap_free = self.import(Jsonpiler::KERNEL32, "HeapFree", 0x357);
    let message_box_w = self.import(Jsonpiler::USER32, "MessageBoxW", 0x28c);
    let msg_box_insts = self.call_api_check_null(message_box_w);
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
  pub(crate) fn get_random(&mut self) -> usize {
    let random_seed = Global { id: self.sym_table["RANDOM_SEED"], disp: 0i32 };
    match self.sym_table.entry("RANDOM") {
      Occupied(entry) => *entry.get(),
      Vacant(entry) => {
        let id = self.label_id;
        self.label_id += 1;
        self.insts.extend_from_slice(&[
          Lbl(id),
          Push(Rbp),
          MovQQ(Rq(Rbp), Rq(Rsp)),
          SubRId(Rsp, 8),
          MovQQ(Rq(Rax), Mq(random_seed)),
          MovQQ(Rq(Rcx), Rq(Rax)),
          ShlRIb(Rcx, 7),
          XorRR(Rax, Rcx),
          MovQQ(Rq(Rcx), Rq(Rax)),
          ShrRIb(Rcx, 9),
          XorRR(Rax, Rcx),
          MovQQ(Rq(Rcx), Rq(Rax)),
          ShlRIb(Rcx, 13),
          XorRR(Rax, Rcx),
          MovQQ(Mq(random_seed), Rq(Rax)),
          MovQQ(Rq(Rsp), Rq(Rbp)),
          Pop(Rbp),
          Ret,
        ]);
        entry.insert(id);
        id
      }
    }
  }
  pub(crate) fn get_u8_to_16(&mut self) -> usize {
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
          MovQQ(Rq(Rcx), Mq(Global { id: heap, disp: 0i32 })),
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
