pub(crate) mod consts;
pub(crate) mod json;
pub(crate) mod macros;
pub(crate) mod move_json;
pub(crate) mod other;
pub(crate) mod scope;
use crate::prelude::*;
use std::{
  env,
  time::{SystemTime, UNIX_EPOCH},
};
impl Jsonpiler {
  pub(crate) fn bss(&mut self, size: u32, align: u32) -> u32 {
    let id = self.id();
    self.data.push(BssLbl(id, size, align));
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
  pub(crate) fn check_defined(&self, name: &str, pos: Position, scope: &mut Scope) -> ErrOR<()> {
    if scope.get_var_local(name).is_some() {
      return err!(pos, DuplicateName(LocalVar, name.to_owned()));
    }
    if self.globals.contains_key(name) {
      return err!(pos, DuplicateName(GlobalVar, name.to_owned()));
    }
    if self.builtin.contains_key(name) {
      return err!(pos, DuplicateName(BuiltInFunc, name.to_owned()));
    }
    if self.user_defined.contains_key(name) {
      return err!(pos, DuplicateName(UserDefinedFunc, name.to_owned()));
    }
    Ok(())
  }
  pub(crate) fn get_global(&mut self, name: &str) -> Option<Json> {
    let global = self.globals.get_mut(name)?;
    global.val.used = true;
    Some(global.val.val.clone())
  }
  pub(crate) fn get_var(&mut self, var: &WithPos<String>, scope: &mut Scope) -> ErrOR<Json> {
    if let Some(val) = scope.get_var_local(&var.val).or_else(|| self.get_global(&var.val)) {
      Ok(val)
    } else {
      err!(var.pos, UndefinedVar(var.val.clone()))
    }
  }
  pub(crate) fn global_b(&mut self, boolean: bool) -> Memory {
    let id = self.id();
    self.data.push(if boolean { Byte(id, bool2byte(boolean)) } else { BssLbl(id, 1, 1) });
    Memory(Global(id), Size(1))
  }
  pub(crate) fn global_q(&mut self, value: u64) -> Memory {
    let id = self.id();
    self.data.push(if value != 0 { Quad(id, value) } else { BssLbl(id, 8, 8) });
    Memory(Global(id), Size(8))
  }
  pub(crate) fn global_str<T: Into<String>>(&mut self, value: T) -> u32 {
    self.cache_string(value.into(), false)
  }
  pub(crate) fn global_w_chars<T: Into<String>>(&mut self, value: T) -> u32 {
    self.cache_string(value.into(), true)
  }
  pub(crate) fn heap_free(&mut self, addr: Address, scope: &mut Scope) {
    let heap_free = self.import(KERNEL32, "HeapFree");
    scope.extend(&[
      mov_q(Rcx, Global(self.symbols[HEAP])),
      Clear(Rdx),
      mov_q(R8, addr),
      CallApiCheck(heap_free),
      DecMd(Global(self.symbols[LEAK_CNT])),
    ]);
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
#[expect(clippy::cast_possible_truncation)]
pub(crate) fn time_stamp() -> u32 {
  SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as u32
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
pub(crate) fn full_path(file: &str) -> Result<String, io::Error> {
  Ok(env::current_dir()?.join(Path::new(file)).canonicalize()?.to_string_lossy().to_string())
}
pub(crate) fn op_precedence(op: &str) -> Option<usize> {
  OP_PRECEDENCE.iter().position(|ops| ops.contains(&op))
}
