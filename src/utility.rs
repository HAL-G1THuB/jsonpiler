use crate::{Bind, ErrOR, Jsonpiler, Label, ScopeInfo, VarKind::Global, add, mn};
use core::fmt::LowerHex;
impl Jsonpiler {
  pub(crate) fn get_bss(&mut self, size: usize) -> ErrOR<Label> {
    let label = self.get_label(size)?;
    self.bss.push(mn!(".lcomm", label.to_ref(), size));
    Ok(label)
  }
  pub(crate) fn get_global_num<T: LowerHex>(&mut self, value: T) -> ErrOR<Label> {
    let label = self.get_label(8)?;
    self.data.push(mn!(".align", "8"));
    self.data.push(label.to_def());
    self.data.push(mn!(".quad", format!("{value:#x}")));
    Ok(label)
  }
  pub(crate) fn get_global_str(&mut self, value: &str) -> ErrOR<Label> {
    if let Some(cached_label) = self.str_cache.get(value) {
      return Ok(Label { kind: Global, id: *cached_label, size: 8 });
    }
    let label = self.get_label(8)?;
    self.str_cache.insert(value.to_owned(), label.id);
    self.data.push(label.to_def());
    self.data.push(mn!(".string", format!("\"{value}\"")));
    Ok(label)
  }
  pub(crate) fn get_label(&mut self, size: usize) -> ErrOR<Label> {
    let id = self.label_id;
    self.label_id = add!(id, 1)?;
    Ok(Label { id, kind: Global, size })
  }
}
pub(crate) fn get_int_str(int: &Bind<i64>, scope: &mut ScopeInfo) -> ErrOR<String> {
  match int {
    Bind::Lit(l_int) => Ok(l_int.to_string()),
    Bind::Var(label) => label.try_free_and_2str(scope),
  }
}
pub(crate) fn get_bool_str(boolean: &Bind<bool>, scope: &mut ScopeInfo) -> ErrOR<String> {
  match boolean {
    Bind::Lit(l_bool) => Ok(if *l_bool { -1i8 } else { 0i8 }.to_string()),
    Bind::Var(label) => label.try_free_and_2str(scope),
  }
}
#[expect(clippy::single_call_fn, reason = "")]
pub(crate) fn imp_call(func: &str) -> String {
  mn!("call", format!("[qword ptr __imp_{func}[rip]]"))
}
