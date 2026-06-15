#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum A64Sect {
  BssA,
  CString,
  DataA,
  Got,
  LaSym,
  StubHelper,
  Stubs,
  TextA,
}
