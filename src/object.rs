//! Implementation of the `JObject`.
use {
  crate::{JObject, JsonWithPos},
  core::mem,
};
impl JObject {
  /// Clears all entries and index mappings from the object.
  #[expect(dead_code, reason = "todo")]
  pub fn clear(&mut self) {
    self.entries.clear();
    self.index.clear();
  }
  /// Returns a reference to the value associated with the given key.
  /// Returns `None` if the key is not found.
  #[must_use]
  #[expect(dead_code, reason = "todo")]
  pub fn get(&self, key: &str) -> Option<&JsonWithPos> {
    Some(&self.entries.get(*self.index.get(key)?)?.1)
  }
  /// Returns a mutable reference to the value associated with the given key.
  /// Returns `None` if the key is not found.
  #[expect(dead_code, reason = "todo")]
  pub fn get_mut(&mut self, key: &str) -> Option<&mut JsonWithPos> {
    Some(&mut self.entries.get_mut(*self.index.get(key)?)?.1)
  }
  /// Inserts a key-value pair into the object.
  /// If the key already exists, replaces the value and returns the old one.
  /// Otherwise, inserts a new entry and returns `None`.
  pub fn insert(&mut self, key: String, value: JsonWithPos) -> Option<JsonWithPos> {
    if let Some(&idx) = self.index.get(&key) {
      Some(mem::replace(&mut self.entries.get_mut(idx)?.1, value))
    } else {
      self.index.insert(key.clone(), self.entries.len());
      self.entries.push((key, value));
      None
    }
  }
  /// Returns `true` if the object contains no entries.
  #[must_use]
  #[expect(dead_code, reason = "todo")]
  pub fn is_empty(&self) -> bool {
    self.entries.is_empty()
  }
  /// Returns an iterator over all key-value pairs in insertion order.
  pub fn iter(&self) -> impl Iterator<Item = &(String, JsonWithPos)> {
    self.entries.iter()
  }
  /// Returns a mutable iterator over all key-value pairs in insertion order.
  #[expect(dead_code, reason = "todo")]
  pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut (String, JsonWithPos)> {
    self.entries.iter_mut()
  }
  /// Returns the number of entries in the object.
  #[must_use]
  #[expect(dead_code, reason = "todo")]
  pub fn len(&self) -> usize {
    self.entries.len()
  }
  /// Returns a reference to the key-value pair at the specified index.
  /// Returns `None` if the index is out of bounds.
  #[must_use]
  #[expect(dead_code, reason = "todo")]
  pub fn nth(&self, index: usize) -> Option<&(String, JsonWithPos)> {
    self.entries.get(index)
  }
  /// Returns a mutable reference to the key-value pair at the specified index.
  /// Returns `None` if the index is out of bounds.
  #[expect(dead_code, reason = "todo")]
  pub fn nth_mut(&mut self, index: usize) -> Option<&mut (String, JsonWithPos)> {
    self.entries.get_mut(index)
  }
  /// Removes the entry with the given key and returns its value, if it exists.
  #[expect(dead_code, reason = "todo")]
  pub fn remove(&mut self, key: &str) -> Option<JsonWithPos> {
    let remove_index = *self.index.get(key)?;
    let removed_value = self.entries.remove(remove_index).1;
    self.index.remove(key);
    for (i, j) in self.entries.iter().enumerate().skip(remove_index) {
      self.index.insert(j.0.clone(), i);
    }
    Some(removed_value)
  }
}
