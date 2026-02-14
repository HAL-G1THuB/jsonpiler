use crate::{
  Bind::{self, Var},
  CompilationErrKind::*,
  /*Disp, */ ErrOR, FuncInfo,
  InternalErrKind::*,
  Json,
  JsonpilerErr::{self, *},
  Label,
  Memory::{self, *},
  Operand::{self, *},
  Register, TokenKind, WithPos,
};
use core::ops::Add;
use std::fmt;
use std::{io, num::TryFromIntError};
impl<T> Bind<T> {
  pub(crate) fn describe(&self, ty: &str) -> String {
    format!("{ty}{}", if let Var(label) = self { &format!("{label}") } else { " (Literal)" })
  }
}
impl Default for Memory {
  fn default() -> Self {
    Tmp { offset: 0 }
  }
}
impl<T> Default for Bind<T> {
  fn default() -> Self {
    Bind::Var(Label::default())
  }
}
impl FuncInfo {
  pub(crate) fn arg(&mut self) -> ErrOR<WithPos<Json>> {
    self.nth += 1;
    self.args.next().ok_or(InternalError(NonExistentArg))
  }
  pub(crate) fn push_free_tmp(&mut self, label: &Label) {
    if let Label { mem: Tmp { offset }, size } = label {
      self.free_list.push((*offset, *size));
    }
  }
}
impl fmt::Display for Label {
  #[expect(clippy::min_ident_chars)]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self.mem {
      Tmp { .. } => Ok(()),
      Local { .. } => write!(f, " (Local variable)"),
      Global { .. } | GlobalD { .. } => write!(f, " (Global variable)"),
    }
  }
}
impl Memory {
  #[rustfmt::skip]
  pub(crate) fn size_of_mo_si_di(&self) -> u32 {
    match *self {
      Global { .. } | GlobalD { .. } => 5,
      Local { offset } | Tmp { offset } => {
        if i8::try_from(-offset).is_ok() { 2 } else { 5 }
      }
    }
  }
}
/*
impl Disp {
  pub(crate) fn size(self) -> u32 {
    match self {
      Disp::Zero => 0,
      Disp::Byte(_) => 1,
      Disp::Dword(_) => 4,
    }
  }
}
*/
impl<T> From<T> for Operand<T>
where
  T: Copy + Add<Output = T>,
{
  fn from(src: T) -> Operand<T> {
    Imm(src)
  }
}
impl<T> From<Register> for Operand<T> {
  fn from(src: Register) -> Operand<T> {
    Reg(src)
  }
}
impl<T> From<Memory> for Operand<T> {
  fn from(src: Memory) -> Operand<T> {
    Mem(src)
  }
}
impl From<TryFromIntError> for JsonpilerErr {
  fn from(_: TryFromIntError) -> Self {
    InternalError(CastError)
  }
}
impl From<WithPos<io::Error>> for JsonpilerErr {
  fn from(err: WithPos<io::Error>) -> Self {
    CompilationError { kind: IOError(err.value), pos: err.pos }
  }
}
impl From<io::Error> for JsonpilerErr {
  fn from(err: io::Error) -> Self {
    InternalError(InternalIOError(err))
  }
}
impl fmt::Display for TokenKind {
  #[expect(clippy::min_ident_chars)]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      TokenKind::Char(c) => write!(f, "character: `{c}`"),
      TokenKind::Eof => write!(f, "EOF"),
      TokenKind::NewLineOrSemiColon => write!(f, "newline or semicolon"),
    }
  }
}
