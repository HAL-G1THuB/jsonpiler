use std::collections::HashMap;
use std::error::Error;
mod impl_compiler;
mod impl_json;
mod impl_parser;
pub mod utility;
use utility::format_err;
pub type JResult = Result<Json, Box<dyn Error>>;
pub type JFunc<T> = fn(&mut T, &[Json], &mut String) -> JResult;
#[derive(Debug, Clone)]
pub struct Json {
  pub pos: usize,
  pub ln: usize,
  pub value: JValue,
}
#[derive(Debug, Clone)]
pub enum JValue {
  Null,
  Bool(bool),
  Int(i64),
  Float(f64),
  String(String),
  Array(Vec<Json>),
  Object(HashMap<String, Json>),
  FuncVar(String, Vec<Json>),
  BoolVar(String),
  IntVar(String),
  FloatVar(String),
  StringVar(String),
  ArrayVar(String),
  ObjectVar(String),
}
#[derive(Debug, Clone, Default)]
pub struct Jsompiler<'a> {
  input_code: &'a str,
  pos: usize,
  seed: usize,
  ln: usize,
  data: String,
  bss: String,
  text: String,
  f_table: HashMap<String, JFunc<Self>>,
  globals: HashMap<String, JValue>,
  vars: HashMap<String, JValue>,
}
impl Jsompiler<'_> {
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
