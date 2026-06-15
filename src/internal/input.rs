use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn get_input_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x20;
    let id = symbol!(self, caller, INPUT);
    let buffer = Local(Tmp, -0x8).v_rq();
    let read_len = Local(Tmp, -0x10).v_rq();
    let no_trim = self.id();
    let abort = self.id();
    let epilogue = self.id();
    let str_len = self.str_len_a(id)?;
    let clone_str = self.clone_str_a(id)?;
    let insts = vec![
      vec![
        MovRR(X0, Xzr),
        BApi(self.api(LIB_EDIT, "_readline")),
        CmpRR(X0, Xzr),
        BCc(E.into(), abort),
      ],
      store_a(buffer, X9, X0)?,
      vec![Bl(str_len), CmpRR(X0, Xzr), BCc(E.into(), no_trim)],
      store_a(read_len, X9, X0)?,
      load_a(X1, buffer)?,
      load_a(X2, read_len)?,
      vec![SubRI12(X2, X2, 1), AddR3(X1, X1, X2)],
      load_imm_a(X2, i64::from(b'\n')),
      vec![LdR(S1, X3, X1, 0), CmpRR(X3, X2), BCc(Ne.into(), no_trim), StR(S1, Xzr, X1, 0)],
      load_a(X0, read_len)?,
      vec![SubRI12(X0, X0, 1)],
      store_a(read_len, X9, X0)?,
      vec![LblA(no_trim)],
      load_a(X1, buffer)?,
      load_a(X2, read_len)?,
      vec![AddR3(X1, X1, X2), StR(S1, Xzr, X1, 0)],
      load_a(X0, buffer)?,
      load_a(X1, read_len)?,
      vec![AddRI12(X1, X1, 1), Bl(clone_str), B_(epilogue), LblA(abort)],
      load_imm_a(X0, 1),
      self.call_alloc_x0_a()?,
      vec![StR(S1, Xzr, X0, 0), LblA(epilogue)],
    ];
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn get_input_x(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x40;
    const CAPACITY: u32 = 1 << 20;
    let id = symbol!(self, caller, INPUT);
    let buffer = Local(Tmp, -0x8);
    let read_len = Local(Tmp, -0x10);
    let handle_pipe = self.id();
    let handle_stdin = self.id();
    let re_alloc = self.id();
    let epilogue = self.id();
    let u16_to_8 = self.get_u16_to_8(id)?;
    let str_len = self.str_len_x(id)?;
    let std_i = self.get_std_i_x()?;
    let insts = vec![
      vec![MovMId(Reg(R8), CAPACITY)],
      self.call_alloc_r8_x()?,
      vec![
        store(S8, buffer, Rax),
        load(S8, Rcx, Global(std_i)),
        load(S8, Rdx, buffer),
        MovMId(Reg(R8), (CAPACITY >> 1) - 1),
        LeaRM(R9, read_len),
        Clear(Rax),
        store(S8, Args(5), Rax),
        CallApi(self.api(KERNEL32, "ReadConsoleW")),
        TestRR(S8, Rax),
        JCc(Ne, handle_stdin),
        CallApi(self.api(KERNEL32, "GetLastError")),
      ],
      je_pipe(handle_pipe, 1)?,
      je_pipe(handle_pipe, 6)?,
      je_pipe(handle_pipe, 0x57)?,
      vec![
        Jmp(self.handlers.os),
        LblX(handle_pipe),
        load(S8, Rcx, Global(std_i)),
        load(S8, Rdx, buffer),
        MovMId(Reg(R8), CAPACITY - 1),
        LeaRM(R9, read_len),
        Clear(Rax),
        store(S8, Args(5), Rax),
        CallApi(self.api(KERNEL32, "ReadFile")),
        TestRR(S8, Rax),
        JCc(Ne, re_alloc),
        CallApi(self.api(KERNEL32, "GetLastError")),
        m8i(Cmp, Rax, 0x6d),
        JCc(Ne, self.handlers.os),
        Jmp(re_alloc),
        LblX(handle_stdin),
        load(S4, Rcx, read_len),
        ShiftR(Shl, Rcx, Shift::One),
        load(S8, Rax, buffer),
        RR(S8, Add, Rax, Rcx),
        m8i(Sub, Rax, 2),
        MovMIb(Ref(Rax), 0),
        load(S8, Rcx, buffer),
        MovMId(Reg(Rdx), CP_UTF8),
        Call(u16_to_8),
        load(S8, R8, buffer),
        store(S8, buffer, Rax),
      ],
      self.call_free_r8_x()?,
      vec![
        load(S8, Rcx, buffer),
        Call(str_len),
        IncR(Rax),
        store(S4, read_len, Rax),
        load(S8, Rax, buffer),
        Jmp(epilogue),
        LblX(re_alloc),
        load(S8, Rcx, Global(self.get_heap()?)),
        MovMId(Reg(Rdx), 8),
        load(S8, R8, buffer),
        load(S4, R9, read_len),
        IncR(R9),
        CallApi(self.api(KERNEL32, "HeapReAlloc")),
        LblX(epilogue),
      ],
    ];
    self.link_func_x(id, insts, SIZE);
    Ok(id)
  }
}
pub(crate) fn je_pipe(handle_pipe: LabelId, code: u32) -> ErrOR<Vec<X64Inst>> {
  Ok(vec![m8i(Cmp, Rax, i32::try_from(code)?), JCc(E, handle_pipe)])
}
