use crate::prelude::*;
#[derive(Debug, Clone)]
pub(crate) struct Scope {
  body: AorX,
  pub epilogue: Option<(LabelId, JsonType)>,
  pub id: LabelId,
  pub local_top: BTreeMap<String, Pos<Variable>>,
  pub locals: Vec<BTreeMap<String, Pos<Variable>>>,
  pub loop_labels: Vec<(LabelId, LabelId, usize)>,
  stack_args_count: u32,
  stack_size: i32,
  unused_map: BTreeMap<i32, i32>,
}
impl Scope {
  pub(crate) fn new(id: LabelId, a64: bool) -> Self {
    Scope {
      id,
      stack_args_count: 0,
      body: AorX::new(a64),
      epilogue: None,
      local_top: BTreeMap::new(),
      locals: vec![],
      loop_labels: vec![],
      stack_size: 0,
      unused_map: BTreeMap::new(),
    }
  }
}
impl Scope {
  pub(crate) fn alloc(&mut self, used: i32, align: i32) -> ErrOR<i32> {
    let new_used_end = align_down_i32(-self.stack_size, align)?;
    let new_after = -self.stack_size - new_used_end;
    let new_unused = used + new_after;
    let new_start = -self.stack_size - new_unused;
    for (&start, &unused) in self.unused_map.iter().chain(iter::once((&new_start, &new_unused))) {
      let end = start + unused;
      let used_end = align_down_i32(end, align)?;
      let after = end - used_end;
      let used_start = used_end - used;
      let before = unused - used - after;
      if before < 0 {
        continue;
      }
      if after > 0 {
        self.unused_map.insert(used_end, after);
      }
      if before > 0 {
        self.unused_map.insert(start, before);
      } else {
        self.unused_map.remove(&start);
      }
      self.stack_size = self.stack_size.max(-start);
      return Ok(used_start);
    }
    Err(InternalOverFlow.into())
  }
  pub(crate) fn check_free(&mut self) -> ErrOR<()> {
    let mut unused_total = 0;
    let mut last_start = 0;
    for (start, unused) in &self.unused_map {
      unused_total += unused;
      last_start = last_start.min(*start);
    }
    if last_start == -self.stack_size && unused_total == self.stack_size {
      return Ok(());
    }
    Err(StackLeak.into())
  }
  pub(crate) fn free(&mut self, mut start: i32, mut size: i32) -> ErrOR<()> {
    if let Some((&prev_start, &prev_unused)) = self.unused_map.range(..start).next_back()
      && start == prev_start + prev_unused
    {
      self.unused_map.remove(&prev_start);
      start = prev_start;
      size += prev_unused;
    }
    if let Some((&next_start, &next_unused)) = self.unused_map.range(start..).next()
      && start + size == next_start
    {
      self.unused_map.remove(&next_start);
      size += next_unused;
    }
    self.unused_map.insert(start, size);
    Ok(())
  }
  pub(crate) fn tmp(&mut self, size: i32, align: i32, func: &mut Pos<BuiltIn>) -> ErrOR<Address> {
    let tmp = self.alloc(size, align)?;
    let mem_type = MemType { pass_by: Value, size: Known(size) };
    func.push_free_tmp(Some(Memory(Local(Tmp, tmp), mem_type)));
    Ok(Local(Tmp, tmp))
  }
}
impl Scope {
  pub(crate) fn resolve_a64_stack_size(&self) -> ErrOR<i32> {
    // let x29 = 8; let x30 = 8; (stack_size + x29 + x30) % 16 == 0
    let args_size = self.stack_args_count * 8;
    Ok(i32::try_from(align_up_u32(args_size + u32::try_from(self.stack_size)?, 16)?)?)
  }
  pub(crate) fn resolve_x64_stack_size(&self) -> ErrOR<i32> {
    // let ret_addr = 8; let rbp = 8; (stack_size + ret_addr + rbp) % 16 == 0
    let args_size = self.stack_args_count * 8;
    Ok(i32::try_from(align_up_u32(0x20 + args_size + u32::try_from(self.stack_size)?, 16)?)?)
  }
  pub(crate) fn update_stack_args_count(&mut self, size: u32) {
    self.stack_args_count = self.stack_args_count.max(size);
  }
}
impl Scope {
  pub(crate) fn e_a(&mut self, insts: Vec<A64Inst>) -> ErrOR<()> {
    self.body.a64_mut()?.push(insts);
    Ok(())
  }
  pub(crate) fn e_x(&mut self, insts: Vec<X64Inst>) -> ErrOR<()> {
    self.body.x64_mut()?.push(insts);
    Ok(())
  }
  pub(crate) fn ee_a(&mut self, insts: Vec<Vec<A64Inst>>) -> ErrOR<()> {
    let body = self.body.a64_mut()?;
    body.extend(insts);
    Ok(())
  }
  pub(crate) fn ee_x(&mut self, insts: Vec<Vec<X64Inst>>) -> ErrOR<()> {
    let body = self.body.x64_mut()?;
    body.extend(insts);
    Ok(())
  }
  pub(crate) fn p_a(&mut self, inst: A64Inst) -> ErrOR<()> {
    self.body.a64_mut()?.push(vec![inst]);
    Ok(())
  }
  pub(crate) fn p_x(&mut self, inst: X64Inst) -> ErrOR<()> {
    self.body.x64_mut()?.push(vec![inst]);
    Ok(())
  }
  pub(crate) fn push_jcc(&mut self, cc: X64Cc, lbl: LabelId) {
    match &mut self.body {
      A64(body) => body.push(vec![BCc(cc.into(), lbl)]),
      X64(body) => body.push(vec![JCc(cc, lbl)]),
    }
  }
  pub(crate) fn push_jmp(&mut self, lbl: LabelId) {
    match &mut self.body {
      A64(body) => body.push(vec![B_(lbl)]),
      X64(body) => body.push(vec![Jmp(lbl)]),
    }
  }
  pub(crate) fn push_lbl(&mut self, lbl: LabelId) {
    match &mut self.body {
      A64(body) => body.push(vec![LblA(lbl)]),
      X64(body) => body.push(vec![LblX(lbl)]),
    }
  }
}
impl Scope {
  pub(crate) fn change(&mut self, id: LabelId) -> Self {
    let scope = replace(self, Scope::new(id, self.body.is_a64()));
    self.id = id;
    scope
  }
  pub(crate) fn get_var_local(&mut self, var: &Pos<String>) -> Option<&Pos<Variable>> {
    self.iter_locals().find_map(|locals| locals.get_var(var))
  }
  pub(crate) fn innermost(&mut self) -> &mut BTreeMap<String, Pos<Variable>> {
    self.locals.last_mut().unwrap_or(&mut self.local_top)
  }
  pub(crate) fn iter_locals(
    &mut self,
  ) -> impl Iterator<Item = &mut BTreeMap<String, Pos<Variable>>> {
    self.locals.iter_mut().rev().chain(iter::once(&mut self.local_top))
  }
  pub(crate) fn replace(&mut self, scope: Self) -> AorX {
    replace(self, scope).body
  }
  pub(crate) fn take_body(&mut self) -> AorX {
    take(&mut self.body)
  }
}
fn align_down_i32(num: i32, align: i32) -> ErrOR<i32> {
  num.div_euclid(align).checked_mul(align).ok_or(InternalOverFlow.into())
}
