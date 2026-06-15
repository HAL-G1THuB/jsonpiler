use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn get_print_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x10;
    let id = symbol!(self, caller, PRINT);
    let print_n = self.get_print_n_a(id)?;
    self.link_func_a(id, vec![load_imm_a(X1, 1), vec![Bl(print_n)]], SIZE);
    Ok(id)
  }
  pub(crate) fn get_print_e_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x10;
    let id = symbol!(self, caller, PRINT_E);
    let print_n = self.get_print_n_a(id)?;
    self.link_func_a(id, vec![load_imm_a(X1, 2), vec![Bl(print_n)]], SIZE);
    Ok(id)
  }
  pub(crate) fn get_print_e_x(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x20;
    let id = symbol!(self, caller, PRINT_E);
    let print_n = self.get_print_n_x(id)?;
    let std_e = Global(self.get_std_e_x()?);
    self.link_func_x(id, vec![vec![load(S8, Rdx, std_e), Call(print_n)]], SIZE);
    Ok(id)
  }
  pub(crate) fn get_print_n_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x10;
    let id = symbol!(self, caller, PRINT_N);
    let str_len = self.str_len_a(id)?;
    let std_n_and_tmp = Local(Tmp, -0x08).v_rq();
    let string = Local(Tmp, -0x10).v_rq();
    let insts = vec![
      store_a(string, X9, X0)?,
      store_a(std_n_and_tmp, X9, X1)?,
      vec![Bl(str_len), MovRR(X2, X0)],
      load_a(X0, std_n_and_tmp)?,
      load_a(X1, string)?,
      self.call_api_minus_one_a(SYS_B, "_write"),
    ];
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn get_print_n_x(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x40;
    let id = symbol!(self, caller, PRINT_N);
    let str_len = self.str_len_x(id)?;
    let std_n_and_tmp = Local(Tmp, -0x08);
    let string = Local(Tmp, -0x10);
    let insts = vec![
      store(S8, string, Rcx),
      store(S8, std_n_and_tmp, Rdx),
      Call(str_len),
      mov(S8, R8, Rax),
      load(S8, Rcx, std_n_and_tmp),
      load(S8, Rdx, string),
      LeaRM(R9, std_n_and_tmp),
      Clear(Rax),
      store(S8, Args(5), Rax),
      CallApiCheck(self.api(KERNEL32, "WriteFile")),
    ];
    self.link_func_x(id, vec![insts], SIZE);
    Ok(id)
  }
  pub(crate) fn get_print_x(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x20;
    let id = symbol!(self, caller, PRINT);
    let print_n = self.get_print_n_x(id)?;
    let std_o = Global(self.get_std_o_x()?);
    self.link_func_x(id, vec![vec![load(S8, Rdx, std_o), Call(print_n)]], SIZE);
    Ok(id)
  }
  pub(crate) fn get_std_e_x(&mut self) -> ErrOR<LabelId> {
    if let Some(id) = self.symbols.get(STD_E) {
      return Ok(*id);
    }
    let std_e = self.bss_symbol(STD_E, 8, 8);
    let std_n_insts = self.get_std_n_x((-12i32).cast_unsigned(), std_e)?;
    self.startup.x64_mut()?.push(std_n_insts);
    Ok(std_e)
  }
  pub(crate) fn get_std_i_x(&mut self) -> ErrOR<LabelId> {
    if let Some(id) = self.symbols.get(STD_I) {
      return Ok(*id);
    }
    let std_i = self.bss_symbol(STD_I, 8, 8);
    let std_n_insts = self.get_std_n_x((-10i32).cast_unsigned(), std_i)?;
    self.startup.x64_mut()?.push(std_n_insts);
    Ok(std_i)
  }
  pub(crate) fn get_std_n_x(&mut self, std_id: u32, std_n: LabelId) -> ErrOR<Vec<X64Inst>> {
    Ok(vec![
      MovMId(Reg(Rcx), std_id),
      CallApi(self.api(KERNEL32, "GetStdHandle")),
      m8i(Cmp, Rax, -1),
      JCc(E, self.handlers.os),
      store(S8, Global(std_n), Rax),
    ])
  }
  pub(crate) fn get_std_o_x(&mut self) -> ErrOR<LabelId> {
    if let Some(id) = self.symbols.get(STD_O) {
      return Ok(*id);
    }
    let std_o = self.bss_symbol(STD_O, 8, 8);
    let std_n_insts = self.get_std_n_x((-11i32).cast_unsigned(), std_o)?;
    self.startup.x64_mut()?.push(std_n_insts);
    Ok(std_o)
  }
  pub(crate) fn get_u16_to_8(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x60;
    let id = symbol!(self, caller, U16TO8);
    let cp = Local(Tmp, -0x04);
    let tmp_d = Local(Tmp, -0x10);
    let tmp_s = Local(Tmp, -0x18);
    let tmp_b = Local(Tmp, -0x20);
    let insts = vec![
      vec![
        store(S8, tmp_d, Rdi),
        store(S8, tmp_s, Rsi),
        store(S8, tmp_b, Rbx),
        mov(S8, Rdi, Rcx),
        store(S4, cp, Rdx),
        mov(S4, Rcx, Rdx),
        Clear(Rdx),
        mov(S8, R8, Rdi),
        MovMId(Reg(R9), u32::MAX),
        store(S8, Args(5), Rdx),
        store(S8, Args(6), Rdx),
        store(S8, Args(7), Rdx),
        store(S8, Args(8), Rdx),
        CallApiCheck(self.api(KERNEL32, "WideCharToMultiByte")),
        mov(S8, Rsi, Rax),
        mov(S8, R8, Rsi),
      ],
      self.call_alloc_r8_x()?,
      vec![
        mov(S8, Rbx, Rax),
        load(S4, Rcx, cp),
        Clear(Rdx),
        mov(S8, R8, Rdi),
        MovMId(Reg(R9), u32::MAX),
        mov(S8, Rax, Rbx),
        store(S8, Args(5), Rax),
        mov(S8, Rax, Rsi),
        store(S8, Args(6), Rax),
        store(S8, Args(7), Rdx),
        store(S8, Args(8), Rdx),
        CallApiCheck(self.api(KERNEL32, "WideCharToMultiByte")),
        RR(S8, Add, Rax, Rbx),
        DecR(Rax),
        Clear(Rcx),
        store(S1, Ref(Rax), Rcx),
        DecR(Rax),
        store(S1, Ref(Rax), Rcx),
        mov(S8, Rax, Rbx),
        load(S8, Rdi, tmp_d),
        load(S8, Rsi, tmp_s),
        load(S8, Rbx, tmp_b),
      ],
    ];
    self.link_func_x(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn get_u8_to_16(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x60;
    let id = symbol!(self, caller, U8TO16);
    let cp = Local(Tmp, -0x04);
    let tmp_d = Local(Tmp, -0x10);
    let tmp_s = Local(Tmp, -0x18);
    let tmp_b = Local(Tmp, -0x20);
    let insts = vec![
      vec![
        store(S8, tmp_d, Rdi),
        store(S8, tmp_s, Rsi),
        store(S8, tmp_b, Rbx),
        mov(S8, Rdi, Rcx),
        store(S4, cp, Rdx),
        mov(S4, Rcx, Rdx),
        Clear(Rdx),
        mov(S8, R8, Rdi),
        MovMId(Reg(R9), u32::MAX),
        store(S8, Args(5), Rdx),
        store(S8, Args(6), Rdx),
        CallApiCheck(self.api(KERNEL32, "MultiByteToWideChar")),
        ShiftR(Shl, Rax, Shift::One),
        mov(S8, Rsi, Rax),
        mov(S8, R8, Rsi),
      ],
      self.call_alloc_r8_x()?,
      vec![
        mov(S8, Rbx, Rax),
        load(S4, Rcx, cp),
        Clear(Rdx),
        mov(S8, R8, Rdi),
        MovMId(Reg(R9), u32::MAX),
        store(S8, Args(5), Rbx),
        store(S8, Args(6), Rsi),
        CallApiCheck(self.api(KERNEL32, "MultiByteToWideChar")),
        mov(S8, Rax, Rbx),
        load(S8, Rdi, tmp_d),
        load(S8, Rsi, tmp_s),
        load(S8, Rbx, tmp_b),
      ],
    ];
    self.link_func_x(id, insts, SIZE);
    Ok(id)
  }
}
