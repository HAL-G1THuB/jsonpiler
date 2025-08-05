#[macro_export]
#[doc(hidden)]
macro_rules! add {
  ($op1: expr, $op2: expr) => {
    $op1.checked_add($op2).ok_or("InternalError: Overflow occurred")
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! err {
  ($self:ident, $pos:expr, $($arg:tt)*) => {Err($self.fmt_err(&format!($($arg)*), &$pos).into())};
  ($self:ident, $($arg:tt)*) => {Err($self.fmt_err(&format!($($arg)*), &$self.pos).into())};
}
#[macro_export]
#[doc(hidden)]
macro_rules! include_once {
  ($self:ident, $dest:expr, $name:literal) => {
    if !$self.include_flag.contains($name) {
      $self.include_flag.insert($name.into());
      $dest.push(include_str!(concat!("../asm/", $name, ".s")).into());
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
  ($dest:expr, $mne:expr) => {writeln!($dest, "\t{}", $mne)};
  ($dest:expr, $mne:expr, $($arg:expr),+ $(,)?) => {
    writeln!($dest, "\t{}\t{}", $mne, vec![$(format!("{}", $arg)),+].join(",\t"))
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
  ($self:ident, $pos:expr, $($arg:tt)*) => {println!("Warning: {}", $self.fmt_err(&format!($($arg)*), &$pos))};
  ($self:ident, $($arg:tt)*) => {println!("Warning: {}", $self.fmt_err(&format!($($arg)*), &$self.pos))};
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
      return $self.typ_err($ord, &$func.name, $expected, &$val);
    }
  };
}
