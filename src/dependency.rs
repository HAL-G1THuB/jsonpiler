use crate::prelude::*;
use std::collections::btree_map::Entry;
#[derive(Debug, Clone)]
pub(crate) struct Dependency {
  pub id: LabelId,
  pub uses: Vec<LabelId>,
}
#[derive(Debug, Clone)]
pub(crate) struct Analysis {
  pub symbols: Vec<SymbolInfo>,
}
#[derive(Debug, Clone)]
pub(crate) struct SymbolInfo {
  pub definition: Option<Position>,
  pub json_type: JsonType,
  pub kind: NameKind,
  pub name: String,
  pub refs: Vec<Position>,
}
impl Dependency {
  pub(crate) fn reachable(&self, dep_vec: &[&Dependency]) -> BTreeSet<LabelId> {
    let mut stack = vec![self.id];
    let mut reachable = BTreeSet::new();
    while let Some(id) = stack.pop() {
      if !reachable.insert(id) {
        continue;
      }
      if let Some(dep) = dep_vec.iter().find(|dep| dep.id == id) {
        stack.extend_from_slice(&dep.uses);
      }
    }
    reachable
  }
}
impl Jsonpiler {
  pub(crate) fn check_unused_functions(&mut self, root_dep: &Dependency) -> ErrOR<()> {
    let reachable = root_dep.reachable(
      &self
        .user_defined
        .values()
        .map(|u_d| &u_d.val.dep)
        .chain(self.parsers.iter().map(|parser| &parser.val.dep))
        .collect::<Vec<&Dependency>>(),
    );
    for (name, u_d) in self.user_defined.clone() {
      self.push_symbol(SymbolInfo {
        definition: Some(u_d.pos),
        json_type: FuncT(u_d.val.params.clone(), Box::new(u_d.val.ret_type.clone())),
        kind: UserDefinedFunc,
        name: name.clone(),
        refs: u_d.val.refs.clone(),
      });
      if !reachable.contains(&u_d.val.dep.id)
        && !name.starts_with('_')
        && !self.parsers[u_d.pos.file as usize].val.exports.contains_key(&name)
      {
        self.warn(u_d.pos, UnusedName(UserDefinedFunc, name.clone()))?;
      }
    }
    Ok(())
  }
  pub(crate) fn link_function(&mut self, id: LabelId, insts: &[Inst], stack_size: i32) {
    self.link_label(id, insts, stack_size, true, FN_RETURN);
  }
  pub(crate) fn link_label(
    &mut self,
    id: LabelId,
    body: &[Inst],
    stack_size: i32,
    seh: bool,
    (is_function, do_return): (bool, bool),
  ) {
    let end_opt = seh.then_some(self.id());
    let mut insts = vec![Lbl(id)];
    if is_function {
      insts.extend_from_slice(&[Push(Rbp), mov_q(Rbp, Rsp), SubRId(Rsp, stack_size)]);
    }
    insts.extend_from_slice(body);
    if do_return {
      if is_function {
        insts.extend_from_slice(&[AddRId(Rsp, stack_size), Pop(Rbp), Custom(RET)]);
      }
    } else {
      insts.push(CallApi(self.import(KERNEL32, "ExitProcess")));
    }
    if let Some(end) = end_opt {
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
          dep: Dependency { id, uses: vec![] },
          seh: end_opt.map(|end| (end, stack_size)),
        });
      }
    }
  }
  pub(crate) fn resolve_calls(&mut self) -> ErrOR<(Vec<Inst>, Seh)> {
    let reachable = self.first_parser_mut()?.val.dep.clone().reachable(
      &self.functions.values().map(|compiled| &compiled.dep).collect::<Vec<&Dependency>>(),
    );
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
      .flat_map(|compiled| compiled.insts)
      .collect::<Vec<Inst>>();
    Ok((insts, seh))
  }
  pub(crate) fn use_function(&mut self, caller: LabelId, id: LabelId) {
    match self.functions.entry(caller) {
      Entry::Occupied(mut entry) => entry.get_mut().dep.uses.push(id),
      Entry::Vacant(entry) => {
        entry.insert(CompiledFunc {
          insts: vec![],
          dep: Dependency { id: caller, uses: vec![id] },
          seh: None,
        });
      }
    }
  }
  pub(crate) fn use_u_d(&mut self, caller: LabelId, id: LabelId) -> ErrOR<()> {
    let dep = if let Some(root) = self.parsers.iter_mut().rfind(|root| root.val.dep.id == caller) {
      &mut root.val.dep
    } else if let Some(u_d) = self.user_defined.values_mut().find(|u_d| u_d.val.dep.id == caller) {
      &mut u_d.val.dep
    } else {
      return Err(Internal(UnknownLabel));
    };
    dep.uses.push(id);
    Ok(())
  }
}
