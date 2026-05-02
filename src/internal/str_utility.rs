use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn copy_str(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x50;
    let id = symbol!(self, caller, COPY2HEAP);
    let heap = Global(self.symbols[HEAP]);
    let tmp_d = Local(Tmp, -0x08);
    let tmp_s = Local(Tmp, -0x10);
    let tmp_b = Local(Tmp, -0x18);
    let tmp_12 = Local(Tmp, -0x20);
    let insts = &[
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
      CallApi(self.api(KERNEL32, "HeapAlloc")),
      IncMd(Global(self.symbols[LEAK_CNT])),
      mov_q(Rcx, Rbx),
      mov_q(Rdi, Rax),
      mov_q(Rsi, R12),
      Custom(CLD_REP_MOVSB),
      mov_q(Rdi, tmp_d),
      mov_q(Rsi, tmp_s),
      mov_q(Rbx, tmp_b),
      mov_q(R12, tmp_12),
    ];
    self.link_function(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn get_int_to_str(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x50;
    let id = symbol!(self, caller, INT2STR);
    let count_end = self.id();
    let negative = self.id();
    let positive = self.id();
    let count_start = self.id();
    let write_start = self.id();
    let write_positive = self.id();
    let i64_min = self.id();
    let epilogue = self.id();
    let tmp_s = Local(Tmp, -0x08);
    let tmp_b = Local(Tmp, -0x10);
    let count = Local(Tmp, -0x1C);
    let is_neg = Local(Tmp, -0x1D);
    let i64_min_str = Global(self.global_str(i64::MIN.to_string()));
    let copy_str = self.copy_str(id)?;
    let insts = &[
      mov_q(tmp_s, Rsi),
      mov_q(tmp_b, Rbx),
      mov_q(Rsi, Rcx),
      mov_b(is_neg, 0),
      mov_d(count, 1),
      Clear(Rdx),
      LogicRR(Cmp, Rsi, Rdx),
      JCc(G, positive),
      JCc(Ne, negative),
      IncMd(count),
      Jmp(count_end),
      Lbl(negative),
      mov_b(is_neg, 0xFF),
      UnaryR(Neg, Rsi),
      LogicRR(Cmp, Rsi, Rcx),
      JCc(E, i64_min),
      IncMd(count),
      Jmp(positive),
      Lbl(i64_min),
      LeaRM(Rcx, i64_min_str),
      Call(copy_str),
      Jmp(epilogue),
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
      CallApi(self.api(KERNEL32, "HeapAlloc")),
      IncMd(Global(self.symbols[LEAK_CNT])),
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
      AddRId(Rdx, b'0' as u32),
      DecR(Rcx),
      mov_b(Ref(Rcx), Rdx),
      LogicRR(Test, Rax, Rax),
      JCc(Ne, write_start),
      mov_b(Rax, is_neg),
      LogicRbRb(Test, Rax, Rax),
      JCc(E, write_positive),
      mov_b(Rdx, b'-'),
      DecR(Rcx),
      mov_b(Ref(Rcx), Rdx),
      Lbl(write_positive),
      mov_q(Rax, Rcx),
      Lbl(epilogue),
      mov_q(Rsi, tmp_s),
      mov_q(Rbx, tmp_b),
    ];
    self.link_function(id, insts, SIZE);
    Ok(id)
  }
  #[expect(clippy::too_many_lines)]
  pub(crate) fn get_utf8_slice(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x50;
    let id = symbol!(self, caller, UTF8_SLICE);
    let tmp_d = Local(Tmp, -0x08);
    let tmp_s = Local(Tmp, -0x10);
    let tmp_b = Local(Tmp, -0x18);
    let slice = Local(Tmp, -0x20);
    let slice_len = Local(Tmp, -0x28);
    let tmp_12 = Local(Tmp, -0x30);
    let start = self.id();
    let count = self.id();
    let done = self.id();
    let abort = self.id();
    let abort_count = self.id();
    let epilogue = self.id();
    let start_is_posi = self.id();
    let end_is_posi = self.id();
    let search_slice = self.id();
    let insts = &[
      mov_q(tmp_b, Rbx),
      mov_q(tmp_12, R12),
      mov_q(tmp_d, Rdx),
      mov_q(tmp_s, R8),
      mov_q(slice, Rcx),
      Clear(Rdx),
      Clear(R8),
      DecR(R8),
      Call(search_slice),
      mov_q(Rdx, tmp_d),
      mov_q(R8, tmp_s),
      mov_q(tmp_d, Rdi),
      mov_q(tmp_s, Rsi),
      Clear(Rcx),
      LogicRR(Cmp, Rdx, Rcx),
      JCc(Ge, start_is_posi),
      AddRR(Rdx, R11),
      Lbl(start_is_posi),
      LogicRR(Cmp, R8, Rcx),
      JCc(Ge, end_is_posi),
      AddRR(R8, R11),
      Lbl(end_is_posi),
      LogicRR(Cmp, Rdx, R8),
      JCc(Ge, abort),
      mov_q(Rcx, slice),
      Call(search_slice),
      LogicRbRb(Test, Rax, Rax),
      JCc(E, abort),
      mov_q(Rax, slice),
      AddRR(Rax, R9),
      mov_q(slice, Rax),
      mov_q(Rcx, Global(self.symbols[HEAP])),
      mov_d(Rdx, 8),
      mov_q(R8, R10),
      SubRR(R8, R9),
      mov_q(slice_len, R8),
      IncR(R8),
      CallApi(self.api(KERNEL32, "HeapAlloc")),
      IncMd(Global(self.symbols[LEAK_CNT])),
      mov_q(Rcx, slice_len),
      mov_q(Rdi, Rax),
      mov_q(Rsi, slice),
      Custom(CLD_REP_MOVSB),
      Jmp(epilogue),
      Lbl(abort),
      mov_q(Rcx, Global(self.symbols[HEAP])),
      mov_d(Rdx, 8),
      mov_d(R8, 1),
      CallApi(self.api(KERNEL32, "HeapAlloc")),
      IncMd(Global(self.symbols[LEAK_CNT])),
      Lbl(epilogue),
      mov_q(Rdi, tmp_d),
      mov_q(Rsi, tmp_s),
      mov_q(Rbx, tmp_b),
      mov_q(R12, tmp_12),
    ];
    self.link_function(id, insts, SIZE);
    self.use_function(id, search_slice);
    self.link_function(
      search_slice,
      &[
        Clear(Rax),
        Clear(R9),
        Clear(R10),
        Clear(R11),
        Lbl(start),
        mov_b(R12, SibDisp(Sib { base: Rcx, index: R10, scale: S1 }, Disp::Zero)),
        mov_b(Rax, R12),
        mov_b(Rbx, 0xC0),
        LogicRbRb(And, Rax, Rbx),
        mov_b(Rbx, 0x80),
        LogicRbRb(Cmp, Rax, Rbx),
        JCc(E, count),
        LogicRR(Cmp, R11, Rdx),
        CMovCc(E, R9, R10),
        LogicRR(Cmp, R11, R8),
        JCc(E, done),
        LogicRbRb(Test, R12, R12),
        JCc(E, abort_count),
        IncR(R11),
        Lbl(count),
        IncR(R10),
        Jmp(start),
        Lbl(done),
        Clear(Rax),
        IncR(Rax),
        Lbl(abort_count),
      ],
      SIZE,
    );
    Ok(id)
  }
  pub(crate) fn str_chars_len(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x30;
    let id = symbol!(self, caller, STR_CHARS_LEN);
    let start = self.id();
    let count = self.id();
    let epilogue = self.id();
    self.link_function(
      id,
      &[
        Clear(Rax),
        Clear(R9),
        Clear(R10),
        Clear(R11),
        Lbl(start),
        mov_b(R11, SibDisp(Sib { base: Rcx, index: R10, scale: S1 }, Disp::Zero)),
        LogicRbRb(Test, R11, R11),
        JCc(E, epilogue),
        mov_b(Rdx, 0xC0),
        LogicRbRb(And, R11, Rdx),
        mov_b(Rdx, 0x80),
        LogicRbRb(Cmp, R11, Rdx),
        JCc(E, count),
        IncR(Rax),
        Lbl(count),
        IncR(R10),
        Jmp(start),
        Lbl(epilogue),
      ],
      SIZE,
    );
    Ok(id)
  }
  pub(crate) fn str_eq(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x20;
    let id = symbol!(self, caller, STR_EQ);
    let epilogue = self.id();
    let case_true = self.id();
    let start = self.id();
    self.link_function(
      id,
      &[
        Clear(Rax),
        Lbl(start),
        mov_b(R8, Ref(Rcx)),
        mov_b(R9, Ref(Rdx)),
        LogicRbRb(Cmp, R8, R9),
        JCc(Ne, epilogue),
        LogicRbRb(Test, R8, R8),
        JCc(E, case_true),
        IncR(Rcx),
        IncR(Rdx),
        Jmp(start),
        Lbl(case_true),
        mov_b(Rax, 0xFF),
        Lbl(epilogue),
      ],
      SIZE,
    );
    Ok(id)
  }
  pub(crate) fn str_len(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x30;
    let id = symbol!(self, caller, STR_LEN);
    let tmp_d = Local(Tmp, -0x8);
    self.link_function(
      id,
      &[
        mov_q(tmp_d, Rdi),
        mov_q(Rdx, Rcx),
        mov_q(Rdi, Rdx),
        Clear(Rcx),
        DecR(Rcx),
        Clear(Rax),
        Custom(CLD_REPNE_SCASB),
        SubRR(Rdi, Rdx),
        DecR(Rdi),
        mov_q(Rax, Rdi),
        mov_q(Rdi, tmp_d),
      ],
      SIZE,
    );
    Ok(id)
  }
}
