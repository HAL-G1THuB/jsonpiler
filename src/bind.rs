//! Implementation of the `Bind`.
use crate::{
  Bind::{self, Lit, Var},
  Name,
  VarKind::{Global, Local, Tmp},
};
impl<T> Bind<T> {
  /// Gets name of the `Bind`.
  pub(crate) fn describe(&self, ty: &str) -> String {
    format!(
      "{ty} ({})",
      match self {
        Lit(_) => "Literal",
        Var(name) => match name.var {
          Tmp => "Temporary local variable",
          Local => "Local variable",
          Global => "Global variable",
        },
      }
    )
  }
  /// Gets seed of the `Bind`.
  pub(crate) fn get_seed(&self) -> Option<usize> {
    match self {
      Var(Name { var: Tmp, seed }) => Some(*seed),
      Var(_) | Lit(_) => None,
    }
  }
}
