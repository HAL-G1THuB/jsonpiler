//! Implementation for `Jsonpiler` utility functions
use crate::{
  Bind, ErrOR, FuncInfo, Json, JsonWithPos, Jsonpiler, Position, VarKind::Tmp, add, err,
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
    let end = match (idx..len).find(|&i| self.source.get(i) == Some(&b'\n')) {
      Some(i) => i,
      None => len,
    };
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
    &self, name: &str, at_least: bool, expected: usize, supplied: usize, pos: &Position,
  ) -> ErrOR<()> {
    let fmt_require = |text: &str| -> ErrOR<()> {
      let (plural, be) = if expected == 1 { ("", "is") } else { ("s", "are") };
      err!(
        self,
        pos,
        "`{name}` requires {text} {expected}{0}, but {supplied}{0} {be} supplied.",
        format!(" argument{plural}")
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
fn gen_stack_string(value: &str, info: &mut FuncInfo) -> ErrOR<()> {
  const MOV: &str = "  mov ";
  let mut bytes = value.as_bytes().to_vec();
  bytes.push(0);
  let mut i = 0;
  let total_len = bytes.len();
  let mut offset = info.get_local(total_len)?;
  while i < bytes.len() {
    let remaining = bytes.len().saturating_sub(i);
    if remaining >= 4 {
      let mut chunk = [0u8; 4];
      chunk.copy_from_slice(bytes.get(i..add(i, 4)?).ok_or("InternalError: `gen_stack_string`")?);
      let val = u32::from_le_bytes(chunk);
      info.body.push(format!("{MOV}dword{offset}, 0x{val:08x}\n"));
      i = add(i, 4)?;
      offset.seed = add(offset.seed, 4)?;
    } else if remaining >= 2 {
      let mut chunk = [0u8; 2];
      chunk.copy_from_slice(bytes.get(i..add(i, 2)?).ok_or("InternalError: `gen_stack_string`")?);
      let val = u16::from_le_bytes(chunk);
      info.body.push(format!("{MOV}word{offset}, 0x{val:04x}\n"));
      i = add(i, 2)?;
      offset.seed = add(offset.seed, 2)?;
    } else {
      let val = bytes.get(i).ok_or("InternalError: `gen_stack_string`")?;
      info.body.push(format!("{MOV}byte{offset}, 0x{val:02x}\n"));
      i = add(i, 1)?;
      offset.seed = add(offset.seed, 1)?;
    }
  }
  Ok(())
}
/// Get integer string.
pub(crate) fn get_int_str(int: &Bind<i64>, info: &mut FuncInfo) -> ErrOR<String> {
  match int {
    Bind::Lit(l_int) => Ok(l_int.to_string()),
    Bind::Var(name) => {
      if name.var == Tmp {
        info.free(name.seed, 8)?;
      }
      Ok(format!("qword{name}"))
    }
  }
}

/// Call function.
#[expect(clippy::single_call_fn, reason = "")]
pub(crate) fn imp_call(func: &str) -> String {
  format!("  call [qword ptr __imp_{func}[rip]]\n")
}
/// Write mnemonic.
pub(crate) fn mn(mne: &str, args: &[&str]) -> String {
  if args.is_empty() { format!("  {mne}\n") } else { format!("  {mne} {}\n", args.join(", ")) }
}
/// Begin scope.
#[expect(clippy::single_call_fn, reason = "")]
pub(crate) fn scope_begin(tmp: &mut FuncInfo, info: &mut FuncInfo) -> ErrOR<()> {
  info.scope_align = add(info.scope_align, add(info.stack_size, 15)? & !15)?;
  tmp.body = take(&mut info.body);
  tmp.free_map = take(&mut info.free_map);
  tmp.stack_size = take(&mut info.stack_size);
  Ok(())
}
/// Begin scope.
#[expect(clippy::single_call_fn, reason = "")]
pub(crate) fn scope_end(tmp: &mut FuncInfo, info: &mut FuncInfo) -> ErrOR<()> {
  let align = add(info.stack_size, 15)? & !15;
  if align != 0 {
    tmp.body.push(format!("  sub rsp, {align}\n"));
  }
  tmp.body.append(&mut info.body);
  if align != 0 {
    tmp.body.push(format!("  add rsp, {align}\n"));
  }
  info.body = take(&mut tmp.body);
  info.free_map = take(&mut tmp.free_map);
  info.stack_size = take(&mut tmp.stack_size);
  Ok(())
}
