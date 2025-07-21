//! Implementation for `Bind`.
use crate::Bind::{self, Lit, Var};
impl<T> Bind<T> {
  /// Describes name of the `Bind`.
  pub(crate) fn describe(&self, ty: &str) -> String {
    format!(
      "{ty} ({})",
      match self {
        Lit(_) => "Literal",
        Var(name) => name.describe(),
      }
    )
  }
}
