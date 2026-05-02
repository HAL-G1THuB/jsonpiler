use crate::prelude::*;
#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, Eq)]
pub(crate) struct Position {
  pub file: FileIdx,
  pub info: (bool, bool),
  pub line: u32,
  pub offset: u32,
  pub size: u32,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct Pos<T> {
  pub pos: Position,
  pub val: T,
}
impl<T: Copy> Copy for Pos<T> {}
impl<T> Pos<T> {
  pub(crate) fn map<F: Fn(T) -> V, V>(self, map_f: F) -> Pos<V> {
    self.pos.with(map_f(self.val))
  }
  pub(crate) fn map_ref<F: Fn(&T) -> V, V>(&self, map_f: F) -> Pos<V> {
    self.pos.with(map_f(&self.val))
  }
}
impl Position {
  pub(crate) fn contains_inclusive(&self, file: FileIdx, offset: u32) -> bool {
    self.file == file && self.offset <= offset && offset <= self.end()
  }
  pub(crate) fn end(self) -> u32 {
    self.offset + self.size
  }
  #[expect(dead_code)]
  pub(crate) fn in_range(self, offset: u32) -> bool {
    self.offset <= offset && offset < self.end()
  }
  pub(crate) fn new(file: FileIdx) -> Self {
    Self { file, info: INFO_NONE, line: 0, offset: 0, size: 0 }
  }
  pub(crate) fn with<V>(self, val: V) -> Pos<V> {
    Pos { val, pos: self }
  }
}
