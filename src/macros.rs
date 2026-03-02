#[macro_export]
#[doc(hidden)]
macro_rules! symbol {
  ($self:ident, $name:ident) => {{
    if let Some(id) = $self.symbols.get($name) {
      return Ok(*id);
    }
    let id = $self.id();
    $self.symbols.insert($name, id);
    id
  }};
}
#[macro_export]
#[doc(hidden)]
macro_rules! write_all {
  ($writer:expr, $($data:expr),+ $(,)?) => { $($writer.write_all(&$data)?;)+ };
}
#[macro_export]
#[doc(hidden)]
macro_rules! extend {
  ($vec:expr, $($data:expr),+ $(,)?) => { $($vec.extend_from_slice(&$data);)+ };
}
#[macro_export]
#[doc(hidden)]
macro_rules! err {
  ($pos:expr, $kind:expr) => {
    Err(Compilation($kind, $pos))
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! or_err {
  (($($arg:tt)*), $pos:expr, $kind:expr) => {
    $($arg)*.ok_or(Compilation($kind, $pos))
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! warn {
  ($self:ident, $pos:expr, $($arg:tt)*) => {
    let (file, l_c, code, carets) = $self.err_info($pos);
    println!("{WARNING}{}{ERR_SEPARATE}{file}{l_c}{ERR_SEPARATE}{code}| {carets}{ERR_END}", $($arg)*,);
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! arg { ($self:ident, $func:expr, ($($kind:tt)+) => $body:ident) => {{
    let arg = $func.arg()?;
    if let $($kind)+ = arg.val {
      arg.pos.with($body)
    } else {
      let $body = Default::default();
      let expected = $($kind)+.describe();
      return Err(args_type_err($func.nth, &$func.name, expected, &arg));
    }
  }};
}
#[macro_export]
#[doc(hidden)]
macro_rules! arg_custom {
  ($self:ident, $func:expr, $expected:expr, ($($kind:tt)+) => $body:ident) => {{
    let arg = $func.arg()?;
    if let $($kind)+ = arg.val {
      arg.pos.with($body)
    } else {
      return Err(args_type_err($func.nth, &$func.name, $expected.into(), &arg));
    }
  }};
}
#[macro_export]
#[doc(hidden)]
macro_rules! unwrap_arg {
  ($self:ident, $arg:expr, $func:expr, $expected:expr, ($($kind:tt)+) => $body:ident) => {
    if let $($kind)+ = $arg.val {
      $arg.pos.with($body)
    } else {
      return Err(args_type_err($func.nth, &$func.name, $expected.into(), &$arg));
    }
  };
}
#[macro_export]
#[doc(hidden)]
macro_rules! built_in {
  (
    $self:ident, $func:ident, $scope:ident, $register_fn:ident;
    $( $name:ident => { $key:literal, $flags:tt, $arity:expr, $block:block } ),+ $(,)?
  ) => {
    impl Jsonpiler {
      pub(crate) fn $register_fn(&mut self) {
        $(self.register($key, $flags, Jsonpiler::$name, $arity);)+
      }
    }
    #[allow(clippy::allow_attributes, clippy::unnecessary_wraps, clippy::too_many_lines)]
    impl Jsonpiler {
    $( fn $name(&mut $self, $func: &mut Function, $scope: &mut Scope) -> ErrOR<Json> $block )+
    }
  };
}
