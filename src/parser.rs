pub(crate) mod error;
mod format_block;
mod formatter;
pub(crate) mod parse_json;
pub(crate) mod parse_jspl;
pub(crate) mod position;
mod utility;
use crate::prelude::*;
#[derive(Clone)]
pub(crate) struct Comment {
  leading: bool,
  text: String,
}
#[derive(Clone)]
pub(crate) struct Parser {
  comments: BTreeMap<u32, Comment>,
  pub dep: Dependency,
  pub exports: BTreeMap<String, Pos<UserDefinedInfo>>,
  pub file: String,
  pub text: String,
  pub warns: Vec<Pos<Warning>>,
}
impl Pos<Parser> {
  pub(crate) fn new(source: String, file_idx: FileIdx, file: String, id: LabelId) -> Self {
    Position::new(file_idx).with(Parser {
      text: source,
      file,
      comments: BTreeMap::new(),
      exports: BTreeMap::new(),
      warns: vec![],
      dep: Dependency::new(id),
    })
  }
}
impl Jsonpiler {
  pub(crate) fn first_parser(&self) -> ErrOR<&Pos<Parser>> {
    self.parsers.first().ok_or(Internal(MissingFirstParser))
  }
  pub(crate) fn first_parser_mut(&mut self) -> ErrOR<&mut Pos<Parser>> {
    self.parsers.first_mut().ok_or(Internal(MissingFirstParser))
  }
  pub(crate) fn push_parser(&mut self, source: String, file: String) -> ErrOR<&mut Pos<Parser>> {
    let file_idx = len_u32(&self.parsers)?;
    let parser = <Pos<Parser>>::new(source, file_idx, file, self.id());
    self.parsers.push(parser);
    Ok(&mut self.parsers[file_idx as usize])
  }
}
