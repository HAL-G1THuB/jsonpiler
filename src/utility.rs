pub(crate) mod consts;
pub(crate) mod drop;
pub(crate) mod json;
pub(crate) mod macros;
pub(crate) mod move_json;
pub(crate) mod other;
pub(crate) mod scope;
use crate::prelude::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
pub(crate) trait VarTable<T: Ord> {
  fn get_var(&mut self, name: &Pos<T>) -> Option<&Pos<Variable>>;
}
impl<T: Ord> VarTable<T> for BTreeMap<T, Pos<Variable>> {
  fn get_var(&mut self, name: &Pos<T>) -> Option<&Pos<Variable>> {
    let var = self.get_mut(&name.val)?;
    var.val.refs.push(name.pos);
    self.get(&name.val)
  }
}
impl Jsonpiler {
  pub(crate) fn bss(&mut self, size: u32, align: u32) -> u32 {
    let id = self.id();
    self.data.push(BssLbl(id, size, align));
    id
  }
  pub(crate) fn bss_symbol(&mut self, symbol: &'static str, size: u32) -> u32 {
    let id = self.bss(size, size);
    self.symbols.insert(symbol, id);
    id
  }
  pub(crate) fn cache_string(&mut self, string: String, wide: bool) -> u32 {
    if let Some(&id) = self.str_cache.get(&string) {
      return id;
    }
    let id = self.id();
    self.str_cache.insert(string.clone(), id);
    self.data.push(if wide { WStrLbl(id, string) } else { StrLbl(id, string) });
    id
  }
  pub(crate) fn check_defined(
    &self,
    name: &Pos<String>,
    pos: Position,
    scope: &mut Scope,
  ) -> ErrOR<()> {
    if let Some(local) = scope.get_var_local(name) {
      return err!(pos, DuplicateName(local.val.kind, name.val.clone()));
    }
    if let Some(global) = self.globals.get(&name.val) {
      return err!(pos, DuplicateName(global.val.kind, name.val.clone()));
    }
    if self.builtin.contains_key(&name.val.as_ref()) {
      return err!(pos, DuplicateName(BuiltInFunc, name.val.clone()));
    }
    if self.user_defined.contains_key(&name.val) {
      return err!(pos, DuplicateName(UserDefinedFunc, name.val.clone()));
    }
    Ok(())
  }
  pub(crate) fn get_var(&mut self, var: &Pos<String>, scope: &mut Scope) -> ErrOR<Pos<Variable>> {
    if let Some(variable) = scope.get_var_local(var).or_else(|| self.globals.get_var(var)) {
      Ok(variable.clone())
    } else {
      err!(var.pos, UndefinedVar(var.val.clone()))
    }
  }
  pub(crate) fn global_b(&mut self, boolean: bool) -> Memory {
    let id = self.id();
    self.data.push(if boolean { Byte(id, bool2byte(boolean)) } else { BssLbl(id, 1, 1) });
    Memory(Global(id), MemoryType { heap: Value, size: Small(RB) })
  }
  pub(crate) fn global_q(&mut self, value: u64) -> Memory {
    let id = self.id();
    self.data.push(if value != 0 { Quad(id, value) } else { BssLbl(id, 8, 8) });
    Memory(Global(id), MemoryType { heap: Value, size: Small(RQ) })
  }
  pub(crate) fn global_str<T: Into<String>>(&mut self, value: T) -> u32 {
    self.cache_string(value.into(), false)
  }
  pub(crate) fn global_w_chars<T: Into<String>>(&mut self, value: T) -> u32 {
    self.cache_string(value.into(), true)
  }
  // Overflow is unlikely
  pub(crate) fn id(&mut self) -> u32 {
    self.id_seed += 1;
    self.id_seed
  }
  #[expect(clippy::cast_possible_truncation)]
  pub(crate) fn import(&mut self, dll: &str, func: &str) -> Api {
    let idx = self.dlls.iter().position(|(dll2, _)| *dll2 == dll).unwrap_or_else(|| {
      self.dlls.push((dll.to_owned(), vec![]));
      self.dlls.len() - 1
    });
    let idx2 = self.dlls[idx].1.iter().position(|func2| *func2 == func).unwrap_or_else(|| {
      self.dlls[idx].1.push(func.to_owned());
      self.dlls[idx].1.len() - 1
    });
    (idx as u32, idx2 as u32)
  }
  pub(crate) fn push_parser(&mut self, source: String, file: String) {
    let parser = <Pos<Parser>>::new(source, self.parsers.len() as u32, file, self.id());
    self.parsers.push(parser);
  }
  pub(crate) fn push_symbol(&mut self, symbol: SymbolInfo) {
    if let Some(analysis) = &mut self.analysis {
      analysis.symbols.push(symbol);
    }
  }
}
pub(crate) fn align_up(num: usize, align: usize) -> ErrOR<usize> {
  num.div_ceil(align).checked_mul(align).ok_or(Internal(InternalOverFlow))
}
pub(crate) fn align_up_u32(num: u32, align: u32) -> ErrOR<u32> {
  num.div_ceil(align).checked_mul(align).ok_or(Internal(InternalOverFlow))
}
pub(crate) fn align_down_i32(num: i32, align: i32) -> ErrOR<i32> {
  num.div_euclid(align).checked_mul(align).ok_or(Internal(InternalOverFlow))
}
pub(crate) fn now() -> Duration {
  SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default()
}
pub(crate) fn len_u32<T>(data: &[T]) -> ErrOR<u32> {
  Ok(u32::try_from(data.len())?)
}
pub(crate) fn r_size(data: u32) -> ErrOR<u32> {
  align_up_u32(data, FILE_ALIGNMENT)
}
pub(crate) fn bool2byte(boolean: bool) -> u8 {
  if boolean { 0xFF } else { 0 }
}
pub(crate) fn op_precedence(op: &str) -> Option<usize> {
  OP_PRECEDENCE.iter().position(|ops| ops.contains(&op))
}
pub(crate) fn ascii2hex(byte: u8) -> Option<u8> {
  match byte {
    b'0'..=b'9' => Some(byte - b'0'),
    b'a'..=b'f' => Some(byte - b'a' + 10),
    b'A'..=b'F' => Some(byte - b'A' + 10),
    _ => None,
  }
}
