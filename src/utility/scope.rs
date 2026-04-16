use crate::prelude::*;
use core::iter;
#[derive(Default, Debug, Clone)]
pub(crate) struct Scope {
  args_count: u32,
  body: Vec<Inst>,
  pub epilogue: Option<(LabelId, JsonType)>,
  pub id: LabelId,
  pub local_top: BTreeMap<String, Pos<Variable>>,
  pub locals: Vec<BTreeMap<String, Pos<Variable>>>,
  pub loop_labels: Vec<(LabelId, LabelId, usize)>,
  stack_size: i32,
  unused_map: BTreeMap<i32, i32>,
}
#[derive(Default, Debug, Clone)]
pub(crate) struct Variable {
  pub used: bool,
  pub val: Json,
}
impl Variable {
  pub(crate) fn new(val: Json) -> Self {
    Variable { val, used: false }
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
    Err(Internal(InternalOverFlow))
  }
  pub(crate) fn change(&mut self, id: LabelId) -> Self {
    let scope = take(self);
    self.id = id;
    scope
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
    Err(Internal(StackLeak))
  }
  pub(crate) fn extend(&mut self, insts: &[Inst]) {
    self.body.extend_from_slice(insts);
  }
  pub(crate) fn free(&mut self, mut start: i32, mem_type: MemoryType) {
    let mut size = mem_type.size();
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
  }
  pub(crate) fn get_var_local(&mut self, var: &str) -> Option<Json> {
    for locals in self.iter_locals() {
      if let Some(variable) = locals.get_mut(var) {
        variable.val.used = true;
        return Some(variable.val.val.clone());
      }
    }
    None
  }
  pub(crate) fn innermost(&mut self) -> &mut BTreeMap<String, Pos<Variable>> {
    self.locals.last_mut().unwrap_or(&mut self.local_top)
  }
  pub(crate) fn iter_locals(
    &mut self,
  ) -> impl Iterator<Item = &mut BTreeMap<String, Pos<Variable>>> {
    self.locals.iter_mut().rev().chain(iter::once(&mut self.local_top))
  }
  pub(crate) fn new(id: LabelId) -> Self {
    Scope { id, ..Scope::default() }
  }
  pub(crate) fn push(&mut self, inst: Inst) {
    self.body.push(inst);
  }
  pub(crate) fn replace(&mut self, old_scope: Self) -> Vec<Inst> {
    replace(self, old_scope).body
  }
  pub(crate) fn resolve_stack_size(&self) -> ErrOR<i32> {
    // let ret_addr = 8; let rbp = 8; (ret_addr + rbp) % 16 == 0
    Ok(i32::try_from(align_up_u32(
      self.args_count.max(4) * 8 + u32::try_from(self.stack_size)?,
      16,
    )?)?)
  }
  pub(crate) fn ret(&mut self, src: Register) -> ErrOR<Memory> {
    let addr = Local(Tmp, self.alloc(8, 8)?);
    self.push(mov_q(addr, src));
    Ok(Memory(addr, MemoryType { heap: Value, size: Small(RQ) }))
  }
  pub(crate) fn ret_bool(&mut self, src: Register) -> ErrOR<Json> {
    let addr = Local(Tmp, self.alloc(1, 1)?);
    self.push(mov_b(addr, src));
    Ok(Bool(Var(Memory(addr, MemoryType { heap: Value, size: Small(RB) }))))
  }
  pub(crate) fn ret_json_take(&mut self, dst: &Pos<JsonType>, src: Register) -> ErrOR<Json> {
    Ok(match dst.val {
      NullT => Null(Var(self.ret(src)?)),
      IntT => Int(Var(self.ret(src)?)),
      BoolT => self.ret_bool(src)?,
      FloatT => Float(Var(self.ret(src)?)),
      StrT => self.ret_str(src, HeapPtr)?,
      CustomT(_) | ArrayT | ObjectT => return err!(dst.pos, UnsupportedType(dst.val.to_string())),
    })
  }
  pub(crate) fn ret_str(&mut self, src: Register, heap: Storage) -> ErrOR<Json> {
    let addr = Local(Tmp, self.alloc(8, 8)?);
    self.push(mov_q(addr, src));
    Ok(Str(Var(Memory(addr, MemoryType { heap, size: Dynamic }))))
  }
  pub(crate) fn ret_xmm(&mut self, xmm: Register) -> ErrOR<Json> {
    let addr = Local(Tmp, self.alloc(8, 8)?);
    self.push(MovMSd(addr, xmm));
    Ok(Float(Var(Memory(addr, MemoryType { heap: Value, size: Small(RQ) }))))
  }
  pub(crate) fn take_body(self) -> Vec<Inst> {
    self.body
  }
  pub(crate) fn tmp(&mut self, size: i32, align: i32, func: &mut Pos<BuiltIn>) -> ErrOR<Address> {
    Ok(Local(Tmp, self.tmp_offset(size, align, func)?))
  }
  pub(crate) fn tmp_offset(
    &mut self,
    size: i32,
    align: i32,
    func: &mut Pos<BuiltIn>,
  ) -> ErrOR<i32> {
    let tmp = self.alloc(size, align)?;
    let memory = Memory(Local(Tmp, tmp), MemoryType { heap: Value, size: Known(size) });
    func.val.free_list.insert(memory);
    Ok(tmp)
  }
  pub(crate) fn update_args_count(&mut self, size: u32) {
    self.args_count = self.args_count.max(size);
  }
}
