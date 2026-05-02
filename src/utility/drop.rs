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
    self.drop_var_table(
      take(&mut scope.local_top),
      |jsonpiler, json, scope_2| jsonpiler.drop_json(json, true, scope_2),
      scope,
    )
  }
  fn drop_global(&mut self, scope: &mut Scope) -> ErrOR<()> {
    let globals = take(&mut self.globals);
    self.drop_var_table(
      globals,
      |jsonpiler, json, scope_2| {
        if let Some(memory) = json.memory() {
          jsonpiler.heap_free(memory, scope_2);
        }
      },
      scope,
    )
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
    self.drop_var_table(
      scope.locals.pop().unwrap_or_default(),
      |jsonpiler, json, scope_2| jsonpiler.drop_json(json, true, scope_2),
      scope,
    )
  }
  fn drop_var_table<F>(
    &mut self,
    var_table: BTreeMap<String, Pos<Variable>>,
    mut free: F,
    scope: &mut Scope,
  ) -> ErrOR<()>
  where
    F: FnMut(&mut Jsonpiler, Json, &mut Scope),
  {
    for (name, variable) in var_table {
      if variable.val.refs.is_empty() && !name.starts_with('_') {
        self.warn(variable.pos, UnusedName(variable.val.kind, name.clone()))?;
      }
      self.push_symbol(SymbolInfo {
        definition: Some(variable.pos),
        name,
        kind: variable.val.kind,
        json_type: variable.val.val.as_type(),
        refs: variable.val.refs,
      });
      free(self, variable.val.val, scope);
    }
    Ok(())
  }
  pub(crate) fn heap_free(&mut self, Memory(addr, mem_type): Memory, scope: &mut Scope) {
    if mem_type.heap == HeapPtr {
      scope.extend(&[
        mov_q(Rcx, Global(self.symbols[HEAP])),
        Clear(Rdx),
        mov_q(R8, addr),
        CallApiCheck(self.api(KERNEL32, "HeapFree")),
        DecMd(Global(self.symbols[LEAK_CNT])),
      ]);
    }
  }
}
