use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn copy_str(&mut self) -> ErrOR<u32> {
    const SIZE: u32 = 0x50;
    let id = symbol!(self, COPY2HEAP);
    let end = self.id();
    let heap = Global(self.symbols[HEAP]);
    let heap_alloc = self.import(KERNEL32, "HeapAlloc")?;
    self.data_insts.push(Seh(id, end, SIZE));
    let tmp_d = Local(Tmp, -0x08);
    let tmp_s = Local(Tmp, -0x10);
    let tmp_b = Local(Tmp, -0x18);
    let tmp_12 = Local(Tmp, -0x20);
    self.insts.extend_from_slice(&[
      Lbl(id),
      Push(Rbp),
      mov_q(Rbp, Rsp),
      SubRId(Rsp, SIZE),
      mov_q(tmp_d, Rdi),
      mov_q(tmp_s, Rsi),
      mov_q(tmp_b, Rbx),
      mov_q(tmp_12, R12),
      mov_q(Rdi, Rcx),
      mov_q(R12, Rdi),
      Clear(Rcx),
      DecR(Rcx),
      Clear(Rax),
      Custom(CLD_REPNE_SCASB),
      SubRR(Rdi, R12),
      mov_q(Rbx, Rdi),
      mov_q(Rcx, heap),
      mov_d(Rdx, 8),
      mov_q(R8, Rbx),
      CallApi(heap_alloc),
      mov_q(Rcx, Rbx),
      mov_q(Rdi, Rax),
      mov_q(Rsi, R12),
      Custom(CLD_REP_MOVSB),
      mov_q(Rdi, tmp_d),
      mov_q(Rsi, tmp_s),
      mov_q(Rbx, tmp_b),
      mov_q(R12, tmp_12),
      mov_q(Rsp, Rbp),
      Pop(Rbp),
      Custom(RET),
      Lbl(end),
    ]);
    Ok(id)
  }
  pub(crate) fn get_int_to_str(&mut self) -> ErrOR<u32> {
    const SIZE: u32 = 0x50;
    let id = symbol!(self, INT2STR);
    let end = self.id();
    self.data_insts.push(Seh(id, end, SIZE));
    let heap_alloc = self.import(KERNEL32, "HeapAlloc")?;
    let count_end = self.id();
    let nonzero = self.id();
    let positive = self.id();
    let count_start = self.id();
    let write_start = self.id();
    let write_positive = self.id();
    let tmp_s = Local(Tmp, -0x08);
    let tmp_b = Local(Tmp, -0x10);
    let count = Local(Tmp, -0x1C);
    let neg = Local(Tmp, -0x1D);
    self.insts.extend(&[
      Lbl(id),
      Push(Rbp),
      mov_q(Rbp, Rsp),
      SubRId(Rsp, SIZE),
      mov_q(tmp_s, Rsi),
      mov_q(tmp_b, Rbx),
      mov_q(Rsi, Rcx),
      mov_b(neg, 0),
      mov_d(count, 1),
      Clear(Rdx),
      LogicRR(Cmp, Rsi, Rdx),
      JCc(G, positive),
      JCc(Ne, nonzero),
      IncMd(count),
      Jmp(count_end),
      Lbl(nonzero),
      mov_b(neg, 0xFF),
      NegR(Rsi),
      IncMd(count),
      Lbl(positive),
      mov_q(Rax, Rsi),
      Lbl(count_start),
      mov_d(Rbx, 10),
      Custom(CQO),
      IDivR(Rbx),
      IncMd(count),
      LogicRR(Test, Rax, Rax),
      JCc(Ne, count_start),
      Lbl(count_end),
      mov_q(Rcx, Global(self.symbols[HEAP])),
      mov_d(Rdx, 8),
      mov_d(R8, count),
      CallApi(heap_alloc),
      mov_q(Rcx, Rax),
      mov_d(Rax, count),
      AddRR(Rcx, Rax),
      Clear(Rdx),
      DecR(Rcx),
      mov_b(Ref(Rcx), Rdx),
      mov_q(Rax, Rsi),
      Lbl(write_start),
      mov_d(Rbx, 10),
      Custom(CQO),
      IDivR(Rbx),
      AddRId(Rdx, 0x30),
      DecR(Rcx),
      mov_b(Ref(Rcx), Rdx),
      LogicRR(Test, Rax, Rax),
      JCc(Ne, write_start),
      mov_b(Rax, neg),
      LogicRbRb(Test, Rax, Rax),
      JCc(E, write_positive),
      mov_b(Rdx, 0x2D),
      DecR(Rcx),
      mov_b(Ref(Rcx), Rdx),
      Lbl(write_positive),
      mov_q(Rax, Rcx),
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
