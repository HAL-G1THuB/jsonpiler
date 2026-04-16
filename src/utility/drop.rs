use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn drop_all_scope(&mut self, scope: &mut Scope) {
    for _ in 0..scope.locals.len() {
      self.drop_scope(scope);
    }
    for (name, local) in take(&mut scope.local_top) {
      if !local.val.used && !name.starts_with('_') {
        self.warn(local.pos, UnusedName(LocalVar, name));
      }
      self.drop_json(local.val.val, scope, true);
    }
  }
  pub(crate) fn drop_global(&mut self, scope: &mut Scope) {
    for (name, global) in take(&mut self.globals) {
      if !global.val.used && !name.starts_with('_') {
        self.warn(global.pos, UnusedName(GlobalVar, name));
      }
      if let Some(memory) = global.val.val.memory() {
        self.heap_free(memory, scope);
      }
    }
  }
  #[expect(clippy::needless_pass_by_value)]
  pub(crate) fn drop_json(&mut self, json: Json, scope: &mut Scope, force: bool) {
    if let Some(memory @ Memory(Local(lifetime, offset), mem_type)) = json.memory()
      && (force || lifetime == Tmp)
    {
      scope.free(offset, mem_type);
      self.heap_free(memory, scope);
    }
  }
  pub(crate) fn drop_scope(&mut self, scope: &mut Scope) {
    for (name, local) in scope.locals.pop().unwrap_or_default() {
      if !local.val.used && !name.starts_with('_') {
        self.warn(local.pos, UnusedName(LocalVar, name));
      }
      self.drop_json(local.val.val, scope, true);
    }
  }
  pub(crate) fn heap_free(&mut self, Memory(addr, mem_type): Memory, scope: &mut Scope) {
    if mem_type.heap == HeapPtr {
      let heap_free = self.import(KERNEL32, "HeapFree");
      scope.extend(&[
        mov_q(Rcx, Global(self.symbols[HEAP])),
        Clear(Rdx),
        mov_q(R8, addr),
        CallApiCheck(heap_free),
        DecMd(Global(self.symbols[LEAK_CNT])),
      ]);
    }
  }
}
