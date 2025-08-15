use std::collections::HashMap;
#[derive(Default)]
pub(crate) struct CompileContext {
  pub label_id: usize,
  pub str_cache: HashMap<String, usize>,
}
impl CompileContext {
  // Overflow is unlikely.
  pub(crate) fn gen_id(&mut self) -> usize {
    let id = self.label_id;
    self.label_id += 1;
    id
  }
}
