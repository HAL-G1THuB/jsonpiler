use crate::prelude::*;
use std::collections::btree_map::Entry;
#[derive(Debug, Clone)]
pub(crate) struct Dependency {
  pub id: LabelId,
  uses: BTreeSet<LabelId>,
}
#[derive(Debug, Clone)]
pub(crate) struct CompiledFunc {
  pub dep: Dependency,
  pub insts: AorX,
  pub seh: Option<(LabelId, i32)>,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct Analysis {
  pub symbols: Vec<SymbolInfo>,
}
#[derive(Debug, Clone)]
pub(crate) struct SymbolInfo {
  pub def: Option<Position>,
  pub json_type: JsonType,
  pub kind: NameKind,
  pub name: String,
  pub refs: Vec<Position>,
}
impl SymbolInfo {
  pub(crate) fn def_refs(&self) -> Vec<Position> {
    self.refs.iter().chain(self.def.iter()).copied().collect()
  }
}
impl Dependency {
  pub(crate) fn add(&mut self, id: LabelId) {
    self.uses.insert(id);
  }
  pub(crate) fn new(id: LabelId) -> Self {
    Dependency { id, uses: BTreeSet::new() }
  }
  pub(crate) fn reachable(&self, dep_vec: &Vec<&Dependency>) -> BTreeSet<LabelId> {
    let mut stack = vec![self.id];
    let mut reachable = BTreeSet::new();
    while let Some(id) = stack.pop() {
      if !reachable.insert(id) {
        continue;
      }
      if let Some(dep) = dep_vec.iter().find(|dep| dep.id == id) {
        stack.extend(dep.uses.iter());
      }
    }
    reachable
  }
}
impl Jsonpiler {
  pub(crate) fn build_functions_a(&mut self) -> ErrOR<Vec<Vec<Vec<A64Inst>>>> {
    let reachable = self
      .first_file_mut()?
      .dep
      .clone()
      .reachable(&self.functions.values().map(|compiled| &compiled.dep).collect());
    let mut seh = vec![];
    for compiled in self.functions.values().filter(|compiled| reachable.contains(&compiled.dep.id))
    {
      if let Some((end, stack_size)) = compiled.seh {
        seh.push((compiled.dep.id, end, stack_size));
      }
    }
    let insts = reachable
      .into_iter()
      .filter_map(|id| self.functions.remove(&id))
      .map(|compiled| compiled.insts.a64())
      .collect::<ErrOR<_>>()?;
    Ok(insts)
  }
  pub(crate) fn build_functions_x(&mut self) -> ErrOR<(Vec<Vec<Vec<X64Inst>>>, Seh)> {
    let reachable = self
      .first_file_mut()?
      .dep
      .clone()
      .reachable(&self.functions.values().map(|compiled| &compiled.dep).collect());
    let mut seh = vec![];
    for compiled in self.functions.values() {
      if reachable.contains(&compiled.dep.id)
        && let Some((end, stack_size)) = compiled.seh
      {
        seh.push((compiled.dep.id, end, stack_size));
      }
    }
    let insts = reachable
      .into_iter()
      .filter_map(|id| self.functions.remove(&id))
      .map(|compiled| compiled.insts.x64())
      .collect::<ErrOR<_>>()?;
    Ok((insts, seh))
  }
  pub(crate) fn check_unused_functions(&mut self, file_idx: usize) -> ErrOR<()> {
    let reachable = self.files[file_idx].dep.reachable(
      &self
        .user_defined
        .values()
        .map(|u_d| &u_d.val.dep)
        .chain(self.files.iter().map(|parser| &parser.dep))
        .collect(),
    );
    for (name, u_d) in self.user_defined.clone() {
      if !reachable.contains(&u_d.val.dep.id)
        && !name.starts_with('_')
        && !self.files[u_d.pos.file].exports.contains_key(&name)
      {
        self.warn(u_d.pos.with(UnusedName(UserDefinedFunc, name.clone())))?;
      }
      self.push_sym(SymbolInfo {
        def: Some(u_d.pos),
        json_type: FuncT(u_d.val.sig.clone().into()),
        kind: UserDefinedFunc,
        name,
        refs: u_d.val.refs,
      });
    }
    Ok(())
  }
  pub(crate) fn link_func_a(&mut self, id: LabelId, body: Vec<Vec<A64Inst>>, stack_size: i32) {
    self.link_lbl_a(id, body, stack_size, true);
  }
  pub(crate) fn link_func_x(&mut self, id: LabelId, insts: Vec<Vec<X64Inst>>, stack_size: i32) {
    self.link_lbl_x(id, insts, stack_size, true, FN_RETURN)
  }
  pub(crate) fn link_lbl_a(
    &mut self,
    id: LabelId,
    body: Vec<Vec<A64Inst>>,
    stack_size: i32,
    do_return: bool,
  ) {
    let mut insts = vec![
      vec![LblA(id), Stp(X29, X30, SP, -0x10), SubRI12(X29, SP, 0x10)],
      load_imm_a(X9, i64::from(stack_size)),
      vec![SubR3(X9, X29, X9), AddRI12(SP, X9, 0)],
    ];
    insts.extend(body);
    if do_return {
      insts.extend([
        load_imm_a(X9, i64::from(stack_size)),
        vec![
          AddRI12(X10, SP, 0),
          AddR3(X10, X10, X9),
          AddRI12(Xzr, X10, 0x10),
          Ldp(X29, X30, SP, -0x10),
          RetA,
        ],
      ]);
    } else {
      insts.extend([vec![BApi(self.api(SYS_B, "__exit"))]]);
    }
    match self.functions.entry(id) {
      Entry::Occupied(mut entry) => {
        entry.get_mut().insts = A64(insts);
      }
      Entry::Vacant(entry) => {
        entry.insert(CompiledFunc { insts: A64(insts), dep: Dependency::new(id), seh: None });
      }
    }
  }
  pub(crate) fn link_lbl_x(
    &mut self,
    id: LabelId,
    body: Vec<Vec<X64Inst>>,
    stack_size: i32,
    seh: bool,
    (is_func, do_return): (bool, bool),
  ) {
    let end_opt = seh.then_some(self.id());
    let mut insts = vec![];
    let mut prologue = vec![LblX(id)];
    if is_func {
      prologue.extend_from_slice(&[Push(Rbp), mov(S8, Rbp, Rsp), m8i(Sub, Rsp, stack_size)]);
    }
    insts.push(prologue);
    insts.extend(body);
    let mut epilogue = vec![];
    if do_return {
      if is_func {
        epilogue.extend_from_slice(&[m8i(Add, Rsp, stack_size), Pop(Rbp), RetX]);
      }
    } else {
      epilogue.push(CallApi(self.api(KERNEL32, "ExitProcess")));
    }
    if let Some(end) = end_opt {
      epilogue.push(LblX(end));
    }
    insts.push(epilogue);
    match self.functions.entry(id) {
      Entry::Occupied(mut entry) => {
        entry.get_mut().insts = X64(insts);
        entry.get_mut().seh = end_opt.map(|end| (end, stack_size));
      }
      Entry::Vacant(entry) => {
        entry.insert(CompiledFunc {
          insts: X64(insts),
          dep: Dependency::new(id),
          seh: end_opt.map(|end| (end, stack_size)),
        });
      }
    }
  }
  pub(crate) fn use_func(&mut self, caller: LabelId, id: LabelId) {
    match self.functions.entry(caller) {
      Entry::Occupied(mut entry) => entry.get_mut().dep.add(id),
      Entry::Vacant(entry) => {
        let mut dep = Dependency::new(caller);
        dep.add(id);
        entry.insert(CompiledFunc { insts: X64(vec![]), dep, seh: None });
      }
    }
  }
  pub(crate) fn use_u_d(&mut self, caller: LabelId, id: LabelId, pos: Position) -> ErrOR<()> {
    if let Some(u_d) = self.user_defined.values_mut().find(|u_d| u_d.val.dep.id == id) {
      u_d.val.refs.push(pos);
    }
    if let Some(root) = self.files.iter_mut().rfind(|root| root.dep.id == caller) {
      root.dep.add(id);
      Ok(())
    } else if let Some(u_d) = self.user_defined.values_mut().find(|u_d| u_d.val.dep.id == caller) {
      u_d.val.dep.add(id);
      Ok(())
    } else {
      Err(UnknownLabel.into())
    }
  }
}
