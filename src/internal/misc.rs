use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn get_critical_section(&mut self) -> ErrOR<LabelId> {
    if let Some(id) = self.symbols.get(LOCK) {
      return Ok(*id);
    }
    let critical_sect = self.bss_symbol(LOCK, 0x28, 8);
    let insts = vec![
      LeaRM(Rcx, Global(critical_sect)),
      CallApi(self.api(KERNEL32, "InitializeCriticalSection")),
    ];
    self.startup.x64_mut()?.push(insts);
    Ok(critical_sect)
  }
  pub(crate) fn get_heap(&mut self) -> ErrOR<LabelId> {
    if let Some(id) = self.symbols.get(HEAP) {
      return Ok(*id);
    }
    let heap = self.bss_symbol(HEAP, 8, 8);
    let insts = vec![CallApi(self.api(KERNEL32, "GetProcessHeap")), store(S8, Global(heap), Rax)];
    self.startup.x64_mut()?.push(insts);
    Ok(heap)
  }
  pub(crate) fn get_leak(&mut self) -> ErrOR<LabelId> {
    if let Some(id) = self.symbols.get(LEAK) { Ok(*id) } else { Ok(self.bss_symbol(LEAK, 4, 4)) }
  }
  pub(crate) fn get_os_unfair_lock(&mut self) -> ErrOR<LabelId> {
    if let Some(id) = self.symbols.get(LOCK) { Ok(*id) } else { Ok(self.bss_symbol(LOCK, 4, 4)) }
  }
  pub(crate) fn get_random_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x10;
    let id = symbol!(self, caller, RANDOM);
    let tmp = Local(Tmp, -0x08).v_rq();
    let arc4random = self.api(SYS_B, "_arc4random");
    let insts = vec![
      vec![BApi(arc4random)],
      store_a(tmp, X1, X0)?,
      vec![BApi(arc4random)],
      load_a(X1, tmp)?,
      vec![Lsl(X0, X0, 32), OrrR3(X0, X0, X1)],
    ];
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn get_random_x(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x10;
    let id = symbol!(self, caller, RANDOM);
    let seed = Global(self.bss(8, 8));
    let init = vec![
      LeaRM(Rcx, seed),
      CallApiCheck(self.api(KERNEL32, "QueryPerformanceCounter")),
      CallApiCheck(self.api(KERNEL32, "GetCurrentProcessId")),
      m_r(S8, Xor, seed, Rax),
      CallApiCheck(self.api(KERNEL32, "GetCurrentThreadId")),
      m_r(S8, Xor, seed, Rax),
    ];
    self.startup.x64_mut()?.push(init);
    let insts = vec![
      load(S8, Rax, seed),
      mov(S8, Rcx, Rax),
      ShiftR(Shl, Rcx, Shift::Ib(7)),
      RR(S8, Xor, Rax, Rcx),
      mov(S8, Rcx, Rax),
      ShiftR(Shr, Rcx, Shift::Ib(9)),
      RR(S8, Xor, Rax, Rcx),
      mov(S8, Rcx, Rax),
      ShiftR(Shl, Rcx, Shift::Ib(13)),
      RR(S8, Xor, Rax, Rcx),
      store(S8, seed, Rax),
    ];
    self.link_func_x(id, vec![insts], SIZE);
    Ok(id)
  }
}
