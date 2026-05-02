use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn get_msg_box(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x40;
    let id = symbol!(self, caller, MSG_BOX);
    let u8_to_16 = self.get_u8_to_16(id)?;
    let title = Local(Tmp, -0x08);
    let text = Local(Tmp, -0x10);
    let type_and_ret = Local(Tmp, -0x18);
    let insts = &[
      mov_q(text, Rdx),
      mov_q(type_and_ret, R8),
      mov_d(Rdx, 65001),
      Call(u8_to_16),
      mov_q(title, Rax),
      mov_q(Rcx, text),
      mov_d(Rdx, 65001),
      Call(u8_to_16),
      mov_q(text, Rax),
      Clear(Rcx),
      mov_q(Rdx, text),
      mov_q(R8, title),
      mov_q(R9, type_and_ret),
      CallApiCheck(self.api(USER32, "MessageBoxW")),
      mov_q(type_and_ret, Rax),
      mov_q(Rcx, Global(self.symbols[HEAP])),
      Clear(Rdx),
      mov_q(R8, title),
      CallApiCheck(self.api(KERNEL32, "HeapFree")),
      DecMd(Global(self.symbols[LEAK_CNT])),
      mov_q(Rcx, Global(self.symbols[HEAP])),
      Clear(Rdx),
      mov_q(R8, text),
      CallApiCheck(self.api(KERNEL32, "HeapFree")),
      DecMd(Global(self.symbols[LEAK_CNT])),
      mov_q(Rax, type_and_ret),
    ];
    self.link_function(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn get_print(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x20;
    let id = symbol!(self, caller, PRINT);
    let print_n = self.get_print_n(id)?;
    let std_o = Global(self.symbols[STD_O]);
    self.link_function(id, &[mov_q(Rdx, std_o), Call(print_n)], SIZE);
    Ok(id)
  }
  pub(crate) fn get_print_e(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x20;
    let id = symbol!(self, caller, PRINT_E);
    let print_n = self.get_print_n(id)?;
    let std_e = Global(self.symbols[STD_E]);
    self.link_function(id, &[mov_q(Rdx, std_e), Call(print_n)], SIZE);
    Ok(id)
  }
  pub(crate) fn get_print_n(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x40;
    let id = symbol!(self, caller, PRINT_N);
    let str_len = self.str_len(id)?;
    let std_n_and_tmp = Local(Tmp, -0x08);
    let string = Local(Tmp, -0x10);
    let insts = &[
      mov_q(string, Rcx),
      mov_q(std_n_and_tmp, Rdx),
      Call(str_len),
      mov_q(R8, Rax),
      mov_q(Rcx, std_n_and_tmp),
      mov_q(Rdx, string),
      LeaRM(R9, std_n_and_tmp),
      Clear(Rax),
      mov_q(Args(5), Rax),
      CallApiCheck(self.api(KERNEL32, "WriteFile")),
    ];
    self.link_function(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn get_u16_to_8(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x60;
    let id = symbol!(self, caller, U16TO8);
    let cp = Local(Tmp, -0x04);
    let tmp_d = Local(Tmp, -0x10);
    let tmp_s = Local(Tmp, -0x18);
    let tmp_b = Local(Tmp, -0x20);
    let insts = &[
      mov_q(tmp_d, Rdi),
      mov_q(tmp_s, Rsi),
      mov_q(tmp_b, Rbx),
      mov_q(Rdi, Rcx),
      mov_d(cp, Rdx),
      mov_d(Rcx, Rdx),
      Clear(Rdx),
      mov_q(R8, Rdi),
      mov_d(R9, u32::MAX),
      mov_q(Args(5), Rdx),
      mov_q(Args(6), Rdx),
      mov_q(Args(7), Rdx),
      mov_q(Args(8), Rdx),
      CallApiCheck(self.api(KERNEL32, "WideCharToMultiByte")),
      mov_q(Rsi, Rax),
      mov_q(Rcx, Global(self.symbols[HEAP])),
      mov_d(Rdx, 8),
      mov_q(R8, Rsi),
      CallApi(self.api(KERNEL32, "HeapAlloc")),
      IncMd(Global(self.symbols[LEAK_CNT])),
      mov_q(Rbx, Rax),
      mov_d(Rcx, cp),
      Clear(Rdx),
      mov_q(R8, Rdi),
      mov_d(R9, u32::MAX),
      mov_q(Rax, Rbx),
      mov_q(Args(5), Rax),
      mov_q(Rax, Rsi),
      mov_q(Args(6), Rax),
      mov_q(Args(7), Rdx),
      mov_q(Args(8), Rdx),
      CallApiCheck(self.api(KERNEL32, "WideCharToMultiByte")),
      AddRR(Rax, Rbx),
      DecR(Rax),
      Clear(Rcx),
      mov_b(Ref(Rax), Rcx),
      DecR(Rax),
      mov_b(Ref(Rax), Rcx),
      mov_q(Rax, Rbx),
      mov_q(Rdi, tmp_d),
      mov_q(Rsi, tmp_s),
      mov_q(Rbx, tmp_b),
    ];
    self.link_function(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn get_u8_to_16(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x60;
    let id = symbol!(self, caller, U8TO16);
    let cp = Local(Tmp, -0x04);
    let tmp_d = Local(Tmp, -0x10);
    let tmp_s = Local(Tmp, -0x18);
    let tmp_b = Local(Tmp, -0x20);
    let insts = &[
      mov_q(tmp_d, Rdi),
      mov_q(tmp_s, Rsi),
      mov_q(tmp_b, Rbx),
      mov_q(Rdi, Rcx),
      mov_d(cp, Rdx),
      mov_d(Rcx, Rdx),
      Clear(Rdx),
      mov_q(R8, Rdi),
      mov_d(R9, u32::MAX),
      mov_q(Args(5), Rdx),
      mov_q(Args(6), Rdx),
      CallApiCheck(self.api(KERNEL32, "MultiByteToWideChar")),
      ShiftR(Shl, Rax, Shift::One),
      mov_q(Rsi, Rax),
      mov_q(Rcx, Global(self.symbols[HEAP])),
      Clear(Rdx),
      mov_q(R8, Rsi),
      CallApi(self.api(KERNEL32, "HeapAlloc")),
      IncMd(Global(self.symbols[LEAK_CNT])),
      mov_q(Rbx, Rax),
      mov_d(Rcx, cp),
      Clear(Rdx),
      mov_q(R8, Rdi),
      mov_d(R9, u32::MAX),
      mov_q(Args(5), Rbx),
      mov_q(Args(6), Rsi),
      CallApiCheck(self.api(KERNEL32, "MultiByteToWideChar")),
      mov_q(Rax, Rbx),
      mov_q(Rdi, tmp_d),
      mov_q(Rsi, tmp_s),
      mov_q(Rbx, tmp_b),
    ];
    self.link_function(id, insts, SIZE);
    Ok(id)
  }
}
