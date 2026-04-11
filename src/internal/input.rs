use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn get_input(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x40;
    const CAPACITY: u32 = 1 << 20;
    let id = symbol!(self, caller, INPUT);
    let read_file = self.import(KERNEL32, "ReadFile");
    let heap_alloc = self.import(KERNEL32, "HeapAlloc");
    let heap_re_alloc = self.import(KERNEL32, "HeapReAlloc");
    let heap_free = self.import(KERNEL32, "HeapFree");
    let get_last_err = self.import(KERNEL32, "GetLastError");
    let read_console_w = self.import(KERNEL32, "ReadConsoleW");
    let heap = Global(self.symbols[HEAP]);
    let std_i = Global(self.symbols[STD_I]);
    let buffer = Local(Tmp, -0x8);
    let read_len = Local(Tmp, -0x10);
    let handle_pipe = self.id();
    let handle_stdin = self.id();
    let re_alloc = self.id();
    let epilogue = self.id();
    let u16_to_8 = self.get_u16_to_8(id)?;
    let str_len = self.str_len(id)?;
    let leak = Global(self.symbols[LEAK_CNT]);
    self.link_function(
      id,
      &[
        mov_q(Rcx, heap),
        mov_d(Rdx, 8),
        mov_d(R8, CAPACITY),
        CallApi(heap_alloc),
        IncMd(leak),
        mov_q(buffer, Rax),
        mov_q(Rcx, std_i),
        mov_q(Rdx, buffer),
        mov_d(R8, (CAPACITY >> 1) - 1),
        LeaRM(R9, read_len),
        Clear(Rax),
        mov_q(Args(5), Rax),
        CallApi(read_console_w),
        LogicRR(Test, Rax, Rax),
        JCc(Ne, handle_stdin),
        CallApi(get_last_err),
        mov_d(Rcx, 1),
        LogicRR(Cmp, Rax, Rcx),
        JCc(E, handle_pipe),
        mov_d(Rcx, 6),
        LogicRR(Cmp, Rax, Rcx),
        JCc(E, handle_pipe),
        mov_d(Rcx, 0x57),
        LogicRR(Cmp, Rax, Rcx),
        JCc(Ne, self.handlers.win),
        Lbl(handle_pipe),
        mov_q(Rcx, std_i),
        mov_q(Rdx, buffer),
        mov_d(R8, CAPACITY - 1),
        LeaRM(R9, read_len),
        Clear(Rax),
        mov_q(Args(5), Rax),
        CallApi(read_file),
        LogicRR(Test, Rax, Rax),
        JCc(Ne, re_alloc),
        CallApi(get_last_err),
        mov_d(Rcx, 0x6d),
        LogicRR(Cmp, Rax, Rcx),
        JCc(Ne, self.handlers.win),
        Jmp(re_alloc),
        Lbl(handle_stdin),
        mov_d(Rcx, read_len),
        ShiftR(Shl, Rcx, Shift::One),
        mov_q(Rax, buffer),
        AddRR(Rax, Rcx),
        Clear(Rcx),
        DecR(Rax),
        DecR(Rax),
        mov_b(Ref(Rax), Rcx),
        mov_q(Rcx, buffer),
        mov_d(Rdx, 65001),
        Call(u16_to_8),
        mov_q(Rcx, heap),
        Clear(Rdx),
        mov_q(R8, buffer),
        mov_q(buffer, Rax),
        CallApiCheck(heap_free),
        DecMd(leak),
        mov_q(Rcx, buffer),
        Call(str_len),
        IncR(Rax),
        mov_d(read_len, Rax),
        mov_q(Rax, buffer),
        Jmp(epilogue),
        Lbl(re_alloc),
        mov_q(Rcx, heap),
        mov_d(Rdx, 8),
        mov_q(R8, buffer),
        mov_d(R9, read_len),
        IncR(R9),
        CallApi(heap_re_alloc),
        Lbl(epilogue),
      ],
      SIZE,
    );
    Ok(id)
  }
}
