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
    scope: &mut Scope,
  ) -> ErrOR<()> {
    if let Some(local) = scope.get_var_local(name) {
      return err!(pos, DuplicateName(local.val.kind, name.val.clone()));
    }
    if let Some(global) = self.globals.get(&name.val) {
      return err!(pos, DuplicateName(global.val.kind, name.val.clone()));
    }
    if self.builtin.contains_key(&name.val.as_ref()) {
      return err!(pos, DuplicateName(BuiltInFunc, name.val.clone()));
    }
    if self.user_defined.contains_key(&name.val) {
      return err!(pos, DuplicateName(UserDefinedFunc, name.val.clone()));
    }
    Ok(())
  }
  pub(crate) fn get_var(&mut self, var: &Pos<String>, scope: &mut Scope) -> ErrOR<Pos<Variable>> {
    if let Some(variable) = scope.get_var_local(var).or_else(|| self.globals.get_var(var)) {
      Ok(variable.clone())
    } else {
      err!(var.pos, UndefinedVar(var.val.clone()))
    }
  }
  pub(crate) fn push_symbol(&mut self, symbol: SymbolInfo) {
    if let Some(analysis) = &mut self.analysis {
      analysis.symbols.push(symbol);
    }
  }
}
