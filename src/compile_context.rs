use crate::{ErrOR, Label, VarKind::Global, add};
use std::collections::{HashMap, HashSet};
#[derive(Debug, Default)]
pub(crate) struct CompileContext {
  includes: HashSet<String>,
  label_id: usize,
  str_cache: HashMap<String, usize>,
}
impl CompileContext {
  pub(crate) fn get_cache(&mut self, value: &str) -> Option<usize> {
    self.str_cache.get(value).copied()
  }
  pub(crate) fn insert_cache(&mut self, value: &str, id: usize) {
    self.str_cache.insert(value.into(), id);
  }
  pub(crate) fn is_not_included(&mut self, name: &str) -> bool {
    self.includes.insert(name.into())
  }
  pub(crate) fn label(&mut self, size: usize) -> ErrOR<Label> {
    let id = self.label_id;
    self.label_id = add!(id, 1)?;
    Ok(Label { id, kind: Global, size })
  }
}
