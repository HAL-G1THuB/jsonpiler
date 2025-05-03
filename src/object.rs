//! Implementation for `JObject`.
use crate::{JObject, JsonWithPos};
impl JObject {
  /*
  pub fn clear(&mut self) {
    self.entries.clear();
  }
  /// Returns the first matching value by key (if any).
  pub fn get(&self, key: &str) -> Option<&JsonWithPos> {
    self.entries.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v)
  }
  /// Returns a mutable reference to the first matching key (searching from the end).
  pub fn get_mut(&mut self, key: &str) -> Option<&mut JsonWithPos> {
    self.entries.iter_mut().rev().find(|(k, _)| k == key).map(|(_, v)| v)
  }
  /// Returns all values associated with the given key.
  pub fn get_all(&self, key: &str) -> impl Iterator<Item = &JsonWithPos> {
    self.entries.iter().filter(move |(k, _)| k == key).map(|(_, v)| v)
  }
  */
  /// Inserts a key-value pair, allowing duplicates.
  pub fn insert(&mut self, key: String, value: JsonWithPos) {
    self.entries.push((key, value));
  }
  /*
  /// Is `JObject` empty.
  pub fn is_empty(&self) -> bool {
    self.entries.is_empty()
  }
  */
  /// Iterate.
  pub fn iter(&self) -> impl Iterator<Item = &(String, JsonWithPos)> {
    self.entries.iter()
  }
  /// Iterate as mutable.
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
  /// Removes the **first matching** key (in insertion order).
  pub fn remove(&mut self, key: &str) -> Option<JsonWithPos> {
    let idx = self.entries.iter().position(|(k, _)| k == key)?;
    Some(self.entries.remove(idx).1)
  }
  /// Removes **all entries** for the given key.
  pub fn remove_all(&mut self, key: &str) -> usize {
    let old_len = self.entries.len();
    self.entries.retain(|(k, _)| k != key);
    old_len.saturating_sub(self.entries.len())
  }
  */
}
