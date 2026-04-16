use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn get_critical_section(&mut self) -> LabelId {
    if let Some(id) = self.symbols.get(CRITICAL_SECTION) {
      return *id;
    }
    let initialize_cs = self.import(KERNEL32, "InitializeCriticalSection");
    let critical_section = self.bss(0x28, 8);
    self.startup.extend_from_slice(&[LeaRM(Rcx, Global(critical_section)), CallApi(initialize_cs)]);
    self.symbols.insert(CRITICAL_SECTION, critical_section);
    critical_section
  }
  pub(crate) fn get_random(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x10;
    let id = symbol!(self, caller, RANDOM);
    let q_perf_cnt = self.import(KERNEL32, "QueryPerformanceCounter");
    let get_pid = self.import(KERNEL32, "GetCurrentProcessId");
    let get_tid = self.import(KERNEL32, "GetCurrentThreadId");
    let seed = Global(self.bss(8, 8));
    self.startup.extend_from_slice(&[
      LeaRM(Rcx, seed),
      CallApiCheck(q_perf_cnt),
      CallApiCheck(get_pid),
      mov_q(Rcx, seed),
      LogicRR(Xor, Rax, Rcx),
      mov_q(seed, Rax),
      CallApiCheck(get_tid),
      mov_q(Rcx, seed),
      LogicRR(Xor, Rax, Rcx),
      mov_q(seed, Rax),
    ]);
    self.link_function(
      id,
      &[
        mov_q(Rax, seed),
        mov_q(Rcx, Rax),
        ShiftR(Shl, Rcx, Shift::Ib(7)),
        LogicRR(Xor, Rax, Rcx),
        mov_q(Rcx, Rax),
        ShiftR(Shr, Rcx, Shift::Ib(9)),
        LogicRR(Xor, Rax, Rcx),
        mov_q(Rcx, Rax),
        ShiftR(Shl, Rcx, Shift::Ib(13)),
        LogicRR(Xor, Rax, Rcx),
        mov_q(seed, Rax),
      ],
      SIZE,
    );
    Ok(id)
  }
}
