use crate::prelude::*;
use core::ops::Add;
impl<T: Copy> Copy for Bind<T> {}
impl Default for Address {
  fn default() -> Self {
    Local(Tmp, 0)
  }
}
impl<T> Default for Bind<T> {
  fn default() -> Self {
    Var(Label::default())
  }
}
impl Function {
  pub(crate) fn arg(&mut self) -> ErrOR<WithPos<Json>> {
    self.nth += 1;
    self.args.next().ok_or_else(|| Internal(NonExistentArg(self.name.clone(), self.nth)))
  }
  pub(crate) fn push_free_tmp(&mut self, label: Label) {
    if let Label(Local(Tmp, offset), size) = label {
      self.free_vec.push((offset, size));
    }
  }
}
impl Position {
  pub(crate) fn with<V>(self, val: V) -> WithPos<V> {
    WithPos { val, pos: self }
  }
}
impl LabelSize {
  pub(crate) fn to_int(self) -> i32 {
    match self {
      Heap => 8,
      Size(size) => size,
    }
  }
}
impl Address {
  pub(crate) fn modrm_sib_disp(self) -> u32 {
    match self {
      Global(_) => 5,
      Local(_, offset) => 1 + Disp::from(offset).sizeof(Rbp as u8),
    }
  }
}
impl<T> From<T> for Operand<T>
where
  T: Copy + Add<Output = T>,
{
  fn from(src: T) -> Operand<T> {
    Operand::Imm(src)
  }
}
impl<T> From<Register> for Operand<T> {
  fn from(src: Register) -> Operand<T> {
    Operand::Reg(src)
  }
}
impl<T> From<Address> for Operand<T> {
  fn from(src: Address) -> Operand<T> {
    Operand::Mem(src)
  }
}
