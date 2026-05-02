use crate::prelude::*;
#[derive(Debug, Clone, Copy, Default, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct Memory(pub Address, pub MemoryType);
impl fmt::Display for Memory {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self.0 {
      Local(Tmp, _) => Ok(()),
      Local(Long, _) => write!(f, " (Local variable)"),
      Global(_) => write!(f, " (Global variable)"),
    }
  }
}
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct MemoryType {
  pub heap: Storage,
  pub size: MemorySize,
}
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Storage {
  HeapPtr,
  #[default]
  Value,
}
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) enum MemorySize {
  Dynamic,
  Known(i32),
  Small(RegSize),
}
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) enum RegSize {
  RB = 1,
  RD = 4,
  RQ = 8,
}
#[derive(Debug, Clone, Copy, Ord, PartialOrd, PartialEq, Eq)]
pub(crate) enum Address {
  Global(LabelId),
  Local(Lifetime, i32),
}
#[derive(Debug, Clone, Copy, Ord, PartialOrd, PartialEq, Eq)]
pub(crate) enum Lifetime {
  Long,
  Tmp,
}
impl Default for Address {
  fn default() -> Self {
    Local(Tmp, 0)
  }
}
impl Default for MemorySize {
  fn default() -> Self {
    Small(RQ)
  }
}
impl<T> Default for Bind<T> {
  fn default() -> Self {
    Var(Memory::default())
  }
}
impl MemoryType {
  pub(crate) fn size(self) -> i32 {
    if self.heap == HeapPtr {
      8
    } else {
      match self.size {
        Known(size) => size,
        Small(size) => size as i32,
        Dynamic => 8,
      }
    }
  }
}
