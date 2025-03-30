use super::utility::format_err;
use std::collections::HashMap;
use std::error::Error;
mod impl_compiler;
mod impl_json;
mod impl_jvalue;
mod impl_parser;
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
impl JParser<'_> {
  fn obj_err(&self, text: &str, obj: &Json) -> JResult {
    format_err(text, obj.pos, obj.ln, self.input_code)
  }
  fn obj_json(&self, val: JValue, obj: &Json) -> Json {
    Json {
      pos: obj.pos,
      ln: obj.ln,
      value: val,
    }
  }
  fn parse_err(&self, text: &str) -> JResult {
    format_err(text, self.pos, self.ln, self.input_code)
  }
}
