use crate::prelude::*;
impl Jsonpiler {
  #[expect(clippy::too_many_lines)]
  pub(crate) fn get_input(&mut self) -> ErrOR<u32> {
    const SIZE: u32 = 0x50;
    const HANDLER_SIZE: u32 = 0x20;
    const INITIAL_CAP: u32 = 0x80;
    let id = symbol!(self, INPUT);
    let end_and_handler = self.id();
    let handler_end = self.id();
    self.data_insts.push(Seh(id, end_and_handler, SIZE));
    self.data_insts.push(Seh(end_and_handler, handler_end, HANDLER_SIZE));
    let read_file = self.import(KERNEL32, "ReadFile")?;
    let write_file = self.import(KERNEL32, "WriteFile")?;
    let heap_alloc = self.import(KERNEL32, "HeapAlloc")?;
    let heap_realloc = self.import(KERNEL32, "HeapReAlloc")?;
    let get_console_mode = self.import(KERNEL32, "GetConsoleMode")?;
    let set_console_mode = self.import(KERNEL32, "SetConsoleMode")?;
    let flush_buffer = self.import(KERNEL32, "FlushConsoleInputBuffer")?;
    let set_ctrl_c_handler = self.import(KERNEL32, "SetConsoleCtrlHandler")?;
    let add_veh = self.import(KERNEL32, "AddVectoredExceptionHandler")?;
    let remove_veh = self.import(KERNEL32, "RemoveVectoredExceptionHandler")?;
    let heap = Global(self.symbols[HEAP]);
    let std_i = Global(self.symbols[STD_I]);
    let std_o = Global(self.symbols[STD_O]);
    let buf_addr = Local(Tmp, -0x8);
    let buf_cap = Local(Tmp, -0xC);
    let buf_len = Local(Tmp, -0x10);
    let read_len = Local(Tmp, -0x14);
    let tmp = Local(Tmp, -0x18);
    let veh_handle = Local(Tmp, -0x20);
    let prev_mode = Global(self.bss(4, 4));
    let new_line = Global(self.global_str("\n"));
    let cr_scan = self.id();
    let start = self.id();
    let read = self.id();
    let clear_cr = self.id();
    let done = self.id();
    self.insts.extend_from_slice(&[
      Lbl(id),
      Push(Rbp),
      mov_q(Rbp, Rsp),
      SubRId(Rsp, SIZE),
      mov_q(Rcx, std_i),
      CallApiNull(flush_buffer),
      mov_q(Rcx, std_i),
      LeaRM(Rdx, prev_mode),
      CallApiNull(get_console_mode),
      LeaRM(Rcx, Global(end_and_handler)),
      Clear(Rdx),
      IncR(Rdx),
      CallApiNull(set_ctrl_c_handler),
      Clear(Rcx),
      LeaRM(Rdx, Global(end_and_handler)),
      CallApiNull(add_veh),
      mov_q(veh_handle, Rax),
      mov_q(Rcx, std_i),
      mov_d(Rdx, prev_mode),
      mov_d(Rax, 6),
      NotR(Rax),
      LogicRR(And, Rdx, Rax),
      CallApiNull(set_console_mode),
      mov_q(Rcx, heap),
      mov_d(Rdx, 8),
      mov_d(R8, INITIAL_CAP),
      CallApi(heap_alloc),
      mov_q(buf_addr, Rax),
      mov_d(buf_cap, INITIAL_CAP),
      mov_d(buf_len, 0),
      Lbl(start),
      mov_d(Rax, buf_cap),
      mov_d(Rcx, buf_len),
      SubRR(Rax, Rcx),
      DecR(Rax),
      Clear(Rdx),
      LogicRR(Cmp, Rax, Rdx),
      JCc(G, read),
      mov_d(Rax, buf_cap),
      Shl1R(Rax),
      mov_d(buf_cap, Rax),
      mov_q(Rcx, heap),
      mov_d(Rdx, 8),
      mov_q(R8, buf_addr),
      mov_d(R9, buf_cap),
      CallApiNull(heap_realloc),
      mov_q(buf_addr, Rax),
      Jmp(start),
      Lbl(read),
      mov_q(Rax, buf_addr),
      mov_d(Rcx, buf_len),
      AddRR(Rax, Rcx),
      mov_q(Rdx, Rax),
      mov_d(R8, buf_cap),
      mov_d(Rcx, buf_len),
      SubRR(R8, Rcx),
      DecR(R8),
      mov_q(Rcx, std_i),
      LeaRM(R9, read_len),
      Clear(Rax),
      mov_q(Args(5), Rax),
      CallApiNull(read_file),
      mov_d(Rax, read_len),
      LogicRR(Test, Rax, Rax),
      JCc(E, done),
      mov_d(Rcx, buf_len),
      AddRR(Rcx, Rax),
      mov_d(buf_len, Rcx),
      mov_q(Rcx, std_o),
      mov_q(Rdx, buf_addr),
      mov_d(Rax, buf_len),
      AddRR(Rdx, Rax),
      mov_d(Rax, read_len),
      SubRR(Rdx, Rax),
      mov_d(R8, read_len),
      LeaRM(R9, tmp),
      Clear(Rax),
      mov_q(Args(5), Rax),
      CallApiNull(write_file),
      mov_q(Rax, buf_addr),
      mov_d(Rcx, buf_len),
      AddRR(Rax, Rcx),
      mov_d(Rcx, read_len),
      SubRR(Rax, Rcx),
      Lbl(cr_scan),
      LogicRR(Test, Rcx, Rcx),
      JCc(E, start),
      mov_b(Rdx, Ref(Rax)),
      CmpRIb(Rdx, 0x0D),
      JCc(E, clear_cr),
      IncR(Rax),
      DecR(Rcx),
      Jmp(cr_scan),
      Lbl(clear_cr),
      DecMd(buf_len),
      Lbl(done),
      mov_q(Rax, buf_addr),
      mov_d(Rcx, buf_len),
      AddRR(Rax, Rcx),
      Clear(Rcx),
      mov_b(Ref(Rax), Rcx),
      mov_q(Rcx, std_i),
      mov_d(Rdx, prev_mode),
      CallApiNull(set_console_mode),
      mov_q(Rcx, std_o),
      LeaRM(Rdx, new_line),
      Clear(R8),
      IncR(R8),
      LeaRM(R9, tmp),
      Clear(Rax),
      mov_q(Args(5), Rax),
      CallApiNull(write_file),
      LeaRM(Rcx, Global(end_and_handler)),
      Clear(Rdx),
      CallApiNull(set_ctrl_c_handler),
      mov_q(Rcx, veh_handle),
      CallApiNull(remove_veh),
      mov_q(Rax, buf_addr),
      mov_q(Rsp, Rbp),
      Pop(Rbp),
      Custom(RET),
      Lbl(end_and_handler),
      Push(Rbp),
      mov_q(Rbp, Rsp),
      SubRId(Rsp, HANDLER_SIZE),
      mov_q(Rcx, std_i),
      mov_d(Rdx, prev_mode),
      CallApi(set_console_mode),
      Clear(Rax),
      mov_q(Rsp, Rbp),
      Pop(Rbp),
      Custom(RET),
      Lbl(handler_end),
    ]);
    Ok(id)
  }
}
