pub(crate) mod consts;
pub(crate) mod data_lbl;
pub(crate) mod drop;
pub(crate) mod json;
pub(crate) mod macros;
pub(crate) mod memory;
pub(crate) mod move_json;
pub(crate) mod other;
pub(crate) mod scope;
pub(crate) mod var_table;
use crate::prelude::*;
use std::time;
pub(crate) fn now() -> time::Duration {
  time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap_or_default()
}
pub(crate) fn len_u32<T>(data: &[T]) -> ErrOR<u32> {
  Ok(u32::try_from(data.len())?)
}
pub(crate) fn bool2byte(boolean: bool) -> u8 {
  if boolean { 0xFF } else { 0 }
}
pub(crate) fn ascii2hex(byte: u8) -> Option<u8> {
  match byte {
    b'0'..=b'9' => Some(byte - b'0'),
    b'a'..=b'f' => Some(byte - b'a' + 10),
    b'A'..=b'F' => Some(byte - b'A' + 10),
    _ => None,
  }
}
pub(crate) fn align_up_u32(num: u32, align: u32) -> ErrOR<u32> {
  num.div_ceil(align).checked_mul(align).ok_or(Internal(InternalOverFlow))
}
