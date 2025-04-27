//! Implementation for `Jsonpiler` utility functions
use crate::{ErrOR, Json, JsonWithPos, Jsonpiler, Position, err};
impl Jsonpiler {
  /// Format error.
  #[must_use]
  pub(crate) fn fmt_err(&self, err: &str, pos: &Position) -> String {
    let gen_err = |msg: &str| -> String {
      format!("{err}\nError occurred on line: {}\nError position:\n{msg}", pos.line)
    };
    if self.source.is_empty() {
      return gen_err("\n^");
    }
    let len = self.source.len();
    let idx = pos.offset.min(len.saturating_sub(1));
    let start = if idx == 0 {
      0
    } else {
      let Some(left) = self.source.get(..idx) else {
        return gen_err("Error: Failed to get substring");
      };
      match left.rfind('\n') {
        None => 0,
        Some(start_offset) => {
          let Some(res) = start_offset.checked_add(1) else {
            return gen_err("Error: Overflow");
          };
          res
        }
      }
    };
    let Some(right) = self.source.get(idx..) else {
      return gen_err("Error: Failed to get substring");
    };
    let end = match right.find('\n') {
      None => len,
      Some(end_offset) => {
        let Some(res) = idx.checked_add(end_offset) else {
          return gen_err("Error: Overflow");
        };
        res
      }
    };
    let ws = " ".repeat(idx.saturating_sub(start));
    let Some(result) = self.source.get(start..end) else {
      return gen_err("Error: Failed to get substring");
    };
    gen_err(&format!("{result}\n{ws}^"))
  }
  /// Generates a type error.
  pub(crate) fn typ_err(
    &self, ordinal: usize, name: &str, expected: &str, json: &JsonWithPos,
  ) -> ErrOR<Json> {
    let suffix = match ordinal % 100 {
      11..=13 => "th",
      _ => match ordinal % 10 {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
      },
    };
    let typ = json.value.type_name();
    err!(
      self,
      &json.pos,
      "The {ordinal}{suffix} argument to `{name}` must be of a type `{expected}`, \
      but a value of type `{typ}` was provided."
    )
  }
  /// Generate an error.
  pub(crate) fn validate_args(
    &self, name: &str, at_least: bool, expected: usize, supplied: usize, pos: &Position,
  ) -> ErrOR<()> {
    let fmt_require = |text: &str| -> ErrOR<()> {
      let (plural, be) = if expected == 1 { ("", "is") } else { ("s", "are") };
      err!(
        self,
        pos,
        "`{name}` requires {text} {expected} argument{plural}, \
        but {supplied} argument{plural} {be} supplied.",
      )
    };
    if at_least {
      if supplied >= expected { Ok(()) } else { fmt_require("at least") }
    } else if expected == supplied {
      Ok(())
    } else {
      fmt_require("exactly")
    }
  }
}
