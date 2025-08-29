use crate::{
  Arity::{self, Any, AtLeast, AtMost, Exactly, NoArgs, Range},
  ErrOR, FuncInfo, Json, Parser, Position, WithPos,
};
impl Parser {
  pub(crate) fn args_type_error(
    &self, nth: usize, name: &str, expected: &str, json: &WithPos<Json>,
  ) -> String {
    let suffix = match nth % 100 {
      11..=13 => "th",
      _ => match nth % 10 {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
      },
    };
    let typ = json.value.type_name();
    self.fmt_err(
      &format!(
        "TypeError: {nth}{suffix} argument of `{name}` expected type `{expected}`, but got `{typ}`.",
      ),
      json.pos,
    )
  }
  #[must_use]
  // No risk of overflow
  pub(crate) fn fmt_err(&self, err: &str, pos: Position) -> String {
    let gen_err = |column: usize, msg: &str| -> String {
      format!(
        "{err}\nError at {} line: {} column: {column}\nError position:\n{msg}",
        self.file, pos.line
      )
    };
    if self.source.is_empty() {
      return gen_err(0, "\n^");
    }
    let len = self.source.len();
    let index = pos.offset.min(len);
    let start = (0..index).rfind(|i| self.source[*i] == b'\n').map_or(0, |st| st + 1);
    let end = (index..len).find(|i| self.source[*i] == b'\n').unwrap_or(len);
    let line = &self.source[start..end];
    let line_str = String::from_utf8_lossy(line);
    let caret_start = index - start;
    let caret_len = pos.size.max(1).min(end - index);
    let ws = " ".repeat(caret_start);
    let carets = "^".repeat(caret_len);
    gen_err(pos.offset - start, &format!("{line_str}\n{ws}{carets}"))
  }
  pub(crate) fn fmt_require(&self, text: &str, count_desc: &str, func: &FuncInfo) -> String {
    let plural = if func.len == 1 { "" } else { "s" };
    let be = if func.len == 1 { "is" } else { "are" };
    self.fmt_err(
      &format!(
        "ArityError: `{}` requires {text} {count_desc}, but {} argument{plural} {be} supplied.",
        func.name, func.len
      ),
      func.pos,
    )
  }
  pub(crate) fn type_error(&self, name: &str, expected: &str, json: &WithPos<Json>) -> String {
    let typ = json.value.type_name();
    self.fmt_err(
      &format!("TypeError: `{name}` expected type `{expected}`, but got `{typ}`.",),
      json.pos,
    )
  }
  pub(crate) fn validate_args(&self, func: &FuncInfo, expected: Arity) -> ErrOR<()> {
    let supplied = func.len;
    match expected {
      Exactly(n) => {
        if supplied != n {
          return Err(
            self
              .fmt_require(
                "exactly",
                &format!("{n} argument{}", if n == 1 { "" } else { "s" }),
                func,
              )
              .into(),
          );
        }
      }
      AtLeast(min) => {
        if supplied < min {
          return Err(
            self
              .fmt_require(
                "at least",
                &format!("{min} argument{}", if min == 1 { "" } else { "s" }),
                func,
              )
              .into(),
          );
        }
      }
      AtMost(max) => {
        if supplied > max {
          return Err(
            self
              .fmt_require(
                "at most",
                &format!("{max} argument{}", if max == 1 { "" } else { "s" }),
                func,
              )
              .into(),
          );
        }
      }
      Range(min, max) => {
        if supplied < min || supplied > max {
          return Err(
            self.fmt_require("between", &format!("{min} and {max} arguments"), func).into(),
          );
        }
      }
      NoArgs => {
        if supplied != 0 {
          return Err(self.fmt_require("exactly", "0 arguments", func).into());
        }
      }
      Any => (),
    }
    Ok(())
  }
}
