#[macro_export]
#[doc(hidden)]
macro_rules! extend_bytes {
  ($vec:expr, $($data:expr),+ $(,)?) => {
    $($vec.extend_from_slice($data);)+
  };
}
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
  ($self:ident, $pos:expr, $($arg:tt)*) => {Err($self.fmt_err(&format!("ParseError: {}", &format!($($arg)*)), $pos).into())};
  ($self:ident, $($arg:tt)*) => {Err($self.fmt_err(&format!("ParseError: {}", format!($($arg)*)), $self.pos).into())};
}
#[macro_export]
#[doc(hidden)]
macro_rules! err {
  ($self:ident, $pos:expr, $($arg:tt)*) => {Err($self.parser.fmt_err(&format!($($arg)*), $pos).into())};
}
#[macro_export]
#[doc(hidden)]
macro_rules! warn {
  ($self:ident, $pos:expr, $($arg:tt)*) => {println!("Warning: {}", $self.parser.fmt_err(&format!($($arg)*), $pos))};
}
#[macro_export]
#[doc(hidden)]
macro_rules! get_target_kind {
  ($self:expr, $scope:expr, $is_global:expr, $size:expr, $local_label:expr, $pattern:pat => $kind_expr:expr) => {
    if $is_global {
      Global { id: $self.get_bss_id($size) }
    } else if let Some(json) = &$local_label {
      match json {
        $pattern => $kind_expr,
        _ => return Err("InternalError: Unexpected Json variant during reassignment".into()),
      }
    } else {
      $scope.local($size)?.kind
    }
  };
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
      $(self.register($key, Jsonpiler::$attrs, Jsonpiler::$name, $arity);)+
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
