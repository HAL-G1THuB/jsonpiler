use crate::prelude::*;
#[derive(Debug, Clone, Copy, Default, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct Memory(pub Address, pub MemType);
impl fmt::Display for Memory {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self.0 {
      Local(Tmp, _) => Ok(()),
      Local(Long, _) => write!(f, " (Local variable)"),
      Global(_) => write!(f, " (Global variable)"),
    }
  }
}
// Value && Known -> ptr
// Value && Dynamic -> cannot get size
// Value && Small -> value
// (Heap)Ptr && Small -> deref value
// (Heap)Ptr && Known || Dynamic -> ref
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct MemType {
  pub pass_by: PassBy,
  pub size: MemorySize,
}
impl MemType {
  pub(crate) fn h_ptr() -> Self {
    MemType { pass_by: HeapPtr, size: Dynamic }
  }
  pub(crate) fn ptr() -> Self {
    MemType { pass_by: Value, size: Dynamic }
  }
  pub(crate) fn v_rb() -> Self {
    MemType { pass_by: Value, size: Small(S1) }
  }
  pub(crate) fn v_rd() -> Self {
    MemType { pass_by: Value, size: Small(S4) }
  }
  pub(crate) fn v_rq() -> Self {
    MemType { pass_by: Value, size: Small(S8) }
  }
}
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum PassBy {
  HeapPtr,
  Ptr,
  #[default]
  Value,
}
impl PassBy {
  pub(crate) fn is_ptr(self) -> bool {
    self == HeapPtr || self == Ptr
  }
}
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) enum MemorySize {
  Dynamic,
  Known(i32),
  Small(RegSize),
}
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum RegSize {
  S1 = 1,
  S4 = 4,
  S8 = 8,
}
impl RegSize {
  pub(crate) fn rex_w(self) -> u8 {
    u8::from(self == S8)
  }
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
    Small(S8)
  }
}
impl<T> Default for Bind<T> {
  fn default() -> Self {
    Var(Memory::default())
  }
}
impl Address {
  pub(crate) fn h_ptr(self) -> Memory {
    Memory(self, MemType::h_ptr())
  }
  pub(crate) fn ptr(self) -> Memory {
    Memory(self, MemType::ptr())
  }
  pub(crate) fn v_rb(self) -> Memory {
    Memory(self, MemType::v_rb())
  }
  pub(crate) fn v_rd(self) -> Memory {
    Memory(self, MemType::v_rd())
  }
  pub(crate) fn v_rq(self) -> Memory {
    Memory(self, MemType::v_rq())
  }
}
impl MemType {
  pub(crate) fn reg_size(self) -> RegSize {
    if self.pass_by.is_ptr() {
      S8
    } else {
      match self.size {
        Small(size) => size,
        Known(_) => S8,
        Dynamic => S8,
      }
    }
  }
  pub(crate) fn size(self) -> ErrOR<i32> {
    if self.pass_by.is_ptr() {
      Ok(8)
    } else {
      match self.size {
        Small(size) => Ok(size as i32),
        Known(size) => Ok(size),
        Dynamic => Err(InvalidInst("Dynamic pass_by: Value".into()).into()),
      }
    }
  }
}
