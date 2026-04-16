use crate::prelude::*;
use std::collections::btree_map::Entry;
#[derive(Debug, Clone)]
pub(crate) struct Dependency {
  pub id: LabelId,
  pub uses: Vec<LabelId>,
}
impl Dependency {
  pub(crate) fn reachable(&self, dep_vec: &[&Dependency]) -> BTreeSet<LabelId> {
    let mut stack = vec![self.id];
    let mut reachable = BTreeSet::new();
    while let Some(id) = stack.pop() {
      if !reachable.insert(id) {
        continue;
      }
      if let Some(item) = dep_vec.iter().find(|item| item.id == id) {
        stack.extend_from_slice(&item.uses);
      }
    }
    reachable
  }
}
impl Jsonpiler {
  pub(crate) fn check_unused_functions(&mut self, root_dep: &Dependency) {
    let reachable = root_dep.reachable(
      &self
        .user_defined
        .values()
        .map(|u_d| &u_d.val.dep)
        .chain(self.parsers.iter().map(|parser| &parser.val.dep))
        .collect::<Vec<&Dependency>>(),
    );
    for (name, u_d) in self.user_defined.clone() {
      if !reachable.contains(&u_d.val.dep.id)
        && !name.starts_with('_')
        && !self.parsers[u_d.pos.file as usize].val.exports.contains_key(&name)
      {
        self.warn(u_d.pos, UnusedName(UserDefinedFunc, name.clone()));
      }
    }
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
      if is_return {
        if is_function {
          insts.extend_from_slice(&[AddRId(Rsp, stack_size), Pop(Rbp), Custom(RET)]);
        }
      } else {
        insts.push(CallApi(self.import(KERNEL32, "ExitProcess")));
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
          dep: Dependency { id, uses: vec![] },
          seh: end_opt.map(|end| (end, stack_size)),
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
  pub(crate) fn resolve_calls(&mut self) -> (Vec<Inst>, Vec<(LabelId, LabelId, i32)>) {
    let reachable = self.parsers[0].val.dep.clone().reachable(
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
    (insts, seh)
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
