//! Implementation for `Jsonpiler` utility functions
use crate::{
  Bind, ErrOR, FuncInfo, Json, JsonWithPos, Jsonpiler, Position, ScopeInfo, add, err, mn,
};
use core::mem::take;
impl Jsonpiler {
  #[must_use]
  pub(crate) fn fmt_err(&self, err: &str, pos: &Position) -> String {
    let gen_err = |msg: &str| -> String {
      format!("{err}\nError occurred on line: {}\nError position:\n{msg}", pos.line)
    };
    if self.source.is_empty() {
      return gen_err("\n^");
    }
    let len = self.source.len();
    let index = pos.offset.min(len);
    let start = (0..index).rfind(|&i| self.source[i] == b'\n').unwrap_or(0).saturating_add(1);
    let end = (index..len).find(|&i| self.source[i] == b'\n').unwrap_or(len);
    let line = &self.source[start..end];
    let line_str = String::from_utf8_lossy(line);
    let caret_start = index.saturating_sub(start);
    let caret_len = pos.size.max(1).min(end.saturating_sub(index));
    let ws = " ".repeat(caret_start);
    let carets = "^".repeat(caret_len);
    gen_err(&format!("{line_str}\n{ws}{carets}"))
  }
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
      "The {ordinal}{suffix} argument to `{name}` must be of \
      a type `{expected}`, but a value of type `{typ}` was provided."
    )
  }
  pub(crate) fn validate_args(
    &self, args: &FuncInfo, at_least: bool, expected: usize,
  ) -> ErrOR<()> {
    let supplied = args.args.len();
    let fmt_require = |text: &str| -> ErrOR<()> {
      let (plural, be) = if expected == 1 { ("", "is") } else { ("s", "are") };
      err!(
        self,
        args.pos,
        "`{1}` requires {text} {expected}{0}, but {supplied}{0} {be} supplied.",
        format!(" argument{plural}"),
        args.name
      )
    };
    if at_least && supplied < expected {
      fmt_require("at least")
    } else if !at_least && expected != supplied {
      fmt_require("exactly")
    } else {
      Ok(())
    }
  }
}
pub(crate) fn get_int_str(int: &Bind<i64>, scope: &mut ScopeInfo) -> ErrOR<String> {
  match int {
    Bind::Lit(l_int) => Ok(l_int.to_string()),
    Bind::Var(name) => name.try_free_and_2str(scope),
  }
}
#[expect(clippy::single_call_fn, reason = "")]
pub(crate) fn imp_call(func: &str) -> String {
  mn!("call", format!("[qword ptr __imp_{func}[rip]]"))
}
#[expect(clippy::single_call_fn, reason = "")]
pub(crate) fn scope_begin(tmp: &mut ScopeInfo, scope: &mut ScopeInfo) -> ErrOR<()> {
  scope.scope_align = add!(scope.scope_align, add!(scope.stack_size, 15)? & !15)?;
  tmp.body = take(&mut scope.body);
  tmp.free_map = take(&mut scope.free_map);
  tmp.bool_map = take(&mut scope.bool_map);
  tmp.stack_size = take(&mut scope.stack_size);
  Ok(())
}
#[expect(clippy::single_call_fn, reason = "")]
pub(crate) fn scope_end(tmp: &mut ScopeInfo, scope: &mut ScopeInfo) -> ErrOR<()> {
  let align = add!(scope.stack_size, 15)? & !15;
  if align != 0 {
    tmp.body.push(mn!("sub", "rsp", &align.to_string()));
  }
  tmp.body.append(&mut scope.body);
  if align != 0 {
    tmp.body.push(mn!("add", "rsp", &align.to_string()));
  }
  scope.body = take(&mut tmp.body);
  scope.free_map = take(&mut tmp.free_map);
  scope.bool_map = take(&mut tmp.bool_map);
  scope.stack_size = take(&mut tmp.stack_size);
  Ok(())
}
