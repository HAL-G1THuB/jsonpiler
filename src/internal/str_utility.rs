use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn clone_str_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x10;
    let id = symbol!(self, caller, COPY2HEAP);
    let start = self.id();
    let end = self.id();
    let str_len = self.str_len_a(caller)?;
    let src_str = Local(Tmp, -0x08).v_rq();
    let insts = vec![
      store_a(src_str, X1, X0)?,
      vec![Bl(str_len), AddRI12(X0, X0, 1)],
      self.call_alloc_x0_a()?,
      load_a(X2, src_str)?,
      vec![
        MovRR(X3, X0),
        MovZ(X5, 0, Lsl0),
        LblA(start),
        LdR(S1, X4, X2, 0),
        StR(S1, X4, X3, 0),
        CmpRR(X4, X5),
        BCc(E.into(), end),
        AddRI12(X2, X2, 1),
        AddRI12(X3, X3, 1),
        B_(start),
        LblA(end),
      ],
    ];
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn clone_str_x(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x40;
    let id = symbol!(self, caller, COPY2HEAP);
    let tmp_d = Local(Tmp, -0x08);
    let tmp_s = Local(Tmp, -0x10);
    let len = Local(Tmp, -0x18);
    let str_len = self.str_len_x(caller)?;
    let insts = vec![
      vec![
        store(S8, tmp_d, Rdi),
        store(S8, tmp_s, Rsi),
        mov(S8, Rdi, Rcx),
        Call(str_len),
        IncR(Rax),
        store(S8, len, Rax),
        mov(S8, R8, Rax),
      ],
      self.call_alloc_r8_x()?,
      vec![
        load(S8, Rcx, len),
        mov(S8, Rsi, Rdi),
        mov(S8, Rdi, Rax),
        DF(false),
        Repeat(S1, RepE, MovS),
        load(S8, Rdi, tmp_d),
        load(S8, Rsi, tmp_s),
      ],
    ];
    self.link_func_x(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn get_int_to_str_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
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
    let value = Local(Tmp, -0x08).v_rq();
    let count = Local(Tmp, -0x10).v_rd();
    let is_neg = Local(Tmp, -0x11).v_rb();
    let clone_str = self.clone_str_a(id)?;
    let insts = vec![
      store_a(value, X9, X0)?,
      store_a(is_neg, X9, Xzr)?,
      load_imm_a(X1, 1),
      store_a(count, X9, X1)?,
      load_a(X1, value)?,
      vec![CmpRR(X1, Xzr), BCc(G.into(), positive), BCc(Ne.into(), negative)],
      load_a(X1, count)?,
      vec![AddRI12(X1, X1, 1)],
      store_a(count, X9, X1)?,
      vec![B_(count_end), LblA(negative)],
      load_imm_a(X1, bool2byte(true).into()),
      store_a(is_neg, X9, X1)?,
      load_a(X1, value)?,
      vec![SubR3(X2, Xzr, X1), CmpRR(X2, X1), BCc(E.into(), i64_min)],
      store_a(value, X9, X2)?,
      load_a(X1, count)?,
      vec![AddRI12(X1, X1, 1)],
      store_a(count, X9, X1)?,
      vec![B_(positive), LblA(i64_min)],
      load_a(X0, self.global_str(i64::MIN.to_string()))?,
      vec![Bl(clone_str), B_(epilogue), LblA(positive)],
      load_a(X1, value)?,
      vec![LblA(count_start)],
      load_imm_a(X2, 10),
      vec![SDivR3(X3, X1, X2), MovRR(X1, X3)],
      load_a(X5, count)?,
      vec![AddRI12(X5, X5, 1)],
      store_a(count, X9, X5)?,
      vec![CmpRR(X1, Xzr), BCc(Ne.into(), count_start), LblA(count_end)],
      load_a(X0, count)?,
      self.call_alloc_x0_a()?,
      vec![MovRR(X1, X0)],
      load_a(X2, count)?,
      vec![AddR3(X1, X1, X2), SubRI12(X1, X1, 1), StR(S1, Xzr, X1, 0)],
      load_a(X2, value)?,
      vec![LblA(write_start)],
      load_imm_a(X3, 10),
      vec![SDivR3(X4, X2, X3), MulR3(X5, X4, X3), SubR3(X5, X2, X5)],
      load_imm_a(X6, i64::from(b'0')),
      vec![
        AddR3(X5, X5, X6),
        SubRI12(X1, X1, 1),
        StR(S1, X5, X1, 0),
        MovRR(X2, X4),
        CmpRR(X2, Xzr),
        BCc(Ne.into(), write_start),
      ],
      load_a(X2, is_neg)?,
      vec![CmpRR(X2, Xzr), BCc(E.into(), write_positive)],
      load_imm_a(X2, i64::from(b'-')),
      vec![SubRI12(X1, X1, 1), StR(S1, X2, X1, 0), LblA(write_positive)],
      vec![MovRR(X0, X1), LblA(epilogue)],
    ];
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn get_int_to_str_x(&mut self, caller: LabelId) -> ErrOR<LabelId> {
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
    let i64_min_str = self.global_str(i64::MIN.to_string());
    let clone_str = self.clone_str_x(id)?;
    let insts = vec![
      vec![
        store(S8, tmp_s, Rsi),
        store(S8, tmp_b, Rbx),
        mov(S8, Rsi, Rcx),
        MovMIb(Mem(is_neg), bool2byte(false)),
        MovMId(Mem(count), 1),
        m8i(Cmp, Rsi, 0),
        JCc(G, positive),
        JCc(Ne, negative),
        IncMd(count),
        Jmp(count_end),
        LblX(negative),
        MovMIb(Mem(is_neg), bool2byte(true)),
        Unary(S8, Neg, Rsi),
        RR(S8, Cmp, Rsi, Rcx),
        JCc(E, i64_min),
        IncMd(count),
        Jmp(positive),
        LblX(i64_min),
      ],
      load_x(Rcx, i64_min_str)?,
      vec![
        Call(clone_str),
        Jmp(epilogue),
        LblX(positive),
        mov(S8, Rax, Rsi),
        LblX(count_start),
        MovMId(Reg(Rbx), 10),
        Cqo,
        IDivR(Rbx),
        IncMd(count),
        TestRR(S8, Rax),
        JCc(Ne, count_start),
        LblX(count_end),
        load(S4, R8, count),
      ],
      self.call_alloc_r8_x()?,
      vec![
        mov(S8, Rcx, Rax),
        load(S4, Rax, count),
        RR(S8, Add, Rcx, Rax),
        Clear(Rdx),
        DecR(Rcx),
        store(S1, Ref(Rcx), Rdx),
        mov(S8, Rax, Rsi),
        LblX(write_start),
        MovMId(Reg(Rbx), 10),
        Cqo,
        IDivR(Rbx),
        m8i(Add, Rdx, i32::from(b'0')),
        DecR(Rcx),
        store(S1, Ref(Rcx), Rdx),
        TestRR(S8, Rax),
        JCc(Ne, write_start),
        load(S1, Rax, is_neg),
        TestRR(S1, Rax),
        JCc(E, write_positive),
        MovMIb(Reg(Rdx), b'-'),
        DecR(Rcx),
        store(S1, Ref(Rcx), Rdx),
        LblX(write_positive),
        mov(S8, Rax, Rcx),
        LblX(epilogue),
        load(S8, Rsi, tmp_s),
        load(S8, Rbx, tmp_b),
      ],
    ];
    self.link_func_x(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn get_utf8_slice_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x50;
    let id = symbol!(self, caller, UTF8_SLICE);
    let slice = Local(Tmp, -0x08).v_rq();
    let start = Local(Tmp, -0x10).v_rq();
    let end = Local(Tmp, -0x18).v_rq();
    let char_len = Local(Tmp, -0x20).v_rq();
    let start_b = Local(Tmp, -0x28).v_rq();
    let end_b = Local(Tmp, -0x30).v_rq();
    let slice_len = Local(Tmp, -0x38).v_rq();
    let start_is_positive = self.id();
    let end_is_positive = self.id();
    let abort = self.id();
    let epilogue = self.id();
    let copy_loop = self.id();
    let copy_done = self.id();
    let search_slice = self.id();
    let insts = vec![
      store_a(slice, X9, X0)?,
      store_a(start, X9, X1)?,
      store_a(end, X9, X2)?,
      load_a(X0, slice)?,
      load_imm_a(X1, 0),
      load_imm_a(X2, -1),
      vec![Bl(search_slice)],
      store_a(char_len, X9, X5)?,
      load_a(X1, start)?,
      load_a(X11, char_len)?,
      vec![
        CmpRR(X1, Xzr),
        BCc(Ge.into(), start_is_positive),
        AddR3(X1, X1, X11),
        LblA(start_is_positive),
      ],
      store_a(start, X9, X1)?,
      load_a(X2, end)?,
      load_a(X11, char_len)?,
      vec![
        CmpRR(X2, Xzr),
        BCc(Ge.into(), end_is_positive),
        AddR3(X2, X2, X11),
        LblA(end_is_positive),
      ],
      store_a(end, X9, X2)?,
      load_a(X1, start)?,
      load_a(X2, end)?,
      vec![CmpRR(X1, X2), BCc(Ge.into(), abort)],
      load_a(X0, slice)?,
      load_a(X1, start)?,
      load_a(X2, end)?,
      vec![Bl(search_slice), CmpRR(X0, Xzr), BCc(E.into(), abort)],
      store_a(start_b, X9, X3)?,
      store_a(end_b, X9, X4)?,
      load_a(X0, slice)?,
      load_a(X3, start_b)?,
      vec![AddR3(X0, X0, X3)],
      store_a(slice, X9, X0)?,
      load_a(X4, end_b)?,
      load_a(X3, start_b)?,
      vec![SubR3(X4, X4, X3)],
      store_a(slice_len, X9, X4)?,
      load_a(X0, slice_len)?,
      vec![AddRI12(X0, X0, 1)],
      self.call_alloc_x0_a()?,
      vec![MovRR(X1, X0)],
      load_a(X2, slice)?,
      load_a(X3, slice_len)?,
      vec![
        LblA(copy_loop),
        CmpRR(X3, Xzr),
        BCc(E.into(), copy_done),
        LdR(S1, X4, X2, 0),
        StR(S1, X4, X1, 0),
        AddRI12(X2, X2, 1),
        AddRI12(X1, X1, 1),
        SubRI12(X3, X3, 1),
        B_(copy_loop),
        LblA(copy_done),
        StR(S1, Xzr, X1, 0),
        B_(epilogue),
        LblA(abort),
      ],
      load_imm_a(X0, 1),
      self.call_alloc_x0_a()?,
      vec![StR(S1, Xzr, X0, 0), LblA(epilogue)],
    ];
    self.link_func_a(id, insts, SIZE);
    let loop_start = self.id();
    let count = self.id();
    let done = self.id();
    let abort_count = self.id();
    let skip_end_check = self.id();
    let search_return = self.id();
    let search_success = self.id();
    let search_fail = self.id();
    let search_insts = vec![
      vec![
        MovRR(X3, Xzr),
        MovRR(X4, Xzr),
        MovRR(X5, Xzr),
        LblA(loop_start),
        AddR3(X7, X0, X4),
        LdR(S1, X6, X7, 0),
        CmpRR(X6, Xzr),
        BCc(E.into(), abort_count),
        MovRR(X7, X6),
      ],
      load_imm_a(X8, 0xC0),
      vec![AndR3(X7, X7, X8)],
      load_imm_a(X8, 0x80),
      vec![
        CmpRR(X7, X8),
        BCc(E.into(), count),
        CmpRR(X5, X1),
        CSel(X3, X4, X3, E.into()),
        MovRR(X7, Xzr),
        SubRI12(X7, X7, 1),
        CmpRR(X2, X7),
        BCc(E.into(), skip_end_check),
        CmpRR(X5, X2),
        BCc(E.into(), done),
        LblA(skip_end_check),
        AddRI12(X5, X5, 1),
        LblA(count),
        AddRI12(X4, X4, 1),
        B_(loop_start),
        LblA(done),
        MovRR(X0, Xzr),
        AddRI12(X0, X0, 1),
        B_(search_return),
        LblA(abort_count),
        CmpRR(X2, X5),
        BCc(E.into(), search_success),
        B_(search_fail),
        LblA(search_success),
        MovRR(X0, Xzr),
        AddRI12(X0, X0, 1),
        B_(search_return),
        LblA(search_fail),
        MovRR(X0, Xzr),
        LblA(search_return),
      ],
    ];
    self.link_func_a(search_slice, search_insts, 0x20);
    self.use_func(id, search_slice);
    Ok(id)
  }
  #[expect(clippy::too_many_lines)]
  pub(crate) fn get_utf8_slice_x(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x50;
    let id = symbol!(self, caller, UTF8_SLICE);
    let tmp_d = Local(Tmp, -0x08);
    let tmp_s = Local(Tmp, -0x10);
    let tmp_b = Local(Tmp, -0x18);
    let slice = Local(Tmp, -0x20);
    let slice_len = Local(Tmp, -0x28);
    let abort = self.id();
    let epilogue = self.id();
    let start_is_positive = self.id();
    let end_is_positive = self.id();
    let search_slice = self.id();
    let insts = vec![
      vec![
        store(S8, tmp_b, Rbx),
        store(S8, tmp_d, Rdx),
        store(S8, tmp_s, R8),
        store(S8, slice, Rcx),
        Clear(Rdx),
        Clear(R8),
        DecR(R8),
        Call(search_slice),
        load(S8, Rdx, tmp_d),
        load(S8, R8, tmp_s),
        store(S8, tmp_d, Rdi),
        store(S8, tmp_s, Rsi),
        m8i(Cmp, Rdx, 0),
        JCc(Ge, start_is_positive),
        RR(S8, Add, Rdx, R11),
        LblX(start_is_positive),
        m8i(Cmp, R8, 0),
        JCc(Ge, end_is_positive),
        RR(S8, Add, R8, R11),
        LblX(end_is_positive),
        RR(S8, Cmp, Rdx, R8),
        JCc(Ge, abort),
        load(S8, Rcx, slice),
        Call(search_slice),
        TestRR(S1, Rax),
        JCc(E, abort),
        load(S8, Rax, slice),
        RR(S8, Add, Rax, R9),
        store(S8, slice, Rax),
        mov(S8, R8, R10),
        RR(S8, Sub, R8, R9),
        store(S8, slice_len, R8),
        IncR(R8),
      ],
      self.call_alloc_r8_x()?,
      vec![
        load(S8, Rcx, slice_len),
        mov(S8, Rdi, Rax),
        load(S8, Rsi, slice),
        DF(false),
        Repeat(S1, RepE, MovS),
        Jmp(epilogue),
        LblX(abort),
        MovMId(Reg(R8), 1),
      ],
      self.call_alloc_r8_x()?,
      vec![LblX(epilogue), load(S8, Rdi, tmp_d), load(S8, Rsi, tmp_s), load(S8, Rbx, tmp_b)],
    ];
    self.link_func_x(id, insts, SIZE);
    self.use_func(id, search_slice);
    let start = self.id();
    let done = self.id();
    let abort_count = self.id();
    let count = self.id();
    let tmp_12 = Local(Tmp, -8);
    let search_insts = vec![
      store(S8, tmp_12, R12),
      Clear(Rax),
      Clear(R9),
      Clear(R10),
      Clear(R11),
      LblX(start),
      load(S1, R12, SibDisp(Sib { base: Rcx, index: R10, scale: S1 }, Disp::Zero)),
      mov(S1, Rax, R12),
      MovMIb(Reg(Rbx), 0xC0),
      RR(S1, And, Rax, Rbx),
      MovMIb(Reg(Rbx), 0x80),
      RR(S1, Cmp, Rax, Rbx),
      JCc(E, count),
      RR(S8, Cmp, R11, Rdx),
      CMovCc(E, R9, R10),
      RR(S8, Cmp, R11, R8),
      JCc(E, done),
      TestRR(S1, R12),
      JCc(E, abort_count),
      IncR(R11),
      LblX(count),
      IncR(R10),
      Jmp(start),
      LblX(done),
      Clear(Rax),
      IncR(Rax),
      LblX(abort_count),
      load(S8, R12, tmp_12),
    ];
    self.link_func_x(search_slice, vec![search_insts], SIZE);
    Ok(id)
  }
  pub(crate) fn str_chars_len_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x10;
    let id = symbol!(self, caller, STR_CHARS_LEN);
    let start = self.id();
    let count = self.id();
    let epilogue = self.id();
    let insts = vec![
      vec![
        MovRR(X1, Xzr),
        MovRR(X2, Xzr),
        LblA(start),
        AddR3(X3, X0, X1),
        LdR(S1, X4, X3, 0),
        CmpRR(X4, Xzr),
        BCc(E.into(), epilogue),
        MovRR(X5, X4),
      ],
      load_imm_a(X6, 0xC0),
      vec![AndR3(X5, X5, X6)],
      load_imm_a(X6, 0x80),
      vec![
        CmpRR(X5, X6),
        BCc(E.into(), count),
        AddRI12(X2, X2, 1),
        LblA(count),
        AddRI12(X1, X1, 1),
        B_(start),
        LblA(epilogue),
        MovRR(X0, X2),
      ],
    ];
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn str_chars_len_x(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x30;
    let id = symbol!(self, caller, STR_CHARS_LEN);
    let start = self.id();
    let count = self.id();
    let epilogue = self.id();
    let insts = vec![
      Clear(Rax),
      Clear(R9),
      Clear(R10),
      Clear(R11),
      LblX(start),
      load(S1, R11, SibDisp(Sib { base: Rcx, index: R10, scale: S1 }, Disp::Zero)),
      TestRR(S1, R11),
      JCc(E, epilogue),
      MovMIb(Reg(Rdx), 0xC0),
      RR(S1, And, R11, Rdx),
      MovMIb(Reg(Rdx), 0x80),
      RR(S1, Cmp, R11, Rdx),
      JCc(E, count),
      IncR(Rax),
      LblX(count),
      IncR(R10),
      Jmp(start),
      LblX(epilogue),
    ];
    self.link_func_x(id, vec![insts], SIZE);
    Ok(id)
  }
  pub(crate) fn str_eq_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x20;
    let id = symbol!(self, caller, STR_EQ);
    let epilogue = self.id();
    let case_true = self.id();
    let start = self.id();
    let insts = vec![
      MovRR(X4, Xzr),
      LblA(start),
      LdR(S1, X2, X0, 0),
      LdR(S1, X3, X1, 0),
      CmpRR(X2, X3),
      BCc(Ne.into(), epilogue),
      TstRb(X2),
      BCc(E.into(), case_true),
      AddRI12(X0, X0, 1),
      AddRI12(X1, X1, 1),
      B_(start),
      LblA(case_true),
      MovZ(X4, u16::from(bool2byte(true)), Lsl0),
      LblA(epilogue),
      MovRR(X0, X4),
    ];
    self.link_func_a(id, vec![insts], SIZE);
    Ok(id)
  }
  pub(crate) fn str_eq_x(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x20;
    let id = symbol!(self, caller, STR_EQ);
    let epilogue = self.id();
    let case_true = self.id();
    let start = self.id();
    let insts = vec![
      Clear(Rax),
      LblX(start),
      load(S1, R8, Ref(Rcx)),
      load(S1, R9, Ref(Rdx)),
      RR(S1, Cmp, R8, R9),
      JCc(Ne, epilogue),
      TestRR(S1, R8),
      JCc(E, case_true),
      IncR(Rcx),
      IncR(Rdx),
      Jmp(start),
      LblX(case_true),
      MovMIb(Reg(Rax), bool2byte(true)),
      LblX(epilogue),
    ];
    self.link_func_x(id, vec![insts], SIZE);
    Ok(id)
  }
  pub(crate) fn str_len_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x10;
    let id = symbol!(self, caller, STR_LEN);
    let start = self.id();
    let insts = vec![
      MovRR(X1, X0),
      LblA(start),
      LdR(S1, X2, X0, 0),
      AddRI12(X0, X0, 1),
      CmpRI12(X2, 0),
      BCc(Ne.into(), start),
      SubR3(X0, X0, X1),
      SubRI12(X0, X0, 1),
    ];
    self.link_func_a(id, vec![insts], SIZE);
    Ok(id)
  }
  pub(crate) fn str_len_x(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x30;
    let id = symbol!(self, caller, STR_LEN);
    let tmp_d = Local(Tmp, -0x8);
    let insts = vec![
      store(S8, tmp_d, Rdi),
      mov(S8, Rdx, Rcx),
      mov(S8, Rdi, Rdx),
      Clear(Rcx),
      DecR(Rcx),
      Clear(Rax),
      DF(false),
      Repeat(S1, RepNe, ScaS),
      RR(S8, Sub, Rdi, Rdx),
      DecR(Rdi),
      mov(S8, Rax, Rdi),
      load(S8, Rdi, tmp_d),
    ];
    self.link_func_x(id, vec![insts], SIZE);
    Ok(id)
  }
}
