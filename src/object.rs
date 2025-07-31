//! Implementation for `JObject`.
use crate::{JObject, JsonWithPos};
impl JObject {
  /*
  pub fn clear(&mut self) {
    self.entries.clear();
  }
    pub fn get(&self, key: &str) -> Option<&JsonWithPos> {
    self.entries.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v)
  }
    pub fn get_mut(&mut self, key: &str) -> Option<&mut JsonWithPos> {
    self.entries.iter_mut().rev().find(|(k, _)| k == key).map(|(_, v)| v)
  }
    pub fn get_all(&self, key: &str) -> impl Iterator<Item = &JsonWithPos> {
    self.entries.iter().filter(move |(k, _)| k == key).map(|(_, v)| v)
  }
  */
  pub fn insert(&mut self, key: String, value: JsonWithPos) {
    self.entries.push((key, value));
  }
  /*
    pub fn is_empty(&self) -> bool {
    self.entries.is_empty()
  }
  */
  pub fn iter(&self) -> impl Iterator<Item = &(String, JsonWithPos)> {
    self.entries.iter()
  }
  pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut (String, JsonWithPos)> {
    self.entries.iter_mut()
  }
  /*
  pub fn len(&self) -> usize {
    self.entries.len()
  }
  pub fn nth(&self, index: usize) -> Option<&(String, JsonWithPos)> {
    self.entries.get(index)
  }
  pub fn nth_mut(&mut self, index: usize) -> Option<&mut (String, JsonWithPos)> {
    self.entries.get_mut(index)
  }
    pub fn remove(&mut self, key: &str) -> Option<JsonWithPos> {
    let idx = self.entries.iter().position(|(k, _)| k == key)?;
    Some(self.entries.remove(idx).1)
  }
    pub fn remove_all(&mut self, key: &str) -> usize {
    let old_len = self.entries.len();
    self.entries.retain(|(k, _)| k != key);
    old_len.saturating_sub(self.entries.len())
  }
  */
}
