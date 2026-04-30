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
#[derive(Debug, Clone, Default)]
pub(crate) enum JsonNoPos {
  ArrayN(Vec<JsonNoPos>),
  BoolN(bool),
  FloatN(f64),
  IntN(i64),
  #[default]
  NullN,
  ObjectN(Vec<(String, JsonNoPos)>),
  StrN(String),
}
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum JsonType {
  ArrayT,
  BoolT,
  CustomT(String),
  FloatT,
  FuncT(Vec<(String, JsonType)>, Box<JsonType>),
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
  pub(crate) fn delete_pos(self) -> JsonNoPos {
    match self {
      Array(Lit(array)) => {
        ArrayN(array.into_iter().map(|pos_json| pos_json.val.delete_pos()).collect())
      }
      Bool(Lit(bind)) => BoolN(bind),
      Float(Lit(float)) => FloatN(float),
      Int(Lit(int)) => IntN(int),
      Null(Lit(_)) => NullN,
      Object(Lit(obj)) => ObjectN(
        obj.into_iter().map(|(pos_key, pos_val)| (pos_key.val, pos_val.val.delete_pos())).collect(),
      ),
      Str(Lit(bind)) => StrN(bind),
      Array(Var(_)) | Bool(Var(_)) | Float(Var(_)) | Int(Var(_)) | Null(Var(_))
      | Object(Var(_)) | Str(Var(_)) => StrN(self.describe()),
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
  pub(crate) fn memory(&self) -> Option<Memory> {
    match self {
      Int(Var(memory)) | Float(Var(memory)) | Str(Var(memory)) | Bool(Var(memory))
      | Array(Var(memory)) | Null(Var(memory)) | Object(Var(memory)) => Some(*memory),
      Array(_) | Bool(_) | Float(_) | Int(_) | Null(_) | Object(_) | Str(_) => None,
    }
  }
}
impl JsonNoPos {
  pub(crate) fn as_str(&self) -> Option<&str> {
    if let StrN(string) = self { Some(string) } else { None }
  }
  pub(crate) fn get(&self, key: &str) -> Option<&JsonNoPos> {
    if let ObjectN(obj) = self {
      let opt = obj.iter().find(|(ke, _)| ke == key).map(|(_, va)| va);
      if let Some(json) = opt {
        return Some(json);
      }
    }
    None
  }
  pub(crate) fn get_bool(&self, key: &str) -> Option<bool> {
    if let BoolN(boolean) = self.get(key)? { Some(*boolean) } else { None }
  }
  pub(crate) fn get_int(&self, key: &str) -> Option<i64> {
    if let IntN(int) = self.get(key)? { Some(*int) } else { None }
  }
  pub(crate) fn into_str(self) -> Option<String> {
    if let StrN(string) = self { Some(string) } else { None }
  }
  pub(crate) fn take(&mut self, key: &str) -> Option<JsonNoPos> {
    if let ObjectN(obj) = self {
      let opt = obj.iter_mut().find(|(ke, _)| ke == key).map(|(_, va)| va);
      if let Some(json) = opt {
        return Some(take(json));
      }
    }
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
      FuncT(_, _) | ArrayT | ObjectT => err!(pos, UnsupportedType(self.name())),
      CustomT(_) => err!(pos, UnknownType(self.name())),
    }
  }
  pub(crate) fn name(&self) -> String {
    match self {
      BoolT => "Bool",
      NullT => "Null",
      FloatT => "Float",
      ObjectT => "Object",
      FuncT(_, _) => "Func",
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
      FuncT(_, _) | ArrayT | ObjectT => err!(pos, UnsupportedType(self.name())),
      CustomT(name) => err!(pos, UnknownType(name.clone())),
    }
  }
}
impl Pos<Json> {
  pub(crate) fn into_ident(mut self, name: &str) -> ErrOR<Pos<String>> {
    if let Object(Lit(obj)) = &mut self.val
      && obj.len() == 1
      && &obj[0].0.val == "$"
      && let Str(Lit(string)) = take(&mut obj[0].1.val)
    {
      Ok(self.pos.with(string))
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
impl fmt::Display for JsonNoPos {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    self.fmt_with_indent(f, 0)
  }
}
impl fmt::Display for Json {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    self.clone().delete_pos().fmt_with_indent(f, 0)
  }
}
impl JsonNoPos {
  fn fmt_with_indent(&self, fmter: &mut fmt::Formatter, indentation: usize) -> fmt::Result {
    match self {
      ArrayN(array) => {
        write!(fmter, "[")?;
        format_items(array, indentation, fmter, |(_f, item)| Ok(item))?;
        write!(fmter, "]")
      }
      ObjectN(obj) => {
        write!(fmter, "{{")?;
        format_items(obj, indentation, fmter, |(fmter2, (key, val))| {
          write!(fmter2, "{}: ", StrN(key.clone()))?;
          Ok(val)
        })?;
        write!(fmter, "}}")
      }
      BoolN(lit) => write!(fmter, "{lit}"),
      IntN(lit) => write!(fmter, "{lit}"),
      FloatN(lit) => write!(fmter, "{lit}"),
      StrN(lit) => write!(fmter, "\"{}\"", json_escape(lit)),
      Self::NullN => write!(fmter, "null"),
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
fn format_items<'a, T, F: Fn((&mut fmt::Formatter, &'a T)) -> Result<&'a JsonNoPos, fmt::Error>>(
  items: &'a [T],
  indentation: usize,
  fmter: &mut fmt::Formatter,
  task: F,
) -> fmt::Result {
  for (i, item) in items.iter().enumerate() {
    if i != 0 {
      write!(fmter, ",")?;
    }
    writeln!(fmter)?;
    indent(fmter, indentation + 1)?;
    let json = task((fmter, item))?;
    json.fmt_with_indent(fmter, indentation + 1)?;
  }
  if !items.is_empty() {
    writeln!(fmter)?;
    indent(fmter, indentation)
  } else {
    Ok(())
  }
}
fn indent(fmter: &mut fmt::Formatter, n: usize) -> fmt::Result {
  for _ in 0..n {
    write!(fmter, "  ")?;
  }
  Ok(())
}
pub(crate) fn json_escape(input: &str) -> String {
  let mut out = String::with_capacity(input.len());
  for char in input.chars() {
    match char {
      '"' => out.push_str("\\\""),
      '\\' => out.push_str("\\\\"),
      '\n' => out.push_str("\\n"),
      '\r' => out.push_str("\\r"),
      '\t' => out.push_str("\\t"),
      '\u{08}' => out.push_str("\\b"),
      '\u{0C}' => out.push_str("\\f"),
      ctrl if ctrl.is_control() => {
        out.push_str(&format!("\\u{:04x}", ctrl as u32));
      }
      _ => out.push(char),
    }
  }
  out
}
