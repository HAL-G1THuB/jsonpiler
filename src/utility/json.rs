use crate::prelude::*;
pub(crate) type KeyVal = (Pos<String>, Pos<Json>);
#[derive(Debug, Clone)]
pub(crate) enum Json {
  Array(Bind<Vec<Pos<Json>>>),
  Bool(Bind<bool>),
  Float(Bind<f64>),
  Int(Bind<i64>),
  Null(Bind<()>),
  Object(Bind<Vec<KeyVal>>),
  Str(Bind<String>),
}
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum JsonType {
  ArrayT,
  BoolT,
  CustomT(String),
  FloatT,
  IntT,
  NullT,
  ObjectT,
  StrT,
}
impl Json {
  pub(crate) fn as_str(&self) -> Option<&str> {
    if let Str(Lit(string)) = self { Some(string) } else { None }
  }
  pub(crate) fn as_type(&self) -> JsonType {
    match self {
      Array(_) => ArrayT,
      Bool(_) => BoolT,
      Float(_) => FloatT,
      Int(_) => IntT,
      Null(_) => NullT,
      Object(_) => ObjectT,
      Str(_) => StrT,
    }
  }
  pub(crate) fn describe(&self) -> String {
    format!(
      "{}{}",
      self.as_type().name(),
      match self {
        Bool(bind) => format!("{bind}"),
        Null(_) => String::new(),
        Float(bind) => format!("{bind}"),
        Object(bind) => format!("{bind}"),
        Int(bind) => format!("{bind}"),
        Str(bind) => format!("{bind}"),
        Array(bind) => format!("{bind}"),
      }
    )
  }
  #[expect(clippy::print_stderr)]
  pub(crate) fn get(&self, key: &str) -> Option<&Json> {
    if let Object(Lit(obj)) = self {
      let opt = obj.iter().find(|(ke, _)| ke.val == key).map(|(_, va)| &va.val);
      if let Some(json) = opt {
        return Some(json);
      }
    }
    eprintln!("{key} failed");
    None
  }
  pub(crate) fn get_int(&self, key: &str) -> Option<i64> {
    if let Int(Lit(int)) = self.get(key)? { Some(*int) } else { None }
  }
  pub(crate) fn into_str(self) -> Option<String> {
    if let Str(Lit(string)) = self { Some(string) } else { None }
  }
  pub(crate) fn memory(&self) -> Option<Memory> {
    match self {
      Int(Var(memory)) | Float(Var(memory)) | Str(Var(memory)) | Bool(Var(memory))
      | Array(Var(memory)) | Null(Var(memory)) | Object(Var(memory)) => Some(*memory),
      Array(_) | Bool(_) | Float(_) | Int(_) | Null(_) | Object(_) | Str(_) => None,
    }
  }
  #[expect(clippy::print_stderr)]
  pub(crate) fn take(&mut self, key: &str) -> Option<Json> {
    if let Object(Lit(obj)) = self {
      let opt = obj.iter_mut().find(|(ke, _)| ke.val == key).map(|(_, va)| &mut va.val);
      if let Some(json) = opt {
        return Some(take(json));
      }
    }
    eprintln!("{key} failed");
    None
  }
}
impl JsonType {
  pub(crate) fn from_string(name: &str) -> Self {
    match name {
      "Str" => StrT,
      "Int" => IntT,
      "Float" => FloatT,
      "Null" => NullT,
      "Bool" => BoolT,
      "Object" => ObjectT,
      "Array" => ArrayT,
      unknown => CustomT(unknown.to_owned()),
    }
  }
  pub(crate) fn mem_type(&self, pos: Position) -> ErrOR<MemoryType> {
    match self {
      BoolT => Ok(MemoryType { heap: Value, size: Small(RB) }),
      FloatT | IntT | NullT => Ok(MemoryType { heap: Value, size: Small(RQ) }),
      StrT => Ok(MemoryType { heap: HeapPtr, size: Dynamic }),
      ArrayT | ObjectT => err!(pos, UnsupportedType(self.name())),
      CustomT(_) => err!(pos, UnknownType(self.name())),
    }
  }
  pub(crate) fn name(&self) -> String {
    match self {
      BoolT => "Bool",
      NullT => "Null",
      FloatT => "Float",
      ObjectT => "Object",
      IntT => "Int",
      StrT => "Str",
      ArrayT => "Array",
      CustomT(name) => name,
    }
    .into()
  }
  pub(crate) fn to_json(&self, pos: Position, addr: Address) -> ErrOR<Json> {
    let memory = Memory(addr, self.mem_type(pos)?);
    match self {
      StrT => Ok(Str(Var(memory))),
      IntT => Ok(Int(Var(memory))),
      FloatT => Ok(Float(Var(memory))),
      NullT => Ok(Null(Var(memory))),
      BoolT => Ok(Bool(Var(memory))),
      ArrayT | ObjectT => err!(pos, UnsupportedType(self.name())),
      CustomT(name) => err!(pos, UnknownType(name.clone())),
    }
  }
}
impl Pos<Json> {
  pub(crate) fn into_ident(mut self, name: &str) -> ErrOR<Pos<String>> {
    if let Object(Lit(obj)) = &self.val
      && obj.len() == 1
      && let Some(Str(Lit(string))) = self.val.take("$")
    {
      Ok(self.pos.with(string.clone()))
    } else {
      Err(type_err(name.into(), vec![CustomT("Ident".into())], self.map_ref(Json::as_type)))
    }
  }
}
impl fmt::Display for JsonType {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(&self.name())
  }
}
impl fmt::Display for Json {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Array(Lit(array)) => {
        let content = array.iter().map(|item| format!("{}", &item.val));
        write!(f, "[{}]", &content.collect::<Vec<_>>().join(","))
      }
      Bool(Lit(lit)) => lit.fmt(f),
      Int(Lit(lit)) => lit.fmt(f),
      Float(Lit(lit)) => lit.fmt(f),
      Str(Lit(lit)) => lit.fmt(f),
      Object(Lit(obj)) => {
        let content = obj.iter().map(|(key, val)| format!("{}:{}", &key.val, val.val));
        write!(f, "{{{}}}", &content.collect::<Vec<_>>().join(","))
      }
      Null(Lit(())) => write!(f, "null"),
      Null(Var(memory)) | Array(Var(memory)) | Bool(Var(memory)) | Int(Var(memory))
      | Float(Var(memory)) | Str(Var(memory)) | Object(Var(memory)) => memory.fmt(f),
    }
  }
}
impl<T> fmt::Display for Bind<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Var(memory) => write!(f, "{memory}"),
      Lit(_) => f.write_str(" (Literal)"),
    }
  }
}
impl fmt::Display for Memory {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self.0 {
      Local(Tmp, _) => Ok(()),
      Local(Long, _) => write!(f, " (Local variable)"),
      Global(_) => write!(f, " (Global variable)"),
    }
  }
}
impl Default for Json {
  fn default() -> Self {
    Null(Lit(()))
  }
}
