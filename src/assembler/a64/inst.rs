use crate::prelude::*;
#[expect(clippy::arbitrary_source_item_ordering)]
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum A64Inst {
  // arithmetic
  AddR3(A64Reg, A64Reg, A64Reg),
  AddSR3(A64Reg, A64Reg, A64Reg),
  AddRI12(A64Reg, A64Reg, u16),
  SubR3(A64Reg, A64Reg, A64Reg),
  SubSR3(A64Reg, A64Reg, A64Reg),
  SubRI12(A64Reg, A64Reg, u16),
  MulR3(A64Reg, A64Reg, A64Reg),
  SMulH(A64Reg, A64Reg, A64Reg),
  SDivR3(A64Reg, A64Reg, A64Reg),
  // logic
  AndR3(A64Reg, A64Reg, A64Reg),
  OrrR3(A64Reg, A64Reg, A64Reg),
  OrnR3(A64Reg, A64Reg, A64Reg),
  EorR3(A64Reg, A64Reg, A64Reg),
  TstRb(A64Reg),
  Asr(A64Reg, A64Reg, u8),
  Lsl(A64Reg, A64Reg, u8),
  AsrR3(A64Reg, A64Reg, A64Reg),
  LslR3(A64Reg, A64Reg, A64Reg),
  // move
  MovRR(A64Reg, A64Reg),
  MovZ(A64Reg, u16, Shift16),
  MovK(A64Reg, u16, Shift16),
  MovN(A64Reg, u16, Shift16),
  // memory
  LblA(LabelId),
  RetA,
  Adrp(A64Reg, LabelId),
  AddLbl(A64Reg, LabelId),
  LdR(RegSize, A64Reg, A64Reg, u16),
  StR(RegSize, A64Reg, A64Reg, u16),
  FLdRD(A64Reg, A64Reg, u16),
  FStRD(A64Reg, A64Reg, u16),
  Stp(A64Reg, A64Reg, A64Reg, i8),
  Ldp(A64Reg, A64Reg, A64Reg, i8),
  // floating point
  #[expect(dead_code)]
  FMovDX(A64Reg, A64Reg),
  #[expect(dead_code)]
  FMovXD(A64Reg, A64Reg),
  FArithD(ArithSdKind, A64Reg, A64Reg, A64Reg),
  FNegD(A64Reg, A64Reg),
  FSqrtD(A64Reg, A64Reg),
  SCvtFD(A64Reg, A64Reg),
  FCvtZSD(A64Reg, A64Reg),
  FCmpD(A64Reg, A64Reg),
  FAbsD(A64Reg, A64Reg),
  // compare / branch
  CmpRR(A64Reg, A64Reg),
  CmpRI12(A64Reg, u16),
  CSet(A64Reg, A64Cc),
  CSel(A64Reg, A64Reg, A64Reg, A64Cc),
  B_(LabelId),
  BCc(A64Cc, LabelId),
  Bl(LabelId),
  Blr(A64Reg),
  Br(A64Reg),
  BApi(Api),
}
