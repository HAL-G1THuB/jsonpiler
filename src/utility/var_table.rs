use crate::prelude::*;
#[derive(Default, Debug, Clone)]
pub(crate) struct Variable {
  pub kind: NameKind,
  pub refs: Vec<Position>,
  pub val: Json,
}
impl Variable {
  pub(crate) fn new(val: Json, kind: NameKind) -> Self {
    Variable { val, kind, refs: vec![] }
  }
}
pub(crate) trait VarTable<T: Ord> {
  fn get_var(&mut self, name: &Pos<T>) -> Option<&Pos<Variable>>;
}
impl<T: Ord> VarTable<T> for BTreeMap<T, Pos<Variable>> {
  fn get_var(&mut self, name: &Pos<T>) -> Option<&Pos<Variable>> {
    let var = self.get_mut(&name.val)?;
    var.val.refs.push(name.pos);
    self.get(&name.val)
  }
}
impl Jsonpiler {
  pub(crate) fn check_defined(
    &self,
    name: &Pos<String>,
    pos: Position,
    kind: NameKind,
    scope: &mut Scope,
  ) -> ErrOR<()> {
    let dup_err =
      |first_kind| err!(pos, DuplicateName { first: first_kind, kind, name: name.val.clone() });
    if let Some(local) = scope.get_var_local(name) {
      return dup_err(local.val.kind);
    }
    if let Some(global) = self.globals.get(&name.val) {
      return dup_err(global.val.kind);
    }
    if self.builtin.contains_key(&name.val) {
      return dup_err(BuiltInFunc);
    }
    if self.user_defined.contains_key(&name.val) {
      return dup_err(UserDefinedFunc);
    }
    Ok(())
  }
  pub(crate) fn get_var(&mut self, name: &Pos<String>, scope: &mut Scope) -> ErrOR<Pos<Variable>> {
    if let Some(var) = scope.get_var_local(name).or_else(|| self.globals.get_var(name)) {
      Ok(var.clone())
    } else {
      err!(name.pos, UndefinedVar(name.val.clone()))
    }
  }
  pub(crate) fn push_sym(&mut self, symbol: SymbolInfo) {
    if let Some(analysis) = &mut self.analysis {
      analysis.symbols.push(symbol);
    }
  }
}
