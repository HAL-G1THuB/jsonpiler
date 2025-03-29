use std::collections::HashMap;
use std::error::Error;
mod impl_jparser;
mod impl_json;
mod impl_jvalue;
pub type JResult = Result<Json, Box<dyn Error>>;
pub type F<T> = fn(&mut T, &[Json], &mut String) -> JResult;
#[derive(Debug, Clone)]
pub struct Json {
  pub pos: usize,
  pub ln: usize,
  pub value: JValue,
}
#[derive(Debug, Clone)]
pub enum VKind<T> {
  Var(String),
  Lit(T),
}
#[derive(Debug, Clone)]
pub enum JValue {
  Null,
  Bool(VKind<bool>),
  Int(VKind<i64>),
  Float(VKind<f64>),
  String(VKind<String>),
  Array(VKind<Vec<Json>>),
  Object(VKind<HashMap<String, Json>>),
  Function(VKind<Vec<Json>>),
}
#[derive(Default)]
pub struct JParser<'a> {
  input_code: &'a str,
  pos: usize,
  seed: usize,
  ln: usize,
  data: String,
  bss: String,
  text: String,
  f_table: HashMap<String, F<Self>>,
  vars: HashMap<String, Json>,
}
