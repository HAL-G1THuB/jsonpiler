#[macro_export]
macro_rules! le_bytes {
    ($a:expr) => {
        $a.to_le_bytes()
    };
    ($a:expr, $($rest:expr),+ $(,)?) => {{
        const LHS: &[u8] = &$a.to_le_bytes();
        const RHS: &[u8] = &le_bytes!($($rest),+);
        let mut out = [0u8; LHS.len() + RHS.len()];
        let mut i = 0;
        while i < LHS.len() {
            out[i] = LHS[i];
            i += 1;
        }
        let mut j = 0;
        while j < RHS.len() {
            out[LHS.len() + j] = RHS[j];
            j += 1;
        }
        out
    }};
}
#[macro_export]
macro_rules! symbol {
  ($self:ident, $caller:ident, $name:ident) => {{
    if let Some(id) = $self.symbols.get($name).copied() {
      $self.use_func($caller, id);
      return Ok(id);
    }
    let id = $self.id();
    $self.use_func($caller, id);
    $self.symbols.insert($name, id);
    id
  }};
}
#[macro_export]
macro_rules! extend {
  ($vec:expr, $($data:expr),+ $(,)?) => { $($vec.extend_from_slice(&$data);)+ };
}
#[macro_export]
macro_rules! cat_vec {
  ($cap:expr, $($data:expr),+ $(,)?) => {{
    let mut vec = Vec::with_capacity($cap);
    $(vec.extend_from_slice(&$data);)+
    vec
  }};
}
#[macro_export]
macro_rules! err {
  ($pos:expr, $kind:expr) => {
    Err(compilation!($pos, $kind))
  };
}
#[macro_export]
macro_rules! compilation {
  ($pos:expr, $kind:expr) => {
    JsonpilerErr { kind: Compilation($kind), refs: vec![$pos] }
  };
}
#[macro_export]
macro_rules! warning {
  ($pos:expr, $kind:expr) => {
    JsonpilerErr { kind: Warning($kind), refs: vec![$pos] }
  };
}
#[macro_export]
macro_rules! array_arg {
  ($func:ident, $elem_type:expr, $len:expr, ($($kind:tt)+) => $body:ident) => {{
    let arg = $func.arg()?;
    if let Array(_, $($kind)+) = arg.val {
      arg.pos.with($body)
    } else {
      let $body = Default::default();
      let expected = Array(ArrayType::new($elem_type, $len), $($kind)+).as_type();
      return Err($func.args_err(vec![expected], &arg));
    }
  }};
}
#[macro_export]
macro_rules! arg { ($func:ident, ($($kind:tt)+) => $body:ident) => {{
    let arg = $func.arg()?;
    if let $($kind)+ = arg.val {
      arg.pos.with($body)
    } else {
      let $body = Default::default();
      let expected = $($kind)+.as_type();
      return Err($func.args_err(vec![expected], &arg));
    }
  }};
}
#[macro_export]
macro_rules! arg_custom {
  ($func:ident, $expected:expr, ($($kind:tt)+) => $body:ident) => {{
    let arg = $func.arg()?;
    if let $($kind)+ = arg.val {
      arg.pos.with($body)
    } else {
      return Err($func.args_err($expected, &arg));
    }
  }};
}
#[macro_export]
macro_rules! unwrap_arg {
  ($arg:expr, $name:expr, $expected:expr, ($($kind:tt)+) => $body:ident) => {{
    let arg = $arg;
    if let $($kind)+ = arg.val {
      arg.pos.with($body)
    } else {
      return Err(type_err($name.into(), $expected, arg.map_ref(Json::as_type)));
    }
  }};
}
#[macro_export]
macro_rules! built_in {
  (
    $self:ident, $func:ident, $scope:ident, $register_fn:ident;
    $( { $key:literal, $flags:tt, $arity:expr, $name:ident => $block:block $(, $name_a:ident => $block_a:block )? } ),+ $(,)?
  ) => {
    impl Jsonpiler {
      pub(crate) fn $register_fn(&mut self) {
        $(self.register_func($key, $flags, built_in_opt_a!($(Jsonpiler::$name_a)?), Jsonpiler::$name, $arity);)+
      }
    }
    #[allow(clippy::allow_attributes, clippy::unnecessary_wraps, clippy::too_many_lines, clippy::arbitrary_source_item_ordering)]
    impl Jsonpiler {
    $(
      fn $name(&mut $self, $func: &mut Pos<BuiltIn>, $scope: &mut Scope) -> ErrOR<Json> $block
      $( fn $name_a(&mut $self, $func: &mut Pos<BuiltIn>, $scope: &mut Scope) -> ErrOR<Json> $block_a )?
    )+
    }
  };
}
#[macro_export]
macro_rules! built_in_opt_a {
  ($x:expr) => {
    Some($x)
  };
  () => {
    None
  };
}
