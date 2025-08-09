#[macro_export]
#[doc(hidden)]
macro_rules! return_if {
  ($self:ident, $ch:expr, $pos:ident, $value:expr) => {
    if $self.peek() == $ch {
      $self.pos.offset += 1;
      $pos.extend_to($self.pos.offset);
      return Ok(WithPos { $pos, value: $value });
    }
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! parse_err {
  ($self:ident, $pos:expr, $($arg:tt)*) => {Err($self.fmt_err(&format!($($arg)*), $pos).into())};
  ($self:ident, $($arg:tt)*) => {Err($self.fmt_err(&format!($($arg)*), $self.pos).into())};
}
#[macro_export]
#[doc(hidden)]
macro_rules! err {
  ($self:ident, $pos:expr, $($arg:tt)*) => {Err($self.parser.fmt_err(&format!($($arg)*), $pos).into())};
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
macro_rules! add {
  ($op1:expr, $op2:expr) => {
    $op1.checked_add($op2).ok_or("InternalError: Overflow occurred")
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! sub {
  ($op1:expr, $op2:expr) => {
    $op1.checked_sub($op2).ok_or("InternalError: Underflow occurred")
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! warn {
  ($self:ident, $pos:expr, $($arg:tt)*) => {println!("Warning: {}", $self.parser.fmt_err(&format!($($arg)*), $pos))};
}
#[macro_export]
#[doc(hidden)]
macro_rules! take_arg {
  (
    $self:ident,
    $func:expr,
    $nth:expr,
    $expected:literal,
    $pat:pat => $body:expr
  ) => {{
    let arg = $func.arg()?;
    if let $pat = arg.value {
      ($body, arg.pos)
    } else {
      return Err($self.parser.type_err($nth, &$func.name, $expected, &arg).into());
    }
  }};
}
#[macro_export]
#[doc(hidden)]
macro_rules! built_in {
  (
    $self:ident, $func:ident, $scope:ident, $register_fn:ident;
    $( $name:ident => { $key:literal, $attrs:tt, $arity:expr, $block:block } ),+ $(,)?
  ) => {
    impl Jsonpiler {
      pub(crate) fn $register_fn(&mut self) {
      $(
        self.register($key, Jsonpiler::$attrs, Jsonpiler::$name, $arity);
      )+
      }
    }
    #[allow(clippy::allow_attributes, clippy::single_call_fn, clippy::unnecessary_wraps)]
    impl Jsonpiler {
    $(
      fn $name(
        &mut $self,
        $func: &mut FuncInfo,
        $scope: &mut ScopeInfo
      ) -> ErrOR<Json> {
        $block
      }
    )+
  }};
}
