//! Implementation of the `JObject`.
use {
  crate::{JObject, Json},
  core::mem,
};
#[expect(dead_code, reason = "todo")]
impl JObject {
  /// Clears all entries and index mappings from the object.
  #[inline]
  pub fn clear(&mut self) {
    self.entries.clear();
    self.idx.clear();
  }
  /// Returns a reference to the value associated with the given key.
  /// Returns `None` if the key is not found.
  #[inline]
  #[must_use]
  pub fn get(&self, key: &str) -> Option<&Json> {
    Some(&self.entries.get(*self.idx.get(key)?)?.1)
  }
  /// Returns a mutable reference to the value associated with the given key.
  /// Returns `None` if the key is not found.
  #[inline]
  pub fn get_mut(&mut self, key: &str) -> Option<&mut Json> {
    Some(&mut self.entries.get_mut(*self.idx.get(key)?)?.1)
  }
  /// Inserts a key-value pair into the object.
  /// If the key already exists, replaces the value and returns the old one.
  /// Otherwise, inserts a new entry and returns `None`.
  #[inline]
  pub fn insert(&mut self, key: String, value: Json) -> Option<Json> {
    if let Some(&idx) = self.idx.get(&key) {
      let entry = self.entries.get_mut(idx)?;
      let old_value = mem::replace(&mut entry.1, value);
      Some(old_value)
    } else {
      let index = self.entries.len();
      self.entries.push((key.clone(), value));
      self.idx.insert(key, index);
      None
    }
  }
  /// Returns `true` if the object contains no entries.
  #[inline]
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.entries.is_empty()
  }
  /// Returns an iterator over all key-value pairs in insertion order.
  #[inline]
  pub fn iter(&self) -> impl Iterator<Item = &(String, Json)> {
    self.entries.iter()
  }
  /// Returns a mutable iterator over all key-value pairs in insertion order.
  #[inline]
  pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut (String, Json)> {
    self.entries.iter_mut()
  }
  /// Returns the number of entries in the object.
  #[inline]
  #[must_use]
  pub fn len(&self) -> usize {
    self.entries.len()
  }
  /// Returns a reference to the key-value pair at the specified index.
  /// Returns `None` if the index is out of bounds.
  #[inline]
  #[must_use]
  pub fn nth(&self, index: usize) -> Option<&(String, Json)> {
    self.entries.get(index)
  }
  /// Returns a mutable reference to the key-value pair at the specified index.
  /// Returns `None` if the index is out of bounds.
  #[inline]
  pub fn nth_mut(&mut self, index: usize) -> Option<&mut (String, Json)> {
    self.entries.get_mut(index)
  }
  /// Removes the entry with the given key and returns its value, if it exists.
  #[inline]
  pub fn remove(&mut self, key: &str) -> Option<Json> {
    let remove_idx = *self.idx.get(key)?;
    let removed_value = self.entries.remove(remove_idx).1;
    self.idx.remove(key);
    for (i, j) in self.entries.iter().enumerate().skip(remove_idx) {
      self.idx.insert(j.0.clone(), i);
    }
    Some(removed_value)
  }
}
