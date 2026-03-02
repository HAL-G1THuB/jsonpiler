mod copy_str;
mod err_handler;
mod input;
mod print_n;
mod random;
mod wnd_proc;
use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn get_critical_section(&mut self) -> ErrOR<u32> {
    if let Some(id) = self.symbols.get(CRITICAL_SECTION) {
      return Ok(*id);
    }
    let initialize_cs = self.import(KERNEL32, "InitializeCriticalSection")?;
    let critical_section = self.bss(0x28, 8);
    self.symbols.insert(CRITICAL_SECTION, critical_section);
    self.startup.extend_from_slice(&[LeaRM(Rcx, Global(critical_section)), CallApi(initialize_cs)]);
    Ok(critical_section)
  }
  pub(crate) fn get_msg_box(&mut self) -> ErrOR<u32> {
    const SIZE: u32 = 0x40;
    let id = symbol!(self, MSG_BOX);
    let end = self.id();
    self.data_insts.push(Seh(id, end, SIZE));
    let heap = Global(self.symbols[HEAP]);
    let u8_to_16 = self.get_u8_to_16()?;
    let heap_free = self.import(KERNEL32, "HeapFree")?;
    let message_box_w = self.import(USER32, "MessageBoxW")?;
    let tmp_d = Local(Tmp, -0x08);
    let tmp_s = Local(Tmp, -0x10);
    let tmp_b = Local(Tmp, -0x18);
    self.insts.extend_from_slice(&[
      Lbl(id),
      Push(Rbp),
      mov_q(Rbp, Rsp),
      SubRId(Rsp, SIZE),
      mov_q(tmp_d, Rdi),
      mov_q(tmp_s, Rsi),
      mov_q(tmp_b, Rbx),
      mov_q(Rsi, Rdx),
      mov_q(Rbx, R8),
      Call(u8_to_16),
      mov_q(Rdi, Rax),
      mov_q(Rcx, Rsi),
      Call(u8_to_16),
      mov_q(Rsi, Rax),
      Clear(Rcx),
      mov_q(Rdx, Rsi),
      mov_q(R8, Rdi),
      mov_q(R9, Rbx),
      CallApiNull(message_box_w),
      mov_q(Rcx, heap),
      Clear(Rdx),
      mov_q(R8, Rdi),
      CallApiNull(heap_free),
      mov_q(Rcx, heap),
      Clear(Rdx),
      mov_q(R8, Rsi),
      CallApiNull(heap_free),
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
