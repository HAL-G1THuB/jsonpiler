pub(crate) mod error;
mod format_block;
pub(crate) mod formatter;
pub(crate) mod parse_json;
pub(crate) mod parse_jspl;
pub(crate) mod position;
mod utility;
use crate::prelude::*;
#[derive(Debug, Clone)]
pub(crate) struct Comment {
  leading: bool,
  text: String,
}
pub(crate) struct File {
  pub dep: Dependency,
  pub exports: BTreeMap<String, Pos<UserDefinedInfo>>,
  pub parser: Pos<Parser>,
  pub path: String,
  pub rel_path: String,
}
#[derive(Debug, Clone)]
pub(crate) struct Parser {
  comments: BTreeMap<u32, Comment>,
  pub text: String,
  pub warns: Vec<Pos<Warning>>,
}
impl File {
  pub(crate) fn new(
    source: String,
    file_idx: FileIdx,
    path: String,
    root_path: &str,
    id: LabelId,
  ) -> Self {
    let root_parent = Path::new(root_path).parent().unwrap_or(if cfg!(target_os = "windows") {
      Path::new("C:")
    } else {
      Path::new("")
    });
    let rel_path = Path::new(&path)
      .strip_prefix(root_parent)
      .unwrap_or(Path::new(&path))
      .to_string_lossy()
      .to_string();
    File {
      dep: Dependency::new(id),
      exports: BTreeMap::new(),
      rel_path,
      parser: <Pos<Parser>>::new(source, file_idx),
      path,
    }
  }
}
impl Pos<Parser> {
  pub(crate) fn new(text: String, file_idx: FileIdx) -> Self {
    Position::new(file_idx).with(Parser { text, comments: BTreeMap::new(), warns: vec![] })
  }
}
impl Jsonpiler {
  pub(crate) fn first_file(&self) -> ErrOR<&File> {
    self.files.first().ok_or(MissingFirstParser.into())
  }
  pub(crate) fn first_file_mut(&mut self) -> ErrOR<&mut File> {
    self.files.first_mut().ok_or(MissingFirstParser.into())
  }
  pub(crate) fn push_file(&mut self, source: String, path: String) -> ErrOR<&mut File> {
    let file_idx = self.files.len();
    let id = self.id();
    let root_path = self.first_file().map(|file| &file.path).unwrap_or(&path);
    let file = File::new(source, file_idx, path.clone(), root_path, id);
    self.files.push(file);
    Ok(&mut self.files[file_idx])
  }
}
