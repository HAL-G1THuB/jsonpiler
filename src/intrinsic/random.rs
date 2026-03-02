use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn get_random(&mut self) -> ErrOR<u32> {
    const SIZE: u32 = 0x10;
    let id = symbol!(self, RANDOM);
    let end = self.id();
    self.data_insts.push(Seh(id, end, SIZE));
    let q_perf_cnt = self.import(KERNEL32, "QueryPerformanceCounter")?;
    let get_pid = self.import(KERNEL32, "GetCurrentProcessId")?;
    let get_tid = self.import(KERNEL32, "GetCurrentThreadId")?;
    let seed = Global(self.bss(8, 8));
    self.startup.extend_from_slice(&[
      LeaRM(Rcx, seed),
      CallApiNull(q_perf_cnt),
      CallApiNull(get_pid),
      mov_q(Rcx, seed),
      LogicRR(Xor, Rax, Rcx),
      mov_q(seed, Rax),
      CallApiNull(get_tid),
      mov_q(Rcx, seed),
      LogicRR(Xor, Rax, Rcx),
      mov_q(seed, Rax),
    ]);
    self.insts.extend_from_slice(&[
      Lbl(id),
      Push(Rbp),
      mov_q(Rbp, Rsp),
      SubRId(Rsp, SIZE),
      mov_q(Rax, seed),
      mov_q(Rcx, Rax),
      ShlRIb(Rcx, 7),
      LogicRR(Xor, Rax, Rcx),
      mov_q(Rcx, Rax),
      ShrRIb(Rcx, 9),
      LogicRR(Xor, Rax, Rcx),
      mov_q(Rcx, Rax),
      ShlRIb(Rcx, 13),
      LogicRR(Xor, Rax, Rcx),
      mov_q(seed, Rax),
      mov_q(Rsp, Rbp),
      Pop(Rbp),
      Custom(RET),
      Lbl(end),
    ]);
    Ok(id)
  }
}
