#[macro_export]
#[doc(hidden)]
macro_rules! add {
  ($op1: expr, $op2: expr) => {
    $op1.checked_add($op2).ok_or("InternalError: Overflow occurred")
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! def_mod_and_register {
  ($($name:ident),* $(,)?) => {
    $(
      mod $name;
    )*
    use super::{
  ArgLen,
  Bind::{Lit, Var},
  Builtin, ErrOR, FuncInfo, JFunc, Json, Jsonpiler, ScopeInfo, WithPos, err, mn,
  mn_write,
  utility::{get_int_str_without_free, imp_call},
};
use core::mem::take;
use std::{
  collections::VecDeque,
  io::Write as _,
};
    impl Jsonpiler {
      pub(crate) fn register_all(&mut self) {
        $(
          self.$name();
        )*
      }
    }
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! parse_err {
  ($self:ident, $pos:expr, $($arg:tt)*) => {Err($self.fmt_err(&format!($($arg)*), &$pos).into())};
  ($self:ident, $($arg:tt)*) => {Err($self.fmt_err(&format!($($arg)*), &$self.pos).into())};
}
#[macro_export]
#[doc(hidden)]
macro_rules! err {
  ($self:ident, $pos:expr, $($arg:tt)*) => {Err($self.parser.fmt_err(&format!($($arg)*), &$pos).into())};
}
#[macro_export]
#[doc(hidden)]
macro_rules! include_once {
  ($self:ident, $dest:expr, $name:literal) => {
    if $self.ctx.is_not_included($name) {
      $dest.push(include_str!(concat!("../asm/", $name, ".s")).into());
    }
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! write_once {
  ($self:ident, $name:literal) => {
    if $self.ctx.is_not_included($name) {
      $self.data.write_all(include_bytes!(concat!("../asm/", $name, ".s")))?;
    }
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! mn {
  ($mne:expr) => {format!("\t{}\n", $mne)};
  ($mne:expr, $($arg:expr),+ $(,)?) => {format!("\t{}\t{}\n", $mne, vec![$(format!("{}", $arg)),+].join(",\t"))};
}
#[macro_export]
#[doc(hidden)]
macro_rules! mn_write {
  ($dest:expr, $mne:expr) => {writeln!(&mut $dest, "\t{}", $mne)?};
  ($dest:expr, $mne:expr, $($arg:expr),+ $(,)?) => {
    writeln!(&mut $dest, "\t{}\t{}", $mne, vec![$(format!("{}", $arg)),+].join(",\t"))?
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! sub {
  ($op1: expr, $op2: expr) => {
    $op1.checked_sub($op2).ok_or("InternalError: Underflow occurred")
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! warn {
  ($self:ident, $pos:expr, $($arg:tt)*) => {println!("Warning: {}", $self.parser.fmt_err(&format!($($arg)*), &$pos))};
}
#[macro_export]
#[doc(hidden)]
macro_rules! validate_type {
  (
    $self:ident,
    $func:expr,
    $ord:expr,
    $val:expr,
    $pat:pat => $body:expr,
    $expected:literal
  ) => {
    if let $pat = $val.value {
      $body
    } else {
      return $self.parser.typ_err($ord, &$func.name, $expected, &$val);
    }
  };
}
