use crate::prelude::*;
use core::iter;
use std::collections::BTreeMap;
#[derive(Default, Debug, Clone)]
pub(crate) struct Scope {
  args_count: u32,
  pub body: Vec<Inst>,
  pub epilogue: Option<(u32, Json)>,
  pub local_top: HashMap<String, Json>,
  pub locals: Vec<HashMap<String, Json>>,
  pub loop_labels: Vec<(u32, u32)>,
  stack_size: i32,
  unused_map: BTreeMap<i32, i32>,
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
      self.unused_map.remove(&start);
      if before > 0 {
        self.unused_map.insert(start, before);
      }
      self.stack_size = self.stack_size.max(-start);
      return Ok(used_start);
    }
    Err(Internal(OverFlow))
  }
  pub(crate) fn check_free(&mut self) -> ErrOR<()> {
    if self.stack_size == 0 {
      if !self.unused_map.is_empty() {
        return Err(Internal(UnbalancedStack));
      }
    } else {
      let is_freed = self.unused_map.get(&-self.stack_size) == Some(&self.stack_size);
      if self.unused_map.len() != 1 || !is_freed {
        return Err(Internal(UnbalancedStack));
      }
    }
    Ok(())
  }
  pub(crate) fn extend(&mut self, insts: &[Inst]) {
    self.body.extend_from_slice(insts);
  }
  pub(crate) fn free(&mut self, mut start: i32, size: LabelSize) {
    let mut used = size.to_int();
    if let Some((&prev_start, &prev_unused)) = self.unused_map.range(..start).next_back()
      && start == prev_start + prev_unused
    {
      self.unused_map.remove(&prev_start);
      start = prev_start;
      used += prev_unused;
    }
    if let Some((&next_start, &next_unused)) = self.unused_map.range(start..).next()
      && start + used == next_start
    {
      self.unused_map.remove(&next_start);
      used += next_unused;
    }
    self.unused_map.insert(start, used);
  }
  pub(crate) fn get_var_local(&self, var: &str) -> Option<Json> {
    self.locals.iter().chain(iter::once(&self.local_top)).find_map(|table| table.get(var).cloned())
  }
  pub(crate) fn heap_free(&mut self, offset: i32, (heap, free): (u32, (u32, u32))) {
    self.extend(&[
      mov_q(Rcx, Global(heap)),
      Clear(Rdx),
      mov_q(R8, Local(Tmp, offset)),
      CallApiNull(free),
    ]);
  }
  pub(crate) fn push(&mut self, inst: Inst) {
    self.body.push(inst);
  }
  pub(crate) fn resolve_stack_size(&self) -> ErrOR<u32> {
    // let ret_addr = 8; let rbp = 8; (ret_addr + rbp) % 16 == 0
    align_up_32(self.args_count.max(4) * 8 + u32::try_from(self.stack_size)?, 16)
  }
  pub(crate) fn ret(&mut self, src: Register) -> ErrOR<Label> {
    let addr = Local(Tmp, self.alloc(8, 8)?);
    self.push(mov_q(addr, src));
    Ok(Label(addr, Size(8)))
  }
  pub(crate) fn ret_bool(&mut self, src: Register) -> ErrOR<Json> {
    let addr = Local(Tmp, self.alloc(1, 1)?);
    self.push(mov_b(addr, src));
    Ok(Bool(Var(Label(addr, Size(1)))))
  }
  pub(crate) fn ret_json(&mut self, src: Register, jwp: &WithPos<Json>) -> ErrOR<Json> {
    Ok(match jwp.val {
      Null => Null,
      Int(_) => Int(Var(self.ret(src)?)),
      Bool(_) => self.ret_bool(src)?,
      Float(_) => Float(Var(self.ret(src)?)),
      Str(_) => self.ret_str(src)?,
      Array(_) | Object(_) => return err!(jwp.pos, UnsupportedType(jwp.val.describe())),
    })
  }
  pub(crate) fn ret_str(&mut self, src: Register) -> ErrOR<Json> {
    let addr = Local(Tmp, self.alloc(8, 8)?);
    self.push(mov_q(addr, src));
    Ok(Str(Var(Label(addr, Heap))))
  }
  pub(crate) fn ret_xmm(&mut self, xmm: Register) -> ErrOR<Json> {
    let addr = Local(Tmp, self.alloc(8, 8)?);
    self.push(MovSdMX(addr, xmm));
    Ok(Float(Var(Label(addr, Size(8)))))
  }
  pub(crate) fn update_args_count(&mut self, size: u32) {
    self.args_count = self.args_count.max(size);
  }
}
