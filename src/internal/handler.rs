use crate::prelude::*;
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Handlers {
  pub ctrl_c: LabelId,
  pub err: Option<LabelId>,
  pub internal: LabelId,
  pub os: LabelId,
}
impl Jsonpiler {
  pub(crate) fn ctrl_c_handler_a(&mut self, caller: LabelId) -> ErrOR<()> {
    const SIZE: i32 = 0x10;
    let code = Local(Tmp, -0x4).v_rd();
    let err = Local(Tmp, -0x10).v_rq();
    let mut insts = vec![store_a(code, X9, X0)?];
    let binds = [Lit(make_header(SYSTEM_EXIT)), Lit("\n| ".into())];
    let binds2 = [Var(err), Lit(ERR_END.into())];
    insts.extend([
      self.write_mems_a(&binds)?,
      load_a(X3, code)?,
      self.sig_match_a(SIGHUP, "Terminal disconnected")?,
      self.sig_match_a(SIGINT, "Ctrl+C")?,
      self.sig_match_a(SIGQUIT, "Quit")?,
      self.sig_match_a(SIGABRT, "Aborted")?,
      self.sig_match_a(SIGTERM, "Terminated")?,
      store_a(err, X9, X0)?,
      self.write_mems_a(&binds2)?,
      load_a(X0, code)?,
    ]);
    self.use_func(caller, self.handlers.ctrl_c);
    self.link_lbl_a(self.handlers.ctrl_c, insts, SIZE, false);
    Ok(())
  }
  pub(crate) fn ctrl_c_handler_x(&mut self, caller: LabelId) -> ErrOR<()> {
    const SIZE: i32 = 0x30;
    self.use_func(caller, self.handlers.ctrl_c);
    let code = Local(Tmp, -0x8);
    let print_e = self.get_print_e_x(self.handlers.ctrl_c)?;
    let binds = [Lit(make_header(SYSTEM_EXIT)), Lit("\n| ".into())];
    let insts = vec![
      vec![store(S8, code, Rcx)],
      self.write_mems_x(&binds)?,
      vec![load(S8, Rax, code)],
      load_x(Rcx, self.global_str("Ctrl+C"))?,
      self.ctrl_c_match_x(1, "Ctrl+Break")?,
      self.ctrl_c_match_x(2, "Console closed")?,
      self.ctrl_c_match_x(5, "User logged off")?,
      self.ctrl_c_match_x(6, "System shutdown")?,
      vec![Call(print_e)],
      load_x(Rcx, self.global_str(ERR_END))?,
      vec![Call(print_e), Clear(Rax)],
    ];
    self.link_func_x(self.handlers.ctrl_c, insts, SIZE);
    Ok(())
  }
  pub(crate) fn ctrl_c_match_x(&mut self, err_code: u32, err: &'static str) -> ErrOR<Vec<X64Inst>> {
    Ok(
      [
        load_x(Rdx, self.global_str(err))?,
        vec![m4i(Cmp, Rax, i32::try_from(err_code)?), CMovCc(E, Rcx, Rdx)],
      ]
      .concat(),
    )
  }
  pub(crate) fn err_handler_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x30;
    if let Some(id) = self.handlers.err {
      self.use_func(caller, id);
      return Ok(id);
    }
    let id = self.id();
    self.use_func(caller, id);
    self.handlers.err = Some(id);
    let err = Local(Tmp, -0x08).v_rq();
    let args = Local(Tmp, -0x10).v_rq();
    let path = Local(Tmp, -0x18).v_rq();
    let l_c = Local(Tmp, -0x20).v_rq();
    let code = Local(Tmp, -0x28).v_rq();
    let carets = Local(Tmp, -0x30).v_rq();
    let mut insts = [err, args, path, l_c, code, carets]
      .into_iter()
      .zip(A64_ARG_REGS)
      .map(|(arg, reg)| store_a(arg, X9, reg))
      .collect::<ErrOR<Vec<_>>>()?;
    let binds = [
      Lit(make_header(RUNTIME_ERR)),
      Lit("\n| ".into()),
      Var(err),
      Var(args),
      Lit(ERR_SEP.into()),
      Var(path),
      Var(l_c),
      Lit(ERR_SEP.into()),
      Var(code),
      Lit("| ".into()),
      Var(carets),
      Lit(ERR_END.into()),
    ];
    insts.extend([self.write_mems_a(&binds)?, load_imm_a(X0, 1)]);
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn err_handler_x(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x30;
    if let Some(id) = self.handlers.err {
      self.use_func(caller, id);
      return Ok(id);
    }
    let id = self.id();
    self.use_func(caller, id);
    self.handlers.err = Some(id);
    let err = Local(Tmp, -0x08).v_rq();
    let args = Local(Tmp, -0x10).v_rq();
    let path = Local(Tmp, -0x18).v_rq();
    let l_c = Local(Tmp, -0x20).v_rq();
    let code = Local(Tmp, -0x28).v_rq();
    let carets = Local(Tmp, -0x30).v_rq();
    let binds = [
      Lit(make_header(RUNTIME_ERR)),
      Lit("\n| ".into()),
      Var(err),
      Var(args),
      Lit(ERR_SEP.into()),
      Var(path),
      Var(l_c),
      Lit(ERR_SEP.into()),
      Var(code),
      Lit("| ".into()),
      Var(carets),
      Lit(ERR_END.into()),
    ];
    let insts = vec![
      vec![
        store(S8, err.0, Rcx),
        store(S8, args.0, Rdx),
        store(S8, path.0, R8),
        store(S8, l_c.0, R9),
        load(S8, Rax, Local(Tmp, 4 * 8 + 16)),
        store(S8, code.0, Rax),
        load(S8, Rax, Local(Tmp, 5 * 8 + 16)),
        store(S8, carets.0, Rax),
      ],
      self.write_mems_x(&binds)?,
      vec![MovMId(Reg(Rcx), 1)],
    ];
    self.link_func_x(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn hidden_handler_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0;
    let id = symbol!(self, caller, HIDDEN_ERR);
    let hidden_err = Lit(HIDDEN_ERR.into());
    let insts = vec![self.write_mems_a(&[hidden_err])?, load_imm_a(X0, 1)];
    self.link_lbl_a(id, insts, SIZE, false);
    Ok(id)
  }
  pub(crate) fn hidden_handler_x(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0;
    let id = symbol!(self, caller, HIDDEN_ERR);
    let hidden_err = Lit(HIDDEN_ERR.into());
    let insts = vec![self.write_mems_x(&[hidden_err])?, vec![MovMId(Reg(Rcx), 1)]];
    self.link_lbl_x(id, insts, SIZE, true, LABEL_NOT_RETURN);
    Ok(id)
  }
  pub(crate) fn runtime_err(
    &mut self,
    err: RuntimeErr,
    args: Option<Bind<String>>,
    pos: Position,
    caller: LabelId,
  ) -> ErrOR<LabelId> {
    if self.flags.a64 {
      self.runtime_err_a(err, args, pos, caller)
    } else {
      self.runtime_err_x(err, args, pos, caller)
    }
  }
  fn runtime_err_a(
    &mut self,
    err: RuntimeErr,
    args: Option<Bind<String>>,
    pos: Position,
    caller: LabelId,
  ) -> ErrOR<LabelId> {
    const SIZE: i32 = 0;
    if !self.flags.debug {
      return self.hidden_handler_a(caller);
    }
    let id = self.id();
    self.use_func(caller, id);
    let (path, l_c, code, carets) = self.files[pos.file].err_info(pos);
    let binds = [
      Var(self.global_str(err.to_string())),
      args.unwrap_or(Lit(String::new())),
      Var(self.global_str(path)),
      Var(self.global_str(l_c)),
      Var(self.global_str(code)),
      Var(self.global_str(carets)),
    ];
    let mut insts = vec![];
    for (bind, reg) in binds.iter().zip(A64_ARG_REGS) {
      insts.push(self.load_str_a(reg, bind)?);
    }
    insts.extend([vec![Bl(self.err_handler_a(id)?)], load_imm_a(X0, 1)]);
    self.link_lbl_a(id, insts, SIZE, false);
    Ok(id)
  }
  fn runtime_err_x(
    &mut self,
    err: RuntimeErr,
    args: Option<Bind<String>>,
    pos: Position,
    caller: LabelId,
  ) -> ErrOR<LabelId> {
    const SIZE: i32 = 0;
    if !self.flags.debug {
      return self.hidden_handler_x(caller);
    }
    let id = self.id();
    self.use_func(caller, id);
    let (path, l_c, code, carets) = self.files[pos.file].err_info(pos);
    let insts = vec![
      load_x(Rcx, self.global_str(err.to_string()))?,
      self.load_str_x(Rdx, &args.unwrap_or(Lit(String::new())))?,
      load_x(R8, self.global_str(path))?,
      load_x(R9, self.global_str(l_c))?,
      load_x(Rax, self.global_str(code))?,
      vec![store(S8, Args(5), Rax)],
      load_x(Rax, self.global_str(carets))?,
      vec![store(S8, Args(6), Rax), Call(self.err_handler_x(id)?), MovMId(Reg(Rcx), 1)],
    ];
    self.link_lbl_x(id, insts, SIZE, true, LABEL_NOT_RETURN);
    Ok(id)
  }
  pub(crate) fn seh_handler(&mut self, caller: LabelId) -> ErrOR<()> {
    const SIZE: i32 = 0x30;
    self.use_func(caller, self.handlers.internal);
    let matched = self.id();
    let epilogue = self.id();
    let std_e = Global(self.get_std_e_x()?);
    let tmp = Local(Tmp, -0x8);
    let mut insts = vec![
      vec![load(S8, Rbx, Ref(Rcx))],
      self.write_err_msg(&make_header(INTERNAL_ERR), tmp)?,
      self.write_err_msg("\n| ", tmp)?,
    ];
    for (code, err) in
      [(0xFD, STACK_OVERFLOW), (5, ACCESS_VIOLATION), (0x94, ZERO_DIVISION), (0x1D, ILL_INST)]
    {
      insts.extend_from_slice(&self.seh_match(code, err, matched)?);
    }
    insts.extend([
      load_x(Rdx, self.global_str(EXCEPTION_OCCURRED))?,
      vec![MovMId(Reg(R8), len_u32(EXCEPTION_OCCURRED.as_bytes())?)],
      load_x(Rdi, self.global_str("R0000"))?,
      vec![
        LblX(matched),
        load(S8, Rcx, std_e),
        LeaRM(R9, tmp),
        Clear(Rax),
        store(S8, Args(5), Rax),
        CallApi(self.api(KERNEL32, "WriteFile")),
      ],
      self.write_err_msg(ERR_END, tmp)?,
      self.write_err_msg(ISSUE, tmp)?,
      vec![
        load(S8, Rcx, std_e),
        mov(S8, Rdx, Rdi),
        MovMId(Reg(R8), 5),
        LeaRM(R9, tmp),
        Clear(Rax),
        store(S8, Args(5), Rax),
        CallApi(self.api(KERNEL32, "WriteFile")),
      ],
      self.write_err_msg("`\n", tmp)?,
      vec![LblX(epilogue), mov(S8, Rcx, Rbx)],
    ]);
    self.link_lbl_x(self.handlers.internal, insts, SIZE, false, FN_NOT_RETURN);
    Ok(())
  }
  pub(crate) fn seh_match(
    &mut self,
    err_code: u32,
    err: &'static str,
    matched: LabelId,
  ) -> ErrOR<Vec<Vec<X64Inst>>> {
    Ok(vec![
      vec![MovMId(Reg(Rax), 0xC000_0000 | err_code)],
      load_x(Rdx, self.global_str(err))?,
      vec![MovMId(Reg(R8), len_u32(err.as_bytes())?)],
      load_x(Rdi, self.global_str(format!("R{err_code:04X}")))?,
      vec![RR(S8, Cmp, Rbx, Rax), JCc(E, matched)],
    ])
  }
  pub(crate) fn sig_handler(&mut self, caller: LabelId) -> ErrOR<()> {
    const SIZE: i32 = 0x10;
    self.use_func(caller, self.handlers.internal);
    let code = Local(Tmp, -0x4).v_rd();
    let err = Local(Tmp, -0x10).v_rq();
    let mut insts = vec![store_a(code, X9, X0)?];
    let binds = [Lit(make_header(INTERNAL_ERR)), Lit("\n| ".into())];
    let binds2 = [Lit(ERR_END.into()), Lit(ISSUE.into())];
    insts.extend([
      self.write_mems_a(&binds)?,
      load_a(X3, code)?,
      self.sig_match_a(SIGILL, ILL_INST)?,
      self.sig_match_a(SIGSEGV, ACCESS_VIOLATION)?,
      store_a(err, X9, X0)?,
      self.write_mems_a(&binds2)?,
      load_a(X0, code)?,
    ]);
    self.link_lbl_a(self.handlers.internal, insts, SIZE, false);
    Ok(())
  }
  pub(crate) fn sig_match_a(&mut self, sig: i64, err: &'static str) -> ErrOR<Vec<A64Inst>> {
    Ok(
      [
        load_a(X1, self.global_str(err))?,
        load_imm_a(X2, sig),
        vec![CmpRR(X3, X2), CSel(X0, X1, X0, E.into())],
      ]
      .concat(),
    )
  }
  pub(crate) fn unix_handler(&mut self, caller: LabelId) -> ErrOR<()> {
    const SIZE: i32 = 0x10;
    self.use_func(caller, self.handlers.os);
    let err_no = Local(Tmp, -0x4).v_rd();
    let buf5 = Local(Tmp, -0x10).ptr();
    let hex_loop = self.id();
    let digit = self.id();
    let store = self.id();
    let binds = [Lit(make_header(INTERNAL_ERR)), Lit("\n| ".into())];
    let binds2 =
      [Lit(ERR_END_NO_N.into()), Lit(ISSUE.into()), Lit("P".into()), Var(buf5), Lit("`\n".into())];
    let insts = vec![
      vec![BApi(self.api(SYS_B, "___error")), LdR(S4, X0, X0, 0)],
      store_a(err_no, X9, X0)?,
      load_a(X0, buf5)?,
      vec![AddRI12(X0, X0, 4), StR(S1, Xzr, X0, 0)],
      load_imm_a(X3, 4),
      load_a(X4, err_no)?,
      vec![LblA(hex_loop), SubRI12(X0, X0, 1)],
      load_imm_a(X1, 0xF),
      vec![AndR3(X2, X4, X1)],
      load_imm_a(X1, 10),
      vec![
        CmpRR(X2, X1),
        BCc(Ae.into(), digit),
        AddRI12(X2, X2, 0x37),
        B_(store),
        LblA(digit),
        AddRI12(X2, X2, 0x30),
        LblA(store),
        StR(S1, X2, X0, 0),
        // TODO lsr
        Asr(X4, X4, 4),
        SubRI12(X3, X3, 1),
        CmpRR(X3, Xzr),
        BCc(Ne.into(), hex_loop),
      ],
      self.write_mems_a(&binds)?,
      vec![MovRR(X0, Xzr), BApi(self.api(SYS_B, "_perror"))],
      self.write_mems_a(&binds2)?,
      load_a(X0, err_no)?,
    ];
    self.link_lbl_a(self.handlers.os, insts, SIZE, false);
    Ok(())
  }
  pub(crate) fn win_handler(&mut self, caller: LabelId) -> ErrOR<()> {
    const SIZE: i32 = 0x70;
    self.use_func(caller, self.handlers.os);
    let exit = self.id();
    let print_e = self.get_print_e_x(self.handlers.os)?;
    let u16_to_8 = self.get_u16_to_8(self.handlers.os)?;
    let digit = self.id();
    let hex_loop = self.id();
    let store_lbl = self.id();
    let tmp = Local(Tmp, -0x8);
    let msg = Local(Tmp, -0x10);
    let multi_byte = Local(Tmp, -0x18);
    let buf5 = Local(Tmp, -0x20);
    let std_e = Global(self.get_std_e_x()?);
    let insts = vec![
      vec![
        mov(S8, Rbp, Rsp),
        m8i(Sub, Rsp, SIZE),
        CallApi(self.api(KERNEL32, "GetLastError")),
        mov(S8, Rdi, Rax),
        MovMId(Reg(Rcx), 0x1300),
        Clear(Rdx),
        mov(S8, R8, Rdi),
        Clear(R9),
        LeaRM(Rax, msg),
        store(S8, Args(5), Rax),
        store(S8, Args(6), Rdx),
        store(S8, Args(7), Rdx),
        CallApi(self.api(KERNEL32, "FormatMessageW")),
        TestRR(S8, Rax),
        JCc(E, exit),
        load(S8, Rcx, msg),
        MovMId(Reg(Rdx), CP_UTF8),
        Call(u16_to_8),
        store(S8, multi_byte, Rax),
      ],
      self.write_err_msg(&make_header(INTERNAL_ERR), tmp)?,
      self.write_err_msg(WIN_API_ERR, tmp)?,
      vec![load(S8, Rcx, multi_byte), Call(print_e)],
      self.write_err_msg(ERR_END, tmp)?,
      self.write_err_msg(ISSUE, tmp)?,
      self.write_err_msg("W", tmp)?,
      vec![
        mov(S8, R9, Rdi),
        LeaRM(Rax, buf5),
        m8i(Add, Rax, 4),
        MovMIb(Ref(Rax), 0),
        MovMId(Reg(R8), 4),
        LblX(hex_loop),
        DecR(Rax),
        mov(S8, Rdx, R9),
        m8i(And, Rdx, 0xF),
        m8i(Cmp, Rdx, 10),
        JCc(B, digit),
        m8i(Add, Rdx, 0x37),
        Jmp(store_lbl),
        LblX(digit),
        m8i(Add, Rdx, 0x30),
        LblX(store_lbl),
        store(S1, Ref(Rax), Rdx),
        ShiftR(Shr, R9, Shift::Ib(4)),
        DecR(R8),
        JCc(Ne, hex_loop),
        load(S8, Rcx, std_e),
        LeaRM(Rdx, buf5),
        MovMId(Reg(R8), 4),
        LeaRM(R9, tmp),
        Clear(Rax),
        store(S8, Args(5), Rax),
        CallApi(self.api(KERNEL32, "WriteFile")),
      ],
      self.write_err_msg("`\n", tmp)?,
      vec![
        LblX(exit),
        load(S8, Rcx, msg),
        CallApi(self.api(KERNEL32, "LocalFree")),
        mov(S8, Rcx, Rdi),
      ],
    ];
    self.link_lbl_x(self.handlers.os, insts, SIZE, true, LABEL_NOT_RETURN);
    Ok(())
  }
  pub(crate) fn write_err_msg(&mut self, text: &str, tmp: Address) -> ErrOR<Vec<X64Inst>> {
    Ok(
      [
        vec![load(S8, Rcx, Global(self.get_std_e_x()?))],
        load_x(Rdx, self.global_str(text))?,
        vec![
          MovMId(Reg(R8), len_u32(text.as_bytes())?),
          LeaRM(R9, tmp),
          Clear(Rax),
          store(S8, Args(5), Rax),
          CallApi(self.api(KERNEL32, "WriteFile")),
        ],
      ]
      .concat(),
    )
  }
  pub(crate) fn write_mems_a(&mut self, binds: &[Bind<String>]) -> ErrOR<Vec<A64Inst>> {
    let print_e = self.get_print_e_a(self.handlers.os)?;
    let mut insts = vec![];
    for bind in binds {
      insts.extend([self.load_str_a(X0, bind)?, vec![Bl(print_e)]].concat())
    }
    Ok(insts)
  }
  pub(crate) fn write_mems_x(&mut self, binds: &[Bind<String>]) -> ErrOR<Vec<X64Inst>> {
    let print_e = self.get_print_e_x(self.handlers.os)?;
    let mut insts = vec![];
    for bind in binds {
      insts.extend(self.load_str_x(Rcx, bind)?);
      insts.push(Call(print_e));
    }
    Ok(insts)
  }
}
