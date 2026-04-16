#[macro_export]
macro_rules! symbol {
  ($self:ident, $caller:ident, $name:ident) => {{
    if let Some(id) = $self.symbols.get($name).copied() {
      $self.use_function($caller, id);
      return Ok(id);
    }
    let id = $self.id();
    $self.use_function($caller, id);
    $self.symbols.insert($name, id);
    id
  }};
}
#[macro_export]
macro_rules! write_all {
  ($writer:expr, $($data:expr),+ $(,)?) => { $($writer.write_all(&$data)?;)+ };
}
#[macro_export]
macro_rules! extend {
  ($vec:expr, $($data:expr),+ $(,)?) => { $($vec.extend_from_slice(&$data);)+ };
}
#[macro_export]
macro_rules! err {
  ($pos:expr, $kind:expr) => {
    Err(Compilation($kind, vec![$pos]))
  };
}
#[macro_export]
macro_rules! parse_err {
  ($pos:expr, $kind:expr) => {
    Err($pos.with($kind))
  };
}
#[macro_export]
macro_rules! arg { ($self:ident, $func:expr, ($($kind:tt)+) => $body:ident) => {{
    let arg = $func.arg()?;
    if let $($kind)+ = arg.val {
      arg.pos.with($body)
    } else {
      let $body = Default::default();
      let expected = $($kind)+.as_type();
      return Err($func.args_err(vec![expected], arg.map_ref(Json::as_type)));
    }
  }};
}
#[macro_export]
macro_rules! arg_custom {
  ($self:ident, $func:expr, $expected:expr, ($($kind:tt)+) => $body:ident) => {{
    let arg = $func.arg()?;
    if let $($kind)+ = arg.val {
      arg.pos.with($body)
    } else {
      return Err($func.args_err($expected, arg.map_ref(Json::as_type)));
    }
  }};
}
#[macro_export]
macro_rules! unwrap_arg {
  ($self:ident, $arg:expr, $name:expr, $expected:expr, ($($kind:tt)+) => $body:ident) => {{
    let arg = $arg;
    if let $($kind)+ = arg.val {
      arg.pos.with($body)
    } else {
      return Err(type_err($name.into(), $expected,arg.map_ref(Json::as_type)));
    }
  }};
}
#[macro_export]
macro_rules! built_in {
  (
    $self:ident, $func:ident, $scope:ident, $register_fn:ident;
    $( $name:ident => { $key:literal, $flags:tt, $arity:expr, $block:block } ),+ $(,)?
  ) => {
    impl Jsonpiler {
      pub(crate)  fn $register_fn(&mut self) {
        $(self.register_func($key, $flags, Jsonpiler::$name, $arity);)+
      }
    }
    #[allow(clippy::allow_attributes, clippy::unnecessary_wraps, clippy::too_many_lines)]
    impl Jsonpiler {
    $( fn $name(&mut $self, $func: &mut Pos<BuiltIn>, $scope: &mut Scope) -> ErrOR<Json> $block )+
    }
  };
}
