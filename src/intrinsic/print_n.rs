use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn get_print(&mut self) -> ErrOR<u32> {
    const SIZE: u32 = 0x20;
    let id = symbol!(self, PRINT);
    let end = self.id();
    let print_n = self.get_print_n()?;
    let std_o = Global(self.symbols[STD_O]);
    self.data_insts.push(Seh(id, end, SIZE));
    self.insts.extend_from_slice(&[
      Lbl(id),
      Push(Rbp),
      mov_q(Rbp, Rsp),
      SubRId(Rsp, SIZE),
      mov_q(Rdx, Rcx),
      mov_q(Rcx, std_o),
      Call(print_n),
      mov_q(Rsp, Rbp),
      Pop(Rbp),
      Custom(RET),
      Lbl(end),
    ]);
    Ok(id)
  }
  pub(crate) fn get_print_e(&mut self) -> ErrOR<u32> {
    const SIZE: u32 = 0x20;
    let id = symbol!(self, PRINT_E);
    let end = self.id();
    let print_n = self.get_print_n()?;
    let std_e = Global(self.symbols[STD_E]);
    self.data_insts.push(Seh(id, end, SIZE));
    self.insts.extend_from_slice(&[
      Lbl(id),
      Push(Rbp),
      mov_q(Rbp, Rsp),
      SubRId(Rsp, SIZE),
      mov_q(Rdx, Rcx),
      mov_q(Rcx, std_e),
      Call(print_n),
      mov_q(Rsp, Rbp),
      Pop(Rbp),
      Custom(RET),
      Lbl(end),
    ]);
    Ok(id)
  }
  pub(crate) fn get_print_n(&mut self) -> ErrOR<u32> {
    const SIZE: u32 = 0x40;
    let id = symbol!(self, PRINT_N);
    let end = self.id();
    self.data_insts.push(Seh(id, end, SIZE));
    let write_file = self.import(KERNEL32, "WriteFile")?;
    let std_n_and_tmp = Local(Tmp, -0x08);
    let tmp_d = Local(Tmp, -0x10);
    let tmp_s = Local(Tmp, -0x18);
    self.insts.extend_from_slice(&[
      Lbl(id),
      Push(Rbp),
      mov_q(Rbp, Rsp),
      SubRId(Rsp, SIZE),
      mov_q(tmp_d, Rdi),
      mov_q(tmp_s, Rsi),
      mov_q(std_n_and_tmp, Rcx),
      mov_q(Rsi, Rdx),
      mov_q(Rdi, Rsi),
      Clear(Rcx),
      DecR(Rcx),
      Clear(Rax),
      Custom(CLD_REPNE_SCASB),
      mov_q(Rdx, Rsi),
      SubRR(Rdi, Rdx),
      DecR(Rdi),
      mov_q(R8, Rdi),
      mov_q(Rcx, std_n_and_tmp),
      LeaRM(R9, std_n_and_tmp),
      Clear(Rax),
      mov_q(Args(5), Rax),
      CallApiNull(write_file),
      mov_q(Rdi, tmp_d),
      mov_q(Rsi, tmp_s),
      mov_q(Rsp, Rbp),
      Pop(Rbp),
      Custom(RET),
      Lbl(end),
    ]);
    Ok(id)
  }
  pub(crate) fn get_u8_to_16(&mut self) -> ErrOR<u32> {
    const SIZE: u32 = 0x60;
    let id = symbol!(self, U8TO16);
    let end = self.id();
    let heap = Global(self.symbols[HEAP]);
    let to_wide_char = self.import(KERNEL32, "MultiByteToWideChar")?;
    let heap_alloc = self.import(KERNEL32, "HeapAlloc")?;
    let tmp_d = Local(Tmp, -0x10);
    let tmp_s = Local(Tmp, -0x18);
    let tmp_b = Local(Tmp, -0x20);
    self.data_insts.push(Seh(id, end, SIZE));
    self.insts.extend_from_slice(&[
      Lbl(id),
      Push(Rbp),
      mov_q(Rbp, Rsp),
      SubRId(Rsp, SIZE),
      mov_q(tmp_d, Rdi),
      mov_q(tmp_s, Rsi),
      mov_q(tmp_b, Rbx),
      mov_q(Rdi, Rcx),
      mov_d(Rcx, 65001),
      Clear(Rdx),
      mov_q(R8, Rdi),
      mov_d(R9, u32::MAX),
      Clear(Rax),
      mov_q(Args(5), Rax),
      mov_q(Args(6), Rax),
      CallApiNull(to_wide_char),
      Shl1R(Rax),
      mov_q(Rsi, Rax),
      mov_q(Rcx, heap),
      Clear(Rdx),
      mov_q(R8, Rsi),
      CallApi(heap_alloc),
      mov_q(Rbx, Rax),
      mov_d(Rcx, 65001),
      Clear(Rdx),
      mov_q(R8, Rdi),
      mov_d(R9, u32::MAX),
      mov_q(Args(5), Rbx),
      mov_q(Args(6), Rsi),
      CallApiNull(to_wide_char),
      mov_q(Rax, Rbx),
      mov_q(Rdi, tmp_d),
      mov_q(Rsi, tmp_s),
      mov_q(Rbx, tmp_b),
      mov_q(Rsp, Rbp),
      Pop(Rbp),
      Custom(RET),
      Lbl(end),
    ]);
    Ok(id)
  }
}
