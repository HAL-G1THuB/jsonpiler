use crate::Bind::{self, Lit, Var};
impl<T> Bind<T> {
  pub(crate) fn describe(&self, ty: &str) -> String {
    format!(
      "{ty} ({})",
      match self {
        Lit(_) => "Literal",
        Var(label) => label.describe(),
      }
    )
  }
}
