use crate::{
  ArgLen::{self, Any, AtLeast, AtMost, Exactly, NoArgs, Range, SomeArg},
  ErrOR, FuncInfo, Json, Parser, Position, WithPos, parse_err,
};
impl Parser {
  #[must_use]
  // No risk of overflow
  pub(crate) fn fmt_err(&self, err: &str, pos: &Position) -> String {
    let gen_err = |msg: &str| -> String {
      format!("{err}\nError occurred on line: {}\nError position:\n{msg}", pos.line)
    };
    if self.source.is_empty() {
      return gen_err("\n^");
    }
    let len = self.source.len();
    let index = pos.offset.min(len);
    let start = (0..index).rfind(|&i| self.source[i] == b'\n').map_or(0, |st| st + 1);
    let end = (index..len).find(|&i| self.source[i] == b'\n').unwrap_or(len);
    let line = &self.source[start..end];
    let line_str = String::from_utf8_lossy(line);
    let caret_start = index - start;
    let caret_len = pos.size.max(1).min(end - index);
    let ws = " ".repeat(caret_start);
    let carets = "^".repeat(caret_len);
    gen_err(&format!("{line_str}\n{ws}{carets}"))
  }
  pub(crate) fn typ_err(
    &self, ord: usize, name: &str, expected: &str, json: &WithPos<Json>,
  ) -> ErrOR<Json> {
    let suffix = match ord % 100 {
      11..=13 => "th",
      _ => match ord % 10 {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
      },
    };
    let typ = json.value.type_name();
    parse_err!(
      self,
      &json.pos,
      "The {ord}{suffix} argument to `{name}` must be of \
      a type `{expected}`, but a value of type `{typ}` was provided."
    )
  }
  pub(crate) fn validate_args(&self, func: &FuncInfo, expected: ArgLen) -> ErrOR<()> {
    let supplied = func.len;
    let fmt_require = |text: &str, count_desc: String| -> ErrOR<()> {
      let plural = if supplied == 1 { "" } else { "s" };
      let be = if supplied == 1 { "is" } else { "are" };
      parse_err!(
        self,
        func.pos,
        "`{}` requires {text} {count_desc}, but {supplied} argument{plural} {be} supplied.",
        func.name
      )
    };
    match expected {
      Exactly(n) => {
        if supplied != n {
          return fmt_require("exactly", format!("{n} argument{}", if n == 1 { "" } else { "s" }));
        }
      }
      AtLeast(min) => {
        if supplied < min {
          return fmt_require(
            "at least",
            format!("{min} argument{}", if min == 1 { "" } else { "s" }),
          );
        }
      }
      AtMost(max) => {
        if supplied > max {
          return fmt_require(
            "at most",
            format!("{max} argument{}", if max == 1 { "" } else { "s" }),
          );
        }
      }
      Range(min, max) => {
        if supplied < min || supplied > max {
          return fmt_require("between", format!("{min} and {max} arguments"));
        }
      }
      SomeArg => {
        if supplied == 0 {
          return fmt_require("at least", "1 argument".to_owned());
        }
      }
      NoArgs => {
        if supplied != 0 {
          return fmt_require("exactly", "0 arguments".to_owned());
        }
      }
      Any => (),
    }
    Ok(())
  }
}
