//! Implementation for `Jsonpiler` utility functions
use crate::{
  Bind, ErrOR, FuncInfo, Json, JsonWithPos, Jsonpiler, Position, ScopeInfo, add, err, mn,
};
use core::mem::take;
impl Jsonpiler {
  /// Format error with `^` pointing to the error span.
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
    let start = match (0..=idx).rfind(|&i| self.source.get(i) == Some(&b'\n')) {
      Some(i) => i.saturating_add(1),
      None => 0,
    };
    let end = (idx..len).find(|&i| self.source.get(i) == Some(&b'\n')).unwrap_or(len);
    let line = self.source.get(start..end).unwrap_or(&[]);
    let line_str = String::from_utf8_lossy(line);
    let caret_start = idx.saturating_sub(start);
    let caret_len = pos.size.max(1).min(end.saturating_sub(idx));
    let ws = " ".repeat(caret_start);
    let carets = "^".repeat(caret_len);
    gen_err(&format!("{line_str}\n{ws}{carets}"))
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
      "The {ordinal}{suffix} argument to `{name}` must be of \
      a type `{expected}`, but a value of type `{typ}` was provided."
    )
  }
  /// Generate an error.
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
/// Generates stack string.
#[expect(dead_code, reason = "")]
fn gen_stack_string(value: &str, scope: &mut ScopeInfo) -> ErrOR<()> {
  let mut bytes = value.as_bytes().to_vec();
  bytes.push(0);
  let mut i = 0;
  let total_len = bytes.len();
  let mut name = scope.get_local(total_len)?;
  while i < bytes.len() {
    let remaining = bytes.len().saturating_sub(i);
    if remaining >= 4 {
      let chunk = get_chunk::<4>(&mut bytes, i)?;
      let val = u32::from_le_bytes(chunk);
      scope.body.push(mn!("mov", format!("dword{name}"), format!("{val:#08x}")));
      i = add(i, 4)?;
      name.id = add(name.id, 4)?;
    } else if remaining >= 2 {
      let chunk = get_chunk::<2>(&mut bytes, i)?;
      let val = u16::from_le_bytes(chunk);
      scope.body.push(mn!("mov", format!("word{name}"), format!("{val:#04x}")));
      i = add(i, 2)?;
      name.id = add(name.id, 2)?;
    } else {
      let val = bytes[i];
      scope.body.push(mn!("mov", format!("byte{name}"), format!("{val:#02x}")));
      i = add(i, 1)?;
      name.id = add(name.id, 1)?;
    }
  }
  Ok(())
}
/// Utility functions for `gen_stack_string`.
fn get_chunk<const T: usize>(bytes: &mut [u8], i: usize) -> ErrOR<[u8; T]> {
  bytes[i..add(i, T)?]
    .try_into()
    .map_err(|_err| "InternalError: slice conversion failed in `gen_stack_string`".into())
}
/// Get integer string.
pub(crate) fn get_int_str(int: &Bind<i64>, scope: &mut ScopeInfo) -> ErrOR<String> {
  match int {
    Bind::Lit(l_int) => Ok(l_int.to_string()),
    Bind::Var(name) => name.try_free_and_2str(scope),
  }
}
/// Call function.
#[expect(clippy::single_call_fn, reason = "")]
pub(crate) fn imp_call(func: &str) -> String {
  mn!("call", format!("[qword ptr __imp_{func}[rip]]"))
}
/// Begin scope.
#[expect(clippy::single_call_fn, reason = "")]
pub(crate) fn scope_begin(tmp: &mut ScopeInfo, scope: &mut ScopeInfo) -> ErrOR<()> {
  scope.scope_align = add(scope.scope_align, add(scope.stack_size, 15)? & !15)?;
  tmp.body = take(&mut scope.body);
  tmp.free_map = take(&mut scope.free_map);
  tmp.stack_size = take(&mut scope.stack_size);
  Ok(())
}
/// Begin scope.
#[expect(clippy::single_call_fn, reason = "")]
pub(crate) fn scope_end(tmp: &mut ScopeInfo, scope: &mut ScopeInfo) -> ErrOR<()> {
  let align = add(scope.stack_size, 15)? & !15;
  if align != 0 {
    tmp.body.push(mn!("sub", "rsp", &align.to_string()));
  }
  tmp.body.append(&mut scope.body);
  if align != 0 {
    tmp.body.push(mn!("add", "rsp", &align.to_string()));
  }
  scope.body = take(&mut tmp.body);
  scope.free_map = take(&mut tmp.free_map);
  scope.stack_size = take(&mut tmp.stack_size);
  Ok(())
}
