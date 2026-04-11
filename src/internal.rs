pub(crate) mod handler;
mod input;
mod print_n;
mod random;
mod str_utility;
mod wnd_proc;
use crate::prelude::*;
use std::collections::btree_map::Entry;
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
  pub(crate) fn link_function(&mut self, id: LabelId, insts: &[Inst], stack_size: i32) {
    let end = self.id();
    self.link_label(id, insts, stack_size, Some(end), true, true);
  }
  pub(crate) fn link_function_no_seh(&mut self, id: LabelId, insts: &[Inst], stack_size: i32) {
    self.link_label(id, insts, stack_size, None, true, true);
  }
  pub(crate) fn link_label(
    &mut self,
    id: LabelId,
    body: &[Inst],
    stack_size: i32,
    end_opt: Option<LabelId>,
    is_function: bool,
    is_return: bool,
  ) {
    let mut insts = vec![Lbl(id)];
    if is_function {
      insts.extend_from_slice(&[Push(Rbp), mov_q(Rbp, Rsp), SubRId(Rsp, stack_size)]);
    }
    insts.extend_from_slice(body);
    if let Some(end) = end_opt {
      if !is_return {
        insts.push(CallApi(self.import(KERNEL32, "ExitProcess")));
      }
      if is_function && is_return {
        insts.extend_from_slice(&[AddRId(Rsp, stack_size), Pop(Rbp), Custom(RET)]);
      }
      insts.push(Lbl(end));
    }
    match self.functions.entry(id) {
      Entry::Occupied(mut entry) => {
        entry.get_mut().insts = insts;
        entry.get_mut().seh = end_opt.map(|end| (end, stack_size));
      }
      Entry::Vacant(entry) => {
        entry.insert(CompiledFunc {
          insts,
          uses: vec![],
          seh: end_opt.map(|end| (end, stack_size)),
          /*id,*/
        });
      }
    }
  }
  pub(crate) fn link_not_return(&mut self, id: LabelId, insts: &[Inst], stack_size: i32) {
    let end = self.id();
    self.link_label(id, insts, stack_size, Some(end), false, false);
  }
  pub(crate) fn link_not_return_function(&mut self, id: LabelId, insts: &[Inst], stack_size: i32) {
    let end = self.id();
    self.link_label(id, insts, stack_size, Some(end), true, false);
  }
  pub(crate) fn use_function(&mut self, caller: LabelId, id: LabelId) {
    self
      .functions
      .entry(caller)
      .and_modify(|asm_func| asm_func.uses.push(id))
      .or_insert(CompiledFunc { insts: vec![], uses: vec![id], seh: None /*id: caller*/ });
  }
}
