use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn ctrl_c_handler(&mut self) -> ErrOR<[Inst; 33]> {
    const SIZE: u32 = 0x30;
    let id = self.symbols[CTRL_C_HANDLER];
    let matched = self.id();
    let match_end = self.id();
    let end = self.id();
    let code = Local(Tmp, -0x8);
    let print_e = self.get_print_e()?;
    self.data_insts.push(Seh(id, end, SIZE));
    Ok([
      Lbl(id),
      Push(Rbp),
      mov_q(Rbp, Rsp),
      SubRId(Rsp, SIZE),
      mov_q(code, Rcx),
      LeaRM(Rcx, Global(self.global_str(ABORTED_ERROR))),
      Call(print_e),
      mov_q(Rax, code),
      LeaRM(Rcx, Global(self.global_str("Ctrl+C"))),
      LeaRM(Rdx, Global(self.global_str("Ctrl+Break"))),
      CmpRIb(Rax, 1),
      JCc(E, matched),
      LeaRM(Rdx, Global(self.global_str("Console closed"))),
      CmpRIb(Rax, 2),
      JCc(E, matched),
      LeaRM(Rdx, Global(self.global_str("User logged off"))),
      CmpRIb(Rax, 5),
      JCc(E, matched),
      LeaRM(Rdx, Global(self.global_str("System shutdown"))),
      CmpRIb(Rax, 6),
      CMovCc(E, Rcx, Rdx),
      Jmp(match_end),
      Lbl(matched),
      CMovCc(E, Rcx, Rdx),
      Lbl(match_end),
      Call(print_e),
      LeaRM(Rcx, Global(self.global_str(ERR_END))),
      Call(print_e),
      Clear(Rax),
      mov_q(Rsp, Rbp),
      Pop(Rbp),
      Custom(RET),
      Lbl(end),
    ])
  }
  pub(crate) fn custom_err(
    &mut self,
    err: &'static str,
    args: Bind<String>,
    pos: Position,
  ) -> ErrOR<u32> {
    let id = self.id();
    if self.release {
      let exit_process = self.import(KERNEL32, "ExitProcess")?;
      let print_e = self.get_print_e()?;
      let hidden_err = Global(self.global_str(HIDDEN_ERROR));
      self.insts.extend_from_slice(&[
        Lbl(id),
        LeaRM(Rcx, hidden_err),
        Call(print_e),
        Clear(Rcx),
        IncR(Rcx),
        CallApi(exit_process),
      ]);
      return Ok(id);
    }
    let (file, l_c, code, carets) = self.err_info(pos);
    let slice = [
      Lbl(id),
      LeaRM(Rcx, Global(self.global_str(err))),
      self.mov_str(Rdx, args),
      LeaRM(R8, Global(self.global_str(file))),
      LeaRM(R9, Global(self.global_str(l_c))),
      LeaRM(Rax, Global(self.global_str(code))),
      mov_q(Args(5), Rax),
      LeaRM(Rax, Global(self.global_str(carets))),
      mov_q(Args(6), Rax),
      Call(self.err_handler()?),
    ];
    self.insts.extend_from_slice(&slice);
    Ok(id)
  }
  pub(crate) fn err_handler(&mut self) -> ErrOR<u32> {
    const SIZE: u32 = 0x30;
    let id = symbol!(self, ERR_HANDLER);
    let end = self.id();
    let exit_process = self.import(KERNEL32, "ExitProcess")?;
    let err = Local(Long, -0x08);
    let args = Local(Long, -0x10);
    let file = Local(Long, -0x18);
    let l_c = Local(Long, -0x20);
    let code = Local(Long, -0x28);
    let carets = Local(Long, -0x30);
    let print_e = self.get_print_e()?;
    self.data_insts.push(Seh(id, end, SIZE));
    let err_separate = Global(self.global_str(ERR_SEPARATE));
    let slice = [
      Lbl(id),
      Push(Rbp),
      mov_q(Rbp, Rsp),
      SubRId(Rsp, SIZE),
      mov_q(err, Rcx),
      mov_q(args, Rdx),
      mov_q(file, R8),
      mov_q(l_c, R9),
      mov_q(Rax, Local(Long, 4 * 8 + 16)),
      mov_q(code, Rax),
      mov_q(Rax, Local(Long, 5 * 8 + 16)),
      mov_q(carets, Rax),
      LeaRM(Rcx, Global(self.global_str(RUNTIME_ERROR))),
      Call(print_e),
      mov_q(Rcx, err),
      Call(print_e),
      mov_q(Rcx, args),
      Call(print_e),
      LeaRM(Rcx, err_separate),
      Call(print_e),
      mov_q(Rcx, file),
      Call(print_e),
      mov_q(Rcx, l_c),
      Call(print_e),
      LeaRM(Rcx, err_separate),
      Call(print_e),
      mov_q(Rcx, code),
      Call(print_e),
      LeaRM(Rcx, Global(self.global_str("| "))),
      Call(print_e),
      mov_q(Rcx, carets),
      Call(print_e),
      LeaRM(Rcx, Global(self.global_str(ERR_END))),
      Call(print_e),
      Clear(Rcx),
      IncR(Rcx),
      CallApi(exit_process),
      Lbl(end),
    ];
    self.insts.extend_from_slice(&slice);
    Ok(id)
  }
  pub(crate) fn seh_handler(&mut self) -> ErrOR<Vec<Inst>> {
    const SIZE: u32 = 0x20;
    let id = self.symbols[SEH_HANDLER];
    let exception_matched = self.id();
    let exception_end = self.id();
    let exit = self.id();
    let std_e = Global(self.symbols[STD_E]);
    let exit_process = self.import(KERNEL32, "ExitProcess")?;
    let write_file = self.import(KERNEL32, "WriteFile")?;
    let tmp = Local(Tmp, -0x8);
    Ok(vec![
      Lbl(id),
      Push(Rbp),
      mov_q(Rbp, Rsp),
      SubRId(Rsp, SIZE),
      mov_q(Rbx, Ref(Rcx)),
      mov_q(Rcx, std_e),
      LeaRM(Rdx, Global(self.global_str(INTERNAL_ERROR))),
      mov_d(R8, u32::try_from(INTERNAL_ERROR.len())?),
      LeaRM(R9, tmp),
      Clear(Rax),
      mov_q(Args(5), Rax),
      CallApi(write_file),
      mov_d(Rax, 0xC000_00FD),
      LeaRM(Rcx, Global(self.global_str(STACK_OVERFLOW))),
      mov_d(Rdx, u32::try_from(STACK_OVERFLOW.len())?),
      LeaRM(R12, Global(self.global_str("R00FD"))),
      LogicRR(Cmp, Rbx, Rax),
      JCc(E, exception_matched),
      mov_d(Rax, 0xC000_0005),
      LeaRM(Rcx, Global(self.global_str(ACCESS_VIOLATION))),
      mov_d(Rdx, u32::try_from(ACCESS_VIOLATION.len())?),
      LeaRM(R12, Global(self.global_str("R0005"))),
      LogicRR(Cmp, Rbx, Rax),
      JCc(E, exception_matched),
      mov_d(Rax, 0xC000_0094),
      LeaRM(Rcx, Global(self.global_str(ZERO_DIVISION))),
      mov_d(Rdx, u32::try_from(ZERO_DIVISION.len())?),
      LeaRM(R12, Global(self.global_str("R0094"))),
      LogicRR(Cmp, Rbx, Rax),
      JCc(E, exception_matched),
      LeaRM(Rdi, Global(self.global_str(EXCEPTION_OCCURRED))),
      mov_d(Rsi, u32::try_from(EXCEPTION_OCCURRED.len())?),
      LeaRM(R13, Global(self.global_str("R0000"))),
      Jmp(exception_end),
      Lbl(exception_matched),
      CMovCc(E, Rdi, Rcx),
      CMovCc(E, Rsi, Rdx),
      CMovCc(E, R13, R12),
      Lbl(exception_end),
      mov_q(Rcx, std_e),
      mov_q(Rdx, Rdi),
      mov_d(R8, Rsi),
      LeaRM(R9, tmp),
      Clear(Rax),
      mov_q(Args(5), Rax),
      CallApi(write_file),
      mov_q(Rcx, std_e),
      LeaRM(Rdx, Global(self.global_str(ERR_END))),
      mov_d(R8, u32::try_from(ERR_END.len())?),
      LeaRM(R9, tmp),
      Clear(Rax),
      mov_q(Args(5), Rax),
      CallApi(write_file),
      mov_q(Rcx, std_e),
      LeaRM(Rdx, Global(self.global_str(REPORT_MSG))),
      mov_d(R8, u32::try_from(REPORT_MSG.len())?),
      LeaRM(R9, tmp),
      Clear(Rax),
      mov_q(Args(5), Rax),
      CallApi(write_file),
      mov_q(Rcx, std_e),
      mov_q(Rdx, R13),
      mov_d(R8, 5),
      LeaRM(R9, tmp),
      Clear(Rax),
      mov_q(Args(5), Rax),
      CallApi(write_file),
      mov_q(Rcx, std_e),
      LeaRM(Rdx, Global(self.global_str("`\n"))),
      mov_d(R8, 2),
      LeaRM(R9, tmp),
      Clear(Rax),
      mov_q(Args(5), Rax),
      CallApi(write_file),
      Lbl(exit),
      mov_q(Rcx, Rbx),
      CallApi(exit_process),
    ])
  }
  #[expect(clippy::too_many_lines)]
  pub(crate) fn win_handler(&mut self) -> ErrOR<Vec<Inst>> {
    let id = self.symbols[WIN_HANDLER];
    let exit = self.id();
    let heap = Global(self.symbols[HEAP]);
    let exit_process = self.import(KERNEL32, "ExitProcess")?;
    let format_msg = self.import(KERNEL32, "FormatMessageW")?;
    let to_multi_byte = self.import(KERNEL32, "WideCharToMultiByte")?;
    let get_last_err = self.import(KERNEL32, "GetLastError")?;
    let heap_alloc = self.import(KERNEL32, "HeapAlloc")?;
    let local_free = self.import(KERNEL32, "LocalFree")?;
    let print_e = self.get_print_e()?;
    let digit = self.id();
    let hex_loop = self.id();
    let store = self.id();
    let msg = Local(Long, -8);
    let multi_byte = Local(Long, -0x10);
    let len = Local(Long, -0x18);
    let buf17 = Local(Long, -0x20);
    Ok(vec![
      Lbl(id),
      mov_q(Rbp, Rsp),
      SubRId(Rsp, 0x70),
      CallApi(get_last_err),
      mov_q(Rdi, Rax),
      mov_d(Rcx, 0x1300),
      Clear(Rdx),
      mov_q(R8, Rdi),
      Clear(R9),
      LeaRM(Rax, msg),
      mov_q(Args(5), Rax),
      Clear(Rax),
      mov_q(Args(6), Rax),
      mov_q(Args(7), Rax),
      CallApi(format_msg),
      LogicRR(Test, Rax, Rax),
      JCc(E, exit),
      mov_d(Rcx, 65001),
      Clear(Rdx),
      mov_q(R8, msg),
      mov_d(R9, u32::MAX),
      mov_q(Args(5), Rdx),
      mov_q(Args(6), Rdx),
      mov_q(Args(7), Rdx),
      mov_q(Args(8), Rdx),
      CallApi(to_multi_byte),
      LogicRR(Test, Rax, Rax),
      JCc(E, exit),
      mov_q(len, Rax),
      mov_q(Rcx, heap),
      Clear(Rdx),
      mov_q(R8, len),
      CallApi(heap_alloc),
      LogicRR(Test, Rax, Rax),
      JCc(E, exit),
      mov_q(multi_byte, Rax),
      mov_d(Rcx, 65001),
      Clear(Rdx),
      mov_q(R8, msg),
      mov_d(R9, u32::MAX),
      mov_q(Rax, multi_byte),
      mov_q(Args(5), Rax),
      mov_q(Rax, len),
      mov_q(Args(6), Rax),
      mov_q(Args(7), Rdx),
      mov_q(Args(8), Rdx),
      CallApi(to_multi_byte),
      LogicRR(Test, Rax, Rax),
      JCc(E, exit),
      mov_q(Rcx, multi_byte),
      AddRR(Rax, Rcx),
      DecR(Rax),
      Clear(Rcx),
      mov_b(Ref(Rax), Rcx),
      DecR(Rax),
      mov_b(Ref(Rax), Rcx),
      LeaRM(Rcx, Global(self.global_str(INTERNAL_ERROR))),
      Call(print_e),
      LeaRM(Rcx, Global(self.global_str(WIN_API_ERROR))),
      Call(print_e),
      mov_q(Rcx, multi_byte),
      Call(print_e),
      LeaRM(Rcx, Global(self.global_str(ERR_END))),
      Call(print_e),
      LeaRM(Rcx, Global(self.global_str(REPORT_MSG))),
      Call(print_e),
      LeaRM(Rcx, Global(self.global_str("W"))),
      Call(print_e),
      mov_q(Rax, Rdi),
      LeaRM(Rbx, buf17),
      mov_d(R8, 4),
      AddRR(Rbx, R8),
      Clear(Rcx),
      mov_b(Ref(Rbx), Rcx),
      Lbl(hex_loop),
      DecR(Rbx),
      mov_q(Rdx, Rax),
      mov_d(Rcx, 0xF),
      LogicRR(And, Rdx, Rcx),
      CmpRIb(Rdx, 10),
      JCc(B, digit),
      mov_d(Rcx, 0x37),
      AddRR(Rdx, Rcx),
      Jmp(store),
      Lbl(digit),
      mov_d(Rcx, 0x30),
      AddRR(Rdx, Rcx),
      Lbl(store),
      mov_b(Ref(Rbx), Rdx),
      ShrRIb(Rax, 4),
      DecR(R8),
      JCc(Ne, hex_loop),
      LeaRM(Rcx, buf17),
      Call(print_e),
      LeaRM(Rcx, Global(self.global_str("`\n"))),
      Call(print_e),
      Lbl(exit),
      mov_q(Rcx, msg),
      CallApi(local_free),
      mov_q(Rcx, Rdi),
      CallApi(exit_process),
    ])
  }
}
