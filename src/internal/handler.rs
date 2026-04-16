use crate::prelude::*;
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Handlers {
  pub ctrl_c: LabelId,
  pub err: Option<LabelId>,
  pub seh: LabelId,
  pub win: LabelId,
}
impl Jsonpiler {
  pub(crate) fn ctrl_c_handler(&mut self, caller: LabelId) -> ErrOR<()> {
    const SIZE: i32 = 0x30;
    self.use_function(caller, self.handlers.ctrl_c);
    let code = Local(Tmp, -0x8);
    let print_e = self.get_print_e(self.handlers.ctrl_c)?;
    let new_line = Global(self.global_str("\n| "));
    let mut insts = vec![];
    extend!(
      insts,
      [
        mov_q(code, Rcx),
        LeaRM(Rcx, Global(self.global_str(SYSTEM_EXIT))),
        Call(print_e),
        LeaRM(Rcx, new_line),
        Call(print_e),
        mov_q(Rax, code),
        LeaRM(Rcx, Global(self.global_str("Ctrl+C"))),
      ],
      self.ctrl_c_match(1, "Ctrl+Break"),
      self.ctrl_c_match(2, "Console closed"),
      self.ctrl_c_match(5, "User logged off"),
      self.ctrl_c_match(6, "System shutdown"),
      [Call(print_e), LeaRM(Rcx, Global(self.global_str(ERR_END))), Call(print_e), Clear(Rax)]
    );
    //    self.use_function(self.root_id[0].0, id);
    self.link_function(self.handlers.ctrl_c, &insts, SIZE);
    Ok(())
  }
  pub(crate) fn ctrl_c_match(&mut self, err_code: u32, err: &'static str) -> Vec<Inst> {
    vec![
      LeaRM(Rdx, Global(self.global_str(err))),
      mov_d(R8, err_code),
      LogicRbRb(Cmp, Rax, R8),
      CMovCc(E, Rcx, Rdx),
    ]
  }
  pub(crate) fn custom_err(
    &mut self,
    err: RuntimeErr,
    args: Option<Bind<String>>,
    pos: Position,
    caller: LabelId,
  ) -> ErrOR<LabelId> {
    const SIZE: i32 = 0;
    if self.release {
      return self.hidden_handler(caller);
    }
    let id = self.id();
    self.use_function(caller, id);
    let (file, l_c, code, carets) =
      self.parsers[pos.file as usize].err_info(pos, &self.parsers[0].val.file);
    let insts = &[
      LeaRM(Rcx, Global(self.global_str(format!("{err}")))),
      self.mov_str(Rdx, args.unwrap_or(Lit(String::new()))),
      LeaRM(R8, Global(self.global_str(file))),
      LeaRM(R9, Global(self.global_str(l_c))),
      LeaRM(Rax, Global(self.global_str(code))),
      mov_q(Args(5), Rax),
      LeaRM(Rax, Global(self.global_str(carets))),
      mov_q(Args(6), Rax),
      Call(self.err_handler(id)?),
      mov_d(Rcx, 1),
    ];
    self.link_not_return(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn err_handler(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x30;
    if let Some(id) = self.handlers.err {
      self.use_function(caller, id);
      return Ok(id);
    }
    let id = self.id();
    self.use_function(caller, id);
    self.handlers.err = Some(id);
    let err = Local(Long, -0x08);
    let args = Local(Long, -0x10);
    let file = Local(Long, -0x18);
    let l_c = Local(Long, -0x20);
    let code = Local(Long, -0x28);
    let carets = Local(Long, -0x30);
    let print_e = self.get_print_e(id)?;
    let new_line = Global(self.global_str("\n| "));
    let err_separate = Global(self.global_str(ERR_SEPARATE));
    let insts = &[
      mov_q(err, Rcx),
      mov_q(args, Rdx),
      mov_q(file, R8),
      mov_q(l_c, R9),
      mov_q(Rax, Local(Long, 4 * 8 + 16)),
      mov_q(code, Rax),
      mov_q(Rax, Local(Long, 5 * 8 + 16)),
      mov_q(carets, Rax),
      LeaRM(Rcx, Global(self.global_str(RUNTIME_ERR))),
      Call(print_e),
      LeaRM(Rcx, new_line),
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
    ];
    self.link_function(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn hidden_handler(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0;
    let id = self.id();
    self.use_function(caller, id);
    let print_e = self.get_print_e(id)?;
    let hidden_err = Global(self.global_str(HIDDEN_ERROR));
    self.link_not_return(id, &[LeaRM(Rcx, hidden_err), Call(print_e), mov_d(Rcx, 1)], SIZE);
    Ok(id)
  }
  pub(crate) fn seh_handler(&mut self, caller: LabelId) -> ErrOR<()> {
    const SIZE: i32 = 0x20;
    self.use_function(caller, self.handlers.seh);
    let matched = self.id();
    let epilogue = self.id();
    let std_e = Global(self.symbols[STD_E]);
    let exit_process = self.import(KERNEL32, "ExitProcess");
    let write_file = self.import(KERNEL32, "WriteFile");
    let tmp = Local(Tmp, -0x8);
    let mut insts = vec![];
    extend!(
      insts,
      [mov_q(Rbx, Ref(Rcx))],
      self.write_err_msg(&make_header(INTERNAL_ERR), tmp)?,
      self.write_err_msg("\n| ", tmp)?,
      self.seh_match(0xFD, STACK_OVERFLOW, matched)?,
      self.seh_match(5, ACCESS_VIOLATION, matched)?,
      self.seh_match(0x94, ZERO_DIVISION, matched)?,
      [
        LeaRM(Rdi, Global(self.global_str(EXCEPTION_OCCURRED))),
        mov_d(Rsi, len_u32(EXCEPTION_OCCURRED.as_bytes())?),
        LeaRM(R13, Global(self.global_str("R0000"))),
        Lbl(matched),
        mov_q(Rcx, std_e),
        mov_q(Rdx, Rdi),
        mov_d(R8, Rsi),
        LeaRM(R9, tmp),
        Clear(Rax),
        mov_q(Args(5), Rax),
        CallApi(write_file),
      ],
      self.write_err_msg(ERR_END, tmp)?,
      self.write_err_msg(ISSUE, tmp)?,
      [
        mov_q(Rcx, std_e),
        mov_q(Rdx, R13),
        mov_d(R8, 5),
        LeaRM(R9, tmp),
        Clear(Rax),
        mov_q(Args(5), Rax),
        CallApi(write_file),
      ],
      self.write_err_msg("`\n", tmp)?,
      [Lbl(epilogue), mov_q(Rcx, Rbx), CallApi(exit_process)]
    );
    self.link_function_no_seh(self.handlers.seh, &insts, SIZE);
    Ok(())
  }
  pub(crate) fn seh_match(
    &mut self,
    err_code: u32,
    err: &'static str,
    matched: LabelId,
  ) -> ErrOR<Vec<Inst>> {
    Ok(vec![
      mov_d(Rax, 0xC000_0000 | err_code),
      LeaRM(Rdi, Global(self.global_str(err))),
      mov_d(Rsi, len_u32(err.as_bytes())?),
      LeaRM(R13, Global(self.global_str(format!("R{err_code:04X}")))),
      LogicRR(Cmp, Rbx, Rax),
      JCc(E, matched),
    ])
  }
  pub(crate) fn win_handler(&mut self, caller: LabelId) -> ErrOR<()> {
    const SIZE: i32 = 0x70;
    self.use_function(caller, self.handlers.win);
    let exit = self.id();
    let format_msg = self.import(KERNEL32, "FormatMessageW");
    let get_last_err = self.import(KERNEL32, "GetLastError");
    let local_free = self.import(KERNEL32, "LocalFree");
    let print_e = self.get_print_e(self.handlers.win)?;
    let u16_to_8 = self.get_u16_to_8(self.handlers.win)?;
    let digit = self.id();
    let hex_loop = self.id();
    let store = self.id();
    let tmp = Local(Long, -0x8);
    let msg = Local(Long, -0x10);
    let multi_byte = Local(Long, -0x18);
    let buf17 = Local(Long, -0x20);
    let mut insts = vec![];
    extend!(
      insts,
      [
        mov_q(Rbp, Rsp),
        SubRId(Rsp, SIZE),
        CallApi(get_last_err),
        mov_q(Rdi, Rax),
        mov_d(Rcx, 0x1300),
        Clear(Rdx),
        mov_q(R8, Rdi),
        Clear(R9),
        LeaRM(Rax, msg),
        mov_q(Args(5), Rax),
        mov_q(Args(6), Rdx),
        mov_q(Args(7), Rdx),
        CallApi(format_msg),
        LogicRR(Test, Rax, Rax),
        JCc(E, exit),
        mov_q(Rcx, msg),
        mov_d(Rdx, 65001),
        Call(u16_to_8),
        mov_q(multi_byte, Rax),
      ],
      self.write_err_msg(&make_header(INTERNAL_ERR), tmp)?,
      self.write_err_msg(WIN_API_ERR, tmp)?,
      [mov_q(Rcx, multi_byte), Call(print_e)],
      self.write_err_msg(ERR_END, tmp)?,
      self.write_err_msg(ISSUE, tmp)?,
      self.write_err_msg("W", tmp)?,
      [
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
        mov_d(Rcx, 10),
        LogicRR(Cmp, Rdx, Rcx),
        JCc(B, digit),
        mov_d(Rcx, 0x37),
        AddRR(Rdx, Rcx),
        Jmp(store),
        Lbl(digit),
        mov_d(Rcx, 0x30),
        AddRR(Rdx, Rcx),
        Lbl(store),
        mov_b(Ref(Rbx), Rdx),
        ShiftR(Shr, Rax, Shift::Ib(4)),
        DecR(R8),
        JCc(Ne, hex_loop),
        LeaRM(Rcx, buf17),
        Call(print_e),
      ],
      self.write_err_msg("`\n", tmp)?,
      [Lbl(exit), mov_q(Rcx, msg), CallApi(local_free), mov_q(Rcx, Rdi)]
    );
    self.link_not_return(self.handlers.win, &insts, SIZE);
    Ok(())
  }
  pub(crate) fn write_err_msg(&mut self, text: &str, tmp: Address) -> ErrOR<Vec<Inst>> {
    let write_file = self.import(KERNEL32, "WriteFile");
    Ok(vec![
      mov_q(Rcx, Global(self.symbols[STD_E])),
      LeaRM(Rdx, Global(self.global_str(text))),
      mov_d(R8, len_u32(text.as_bytes())?),
      LeaRM(R9, tmp),
      Clear(Rax),
      mov_q(Args(5), Rax),
      CallApi(write_file),
    ])
  }
}
