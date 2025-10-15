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
  ($self:ident, $ch:expr, $value:expr) => {
    if $self.peek() == $ch {
      $self.pos.offset += 1;
      return Ok($value);
    }
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! parse_err {
  ($self:ident, $pos:expr, $kind:expr) => {
    Err(CompilationError { kind: $kind, pos: $pos })
  };
  ($self:ident, $kind:expr) => {
    Err(CompilationError { kind: $kind, pos: $self.pos })
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! err {
  ($self:ident, $pos:expr, $kind:expr) => {
    Err(CompilationError { kind: $kind, pos: $pos })
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! warn {
  ($self:ident, $pos:expr, $($arg:tt)*) => {println!("Warning: {}", $self.parser[$pos.file].fmt_err(&format!($($arg)*), $pos))};
}
#[macro_export]
#[doc(hidden)]
macro_rules! get_target_mem {
  ($self:expr, $scope:expr, $is_global:expr, $size:expr, $ref_label:expr, $pattern:pat => $kind_expr:expr) => {
    if let Some(json) = &$ref_label {
      match json {
        $pattern => $kind_expr,
        _ => return Err(InternalError(MismatchReassignment)),
      }
    } else if $is_global {
      Global { id: $self.get_bss_id($size, $size), disp: 0i32 }
    } else {
      $scope.local($size, $size)?.mem
    }
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! unwrap_arg {
  (
    $self:ident,
    $arg:expr,
    $func:expr,
    $expected:literal,
    $pat:pat => $body:expr
  ) => {{
    if let $pat = $arg.value {
      $crate::WithPos { value: $body, pos: $arg.pos }
    } else {
      return Err(args_type_error($func.nth, &$func.name, $expected, &$arg));
    }
  }};
}
#[macro_export]
#[doc(hidden)]
macro_rules! take_arg {
  (
    $self:ident,
    $func:expr,
    $expected:literal,
    $pat:pat => $body:expr
  ) => {{
    let arg = $func.arg()?;
    if let $pat = arg.value {
      $crate::WithPos { value: $body, pos: arg.pos }
    } else {
      return Err($crate::utility::args_type_error($func.nth, &$func.name, $expected, &arg).into());
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
