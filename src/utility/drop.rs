use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn call_alloc_r8_x(&mut self) -> ErrOR<Vec<X64Inst>> {
    let leak = Global(self.get_leak()?);
    let heap = Global(self.get_heap()?);
    Ok(vec![
      load(S8, Rcx, heap),
      MovMId(Reg(Rdx), 8),
      CallApiCheck(self.api(KERNEL32, "HeapAlloc")),
      IncMd(leak),
    ])
  }
  pub(crate) fn call_alloc_x0_a(&mut self) -> ErrOR<Vec<A64Inst>> {
    let leak = Global(self.get_leak()?).v_rd();
    let alloc = self.api(SYS_B, "_malloc");
    let mut insts = vec![BApi(alloc)];
    insts.extend_from_slice(&inc_a(leak, X1, X2)?);
    Ok(insts)
  }
  pub(crate) fn call_api_minus_one_a(
    &mut self,
    dylib: &'static str,
    api: &'static str,
  ) -> Vec<A64Inst> {
    vec![
      BApi(self.api(dylib, api)),
      AddRI12(X0, X0, 1),
      CmpRR(X0, Xzr),
      BCc(E.into(), self.handlers.os),
      SubRI12(X0, X0, 1),
    ]
  }
  pub(crate) fn call_api_zero_a(&mut self, dylib: &'static str, api: &'static str) -> Vec<A64Inst> {
    vec![BApi(self.api(dylib, api)), CmpRR(X0, Xzr), BCc(E.into(), self.handlers.os)]
  }
  pub(crate) fn call_free_r8_x(&mut self) -> ErrOR<Vec<X64Inst>> {
    let leak = Global(self.get_leak()?);
    let heap = Global(self.get_heap()?);
    Ok(vec![
      load(S8, Rcx, heap),
      Clear(Rdx),
      CallApiCheck(self.api(KERNEL32, "HeapFree")),
      DecMd(leak),
    ])
  }
  pub(crate) fn call_free_x0_a(&mut self) -> ErrOR<Vec<A64Inst>> {
    let leak = Global(self.get_leak()?).v_rd();
    let free = self.api(SYS_B, "_free");
    let mut insts = vec![BApi(free)];
    insts.extend_from_slice(&load_a(X0, leak)?);
    insts.push(SubRI12(X0, X0, 1));
    insts.extend_from_slice(&store_a(leak, X1, X0)?);
    Ok(insts)
  }
  pub(crate) fn drop_all(&mut self, result: Json, scope: &mut Scope) -> ErrOR<()> {
    self.drop_json(&result, false, scope)?;
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
        if let Some(mem) = json.mem() { jsonpiler.heap_free(mem, scope_2) } else { Ok(()) }
      },
      scope,
    )
  }
  pub(crate) fn drop_json(&mut self, json: &Json, force: bool, scope: &mut Scope) -> ErrOR<()> {
    if let Some(mem @ Memory(Local(lifetime, offset), mem_type)) = json.mem()
      && (force || lifetime == Tmp)
    {
      scope.free(offset, mem_type.size()?)?;
      self.heap_free(mem, scope)?;
    }
    Ok(())
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
    F: FnMut(&mut Jsonpiler, &Json, &mut Scope) -> ErrOR<()>,
  {
    for (name, variable) in var_table {
      if variable.val.refs.is_empty() && !name.starts_with('_') {
        self.warn(variable.pos.with(UnusedName(variable.val.kind, name.clone())))?;
      }
      self.push_sym(SymbolInfo {
        def: Some(variable.pos),
        name,
        kind: variable.val.kind,
        json_type: variable.val.val.as_type(),
        refs: variable.val.refs,
      });
      free(self, &variable.val.val, scope)?;
    }
    Ok(())
  }
  pub(crate) fn heap_free(&mut self, mem: Memory, scope: &mut Scope) -> ErrOR<()> {
    if mem.1.pass_by == HeapPtr {
      if self.flags.a64 {
        scope.e_a(self.heap_free_a(mem)?)?;
      } else {
        scope.e_x(self.heap_free_x(mem)?)?;
      }
    }
    Ok(())
  }
  pub(crate) fn heap_free_a(&mut self, mem: Memory) -> ErrOR<Vec<A64Inst>> {
    Ok(if mem.1.pass_by == HeapPtr {
      [load_a(X0, mem)?, self.call_free_x0_a()?].concat()
    } else {
      vec![]
    })
  }
  pub(crate) fn heap_free_x(&mut self, mem: Memory) -> ErrOR<Vec<X64Inst>> {
    Ok(if mem.1.pass_by == HeapPtr {
      [vec![load(S8, R8, mem.0)], self.call_free_r8_x()?].concat()
    } else {
      vec![]
    })
  }
}
