use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn drop_all(&mut self, result: Json, scope: &mut Scope) -> ErrOR<()> {
    self.drop_json(result, false, scope);
    self.drop_all_local(scope)?;
    self.drop_global(scope)
  }
  pub(crate) fn drop_all_local(&mut self, scope: &mut Scope) -> ErrOR<()> {
    for _ in 0..scope.locals.len() {
      self.drop_scope(scope)?;
    }
    for (name, local) in take(&mut scope.local_top) {
      if local.val.refs.is_empty() && !name.starts_with('_') {
        self.warn(local.pos, UnusedName(LocalVar, name.clone()))?;
      }
      self.push_symbol(SymbolInfo {
        definition: Some(local.pos),
        name,
        kind: LocalVar,
        json_type: local.val.val.as_type(),
        refs: local.val.refs.clone(),
      });
      self.drop_json(local.val.val, true, scope);
    }
    Ok(())
  }
  fn drop_global(&mut self, scope: &mut Scope) -> ErrOR<()> {
    for (name, global) in take(&mut self.globals) {
      if global.val.refs.is_empty() && !name.starts_with('_') {
        self.warn(global.pos, UnusedName(GlobalVar, name.clone()))?;
      }
      self.push_symbol(SymbolInfo {
        definition: Some(global.pos),
        name,
        kind: GlobalVar,
        json_type: global.val.val.as_type(),
        refs: global.val.refs.clone(),
      });
      if let Some(memory) = global.val.val.memory() {
        self.heap_free(memory, scope);
      }
    }
    Ok(())
  }
  #[expect(clippy::needless_pass_by_value)]
  pub(crate) fn drop_json(&mut self, json: Json, force: bool, scope: &mut Scope) {
    if let Some(memory @ Memory(Local(lifetime, offset), mem_type)) = json.memory()
      && (force || lifetime == Tmp)
    {
      scope.free(offset, mem_type);
      self.heap_free(memory, scope);
    }
  }
  pub(crate) fn drop_scope(&mut self, scope: &mut Scope) -> ErrOR<()> {
    for (name, local) in scope.locals.pop().unwrap_or_default() {
      if local.val.refs.is_empty() && !name.starts_with('_') {
        self.warn(local.pos, UnusedName(LocalVar, name.clone()))?;
      }
      self.push_symbol(SymbolInfo {
        definition: Some(local.pos),
        name,
        kind: LocalVar,
        json_type: local.val.val.as_type(),
        refs: local.val.refs.clone(),
      });
      self.drop_json(local.val.val, true, scope);
    }
    Ok(())
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
