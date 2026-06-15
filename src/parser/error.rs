use crate::prelude::*;
use std::num::TryFromIntError;
pub(crate) type ParseErrOR<T> = Result<T, Pos<ParseErr>>;
pub(crate) type ErrOR<T> = Result<T, JsonpilerErr>;
#[derive(Debug, Clone)]
pub(crate) struct JsonpilerErr {
  pub kind: JsonpilerErrKind,
  pub refs: Vec<Position>,
}
#[derive(Debug, Clone)]
pub(crate) enum JsonpilerErrKind {
  Compilation(CompilationErr),
  Internal(InternalErr),
  Parse(ParseErr),
  Platform(String),
  Warning(Warning),
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum Arity {
  AtLeast(u32),
  #[expect(dead_code)]
  AtMost(u32),
  Exact(u32),
  Range(u32, u32),
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum RuntimeErr {
  AssertionErr,
  // Debug,
  RuntimeOverflow,
  RuntimeTooLargeShift,
  RuntimeZeroDivision,
  SecondaryGUIErr,
}
#[derive(Debug, Clone)]
pub(crate) enum CompilationErr {
  ArityErr { name: String, expected: Arity, actual: u32 },
  DuplicateName { first: NameKind, kind: NameKind, name: String },
  IOErr(String),
  IfNoTrueBranch,
  IncludeFuncNotFound(BTreeSet<String>),
  OutSideErr { name: String, place: &'static str },
  Overflow,
  RecursiveInclude(String),
  TooLargeFile,
  TooLargeShift,
  TypeErr { name: String, expected: Vec<JsonType>, actual: JsonType },
  UndefinedFunc(String),
  UndefinedVar(String),
  UnknownType(String),
  UnsupportedFile,
  UnsupportedType(String),
  ZeroDivision,
}
#[derive(Debug, Clone)]
pub(crate) enum ParseErr {
  ExpectedIdent,
  ExpectedToken(TokenKind),
  IntOutOfRange,
  InvalidChar,
  InvalidFloat,
  InvalidKeyword,
  UnexpectedToken(TokenKind),
  UnterminatedLiteral,
}
#[derive(Debug, Clone)]
pub(crate) enum Warning {
  EarlyElse,
  UnreachableIf,
  UnreachableWhile,
  UnusedName(NameKind, String),
  UselessIfTrue,
  UselessLiteral,
}
#[derive(Debug, Clone)]
pub(crate) enum TokenKind {
  Char(char),
  Digits,
  Eof,
  Esc(char),
  Separate,
}
#[derive(Debug, Clone)]
pub(crate) enum InternalErr {
  A64NotImplemented(String),
  ArgNotFound(String, u32),
  CastError,
  DuplicateLabel,
  InternalOverFlow,
  InvalidInst(String),
  MismatchInstGen,
  MissingFirstParser,
  StackLeak,
  UnknownLabel,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum NameKind {
  Argument,
  BuiltInFunc,
  GlobalVar,
  #[default]
  LocalVar,
  UserDefinedFunc,
}
impl From<Pos<ParseErr>> for JsonpilerErr {
  fn from(Pos { val: err, pos }: Pos<ParseErr>) -> Self {
    JsonpilerErr { kind: Parse(err), refs: vec![pos] }
  }
}
impl From<InternalErr> for JsonpilerErr {
  fn from(kind: InternalErr) -> Self {
    JsonpilerErr { kind: Internal(kind), refs: vec![] }
  }
}
impl From<TryFromIntError> for JsonpilerErr {
  fn from(_: TryFromIntError) -> Self {
    JsonpilerErr { kind: Internal(CastError), refs: vec![] }
  }
}
impl From<io::Error> for JsonpilerErr {
  fn from(err: io::Error) -> Self {
    JsonpilerErr { kind: Compilation(IOErr(err.to_string())), refs: vec![] }
  }
}
impl From<Pos<io::Error>> for JsonpilerErr {
  fn from(err: Pos<io::Error>) -> Self {
    JsonpilerErr { kind: Compilation(IOErr(err.val.to_string())), refs: vec![err.pos] }
  }
}
impl fmt::Display for JsonpilerErrKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Compilation(kind) => write!(f, "{kind}"),
      Parse(kind) => write!(f, "{kind}"),
      Internal(kind) => write!(f, "{kind}"),
      Platform(err_str) => write!(f, "{err_str}"),
      Warning(warn) => write!(f, "{warn}"),
    }
  }
}
impl JsonpilerErrKind {
  pub(crate) fn issue_msg(&self) -> Option<String> {
    match self {
      Compilation(_) | Parse(_) | Platform(_) | Warning(_) => None,
      Internal(kind) => Some(format!("{ISSUE}{}`", kind.err_code())),
    }
  }
  pub(crate) fn title(&self) -> String {
    make_header(match self {
      Compilation(_) => "CompilationError",
      Parse(_) => "ParseError",
      Internal(_) => "InternalError",
      Platform(_) => "PlatformError",
      Warning(_) => "Warning",
    })
  }
}
impl JsonpilerErr {
  fn format_err(
    &self,
    mut get_info: impl FnMut(Position) -> (String, String, String, String),
  ) -> String {
    let mut err_str = self.kind.title();
    err_str.push_str(&wrap_text(&self.kind.to_string(), 28));
    for pos in self.refs.iter().rev() {
      let (file_str, l_c, code, carets) = get_info(*pos);
      err_str.push_str(&format!("{ERR_SEP}{file_str}{l_c}{ERR_SEP}{code}| {carets}"));
    }
    err_str.push_str(ERR_END);
    if let Some(issue_msg) = self.kind.issue_msg() {
      err_str.push_str(&issue_msg);
    }
    err_str
  }
}
impl Jsonpiler {
  pub(crate) fn format_err(&self, err: &JsonpilerErr) -> String {
    err.format_err(|pos| self.files[pos.file].err_info(pos))
  }
}
impl Pos<Parser> {
  pub(crate) fn format_err(&self, err: &JsonpilerErr, rel_path: &str) -> String {
    err.format_err(|pos| self.err_info(pos, rel_path))
  }
}
impl File {
  pub(crate) fn err_info(&self, pos: Position) -> (String, String, String, String) {
    self.parser.err_info(pos, &self.rel_path)
  }
}
impl Pos<Parser> {
  #[must_use]
  pub(crate) fn err_info(&self, pos: Position, rel_path: &str) -> (String, String, String, String) {
    let find_ln = |i: &usize| self.val.text.as_bytes()[*i] == b'\n';
    let len = self.val.text.len();
    let index = (pos.offset as usize).min(len);
    let start = (0..index).rfind(&find_ln).map_or(0, |st| st + 1);
    let end = (index..len).find(&find_ln).unwrap_or(len);
    let line = String::from_utf8_lossy(&self.val.text.as_bytes()[start..end]);
    let carets_off = index - start;
    let carets = (pos.size as usize).min(end - index).max(1);
    (
      rel_path.into(),
      format!(":{}:{}", pos.line + 1, carets_off + 1),
      format!("{line}\n"),
      format!("{}{}", " ".repeat(carets_off), "^".repeat(carets)),
    )
  }
}
impl fmt::Display for CompilationErr {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Overflow => write!(f, "Overflow"),
      UnsupportedType(typ) => write!(f, "Unsupported type: {typ}"),
      UnknownType(typ) => write!(f, "Unknown type: {typ}"),
      UndefinedVar(var) => write!(f, "Undefined variable: {var}"),
      UndefinedFunc(func) => write!(f, "Undefined function: {func}"),
      UnsupportedFile => write!(f, "Unsupported file: .json or .jspl expected"),
      RecursiveInclude(file) => write!(f, "Recursive include: {file}"),
      DuplicateName { first, kind, name } => {
        write!(f, "Duplicate {kind}: {first} `{name}` already exists")
      }
      OutSideErr { name, place } => write!(f, "`{name}` outside of {place}"),
      TypeErr { name, expected, actual: typ } => {
        write!(
          f,
          "{name} expected type `{}`, but got `{typ}`",
          expected.iter().map(JsonType::name).collect::<Vec<_>>().join("` or `")
        )
      }
      ArityErr { name, expected, actual } => {
        let be = if *actual == 1 { "is" } else { "are" };
        write!(f, "`{name}` requires {expected},\n  but {actual} {be} supplied")
      }
      ZeroDivision => write!(f, "{ZERO_DIVISION}"),
      IOErr(err) => write!(f, "IOError: {err}"),
      IncludeFuncNotFound(funcs) => {
        write!(f, "Function is either private or not found:")?;
        for func in funcs {
          write!(f, "\n- {func}")?;
        }
        Ok(())
      }
      TooLargeFile => {
        write!(f, "Input file size exceeds 1 GB. Please provide a smaller file.")
      }
      TooLargeShift => write!(f, "{TOO_LARGE_SHIFT}"),
      IfNoTrueBranch => write!(f, "{IF_NO_TRUE_BRANCH}"),
    }
  }
}
impl fmt::Display for InternalErr {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      MismatchInstGen => write!(f, "Mismatch instruction generation"),
      InternalOverFlow => write!(f, "Overflow"),
      DuplicateLabel => write!(f, "Duplicate label"),
      UnknownLabel => write!(f, "Unknown label"),
      InvalidInst(inst) => write!(f, "Invalid instruction: {inst}"),
      ArgNotFound(name, nth) => write!(f, "The {nth} argument of `{name}` does not exist"),
      CastError => write!(f, "Cast error"),
      MissingFirstParser => write!(f, "Missing first parser"),
      StackLeak => write!(f, "Stack is not fully released"),
      A64NotImplemented(feature) => write!(f, "A64 unimplemented feature: {feature}"),
    }
  }
}
impl fmt::Display for RuntimeErr {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      RuntimeOverflow => write!(f, "Overflow"),
      RuntimeZeroDivision => write!(f, "{ZERO_DIVISION}"),
      RuntimeTooLargeShift => write!(f, "{TOO_LARGE_SHIFT}"),
      AssertionErr => write!(f, "AssertionError:\n|   "),
      // Debug => write!(f, "Debug"),
      SecondaryGUIErr => write!(f, "SecondaryGUIError"),
    }
  }
}
impl fmt::Display for TokenKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      TokenKind::Esc(esc) => write!(f, "escape sequence: `\\{esc}`"),
      TokenKind::Char(ch) => write!(f, "character: `{ch}`"),
      TokenKind::Eof => write!(f, "end of file"),
      TokenKind::Separate => write!(f, "newline or semicolon"),
      TokenKind::Digits => write!(f, "digits"),
    }
  }
}
impl fmt::Display for NameKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(match self {
      Argument => "argument",
      BuiltInFunc => "built-in function",
      UserDefinedFunc => "user-defined function",
      GlobalVar => "global variable",
      LocalVar => "local variable",
    })
  }
}
impl fmt::Display for Arity {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{} argument{}", self.range(), self.plural())
  }
}
impl fmt::Display for ParseErr {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      UnexpectedToken(token) => write!(f, "Unexpected {token}"),
      UnterminatedLiteral => write!(f, "Unterminated string literal"),
      InvalidFloat => write!(f, "Invalid float"),
      InvalidKeyword => write!(f, "Invalid keyword"),
      IntOutOfRange => write!(f, "Integer out of range"),
      InvalidChar => write!(f, "Invalid character"),
      ExpectedIdent => write!(f, "Expected identifier"),
      ExpectedToken(token) => write!(f, "Expected {token}"),
    }
  }
}
impl fmt::Display for Warning {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      UselessLiteral => write!(f, "This literal is useless."),
      EarlyElse => write!(f, "The following `if` branch is unreachable."),
      UselessIfTrue => write!(f, "This `if` branch is always reachable."),
      UnreachableIf => write!(f, "This `if` branch is unreachable."),
      UnreachableWhile => write!(f, "This `while` loop body is unreachable."),
      UnusedName(kind, name) => write!(f, "Unused {kind}: `{name}`"),
    }
  }
}
impl Arity {
  fn plural(&self) -> &'static str {
    match self {
      Exact(1) | AtLeast(1) | AtMost(1) => "",
      AtLeast(_) | AtMost(_) | Exact(_) | Range(..) => "s",
    }
  }
  fn range(&self) -> String {
    match self {
      Exact(n) => n.to_string(),
      AtLeast(0) => "any".into(),
      AtLeast(min) => format!("at least {min}"),
      AtMost(max) => format!("at most {max}"),
      Range(min, max) => format!("between {min} and {max}"),
    }
  }
}
impl InternalErr {
  pub(crate) fn err_code(&self) -> &'static str {
    match self {
      DuplicateLabel => "DUPLICATE_LABEL",
      InternalOverFlow => "OVERFLOW",
      UnknownLabel => "UNKNOWN_LABEL",
      InvalidInst(_) => "INVALID_INST",
      ArgNotFound(..) => "ARG_NOT_FOUND",
      A64NotImplemented(_) => "A64_UNIMPLEMENTED",
      CastError => "CAST_ERROR",
      StackLeak => "STACK_LEAK",
      MissingFirstParser => "MISSING_FIRST_PARSER",
      MismatchInstGen => "MISMATCH_INST_GEN",
    }
  }
}
impl Jsonpiler {
  #[expect(clippy::print_stderr)]
  pub(crate) fn warn(&mut self, warn: Pos<Warning>) -> ErrOR<()> {
    let file = &mut self.files[warn.pos.file];
    file.parser.val.warns.push(warn.clone());
    eprintln!("{}", self.format_err(&warning!(warn.pos, warn.val.clone())));
    Ok(())
  }
}
impl Pos<BuiltIn> {
  pub(crate) fn args_err(&mut self, expected: Vec<JsonType>, actual: &Pos<Json>) -> JsonpilerErr {
    type_err(fmt_args(self.val.nth, &self.val.name), expected, actual.map_ref(Json::as_type))
  }
  pub(crate) fn validate_args(&self, expected: Arity) -> ErrOR<()> {
    let name = self.val.name.clone();
    let actual = self.val.len;
    if match expected {
      Exact(n) => actual == n,
      AtLeast(min) => min <= actual,
      AtMost(max) => actual <= max,
      Range(min, max) => min <= actual && actual <= max,
    } {
      Ok(())
    } else {
      err!(self.pos, ArityErr { name, expected, actual })
    }
  }
}
pub(crate) fn fmt_args(nth: u32, name: &str) -> String {
  let suffix = match nth % 10 {
    _ if (11..=13).contains(&(nth % 100)) => "th",
    1 => "st",
    2 => "nd",
    3 => "rd",
    _ => "th",
  };
  format!("{nth}{suffix} argument of `{name}`")
}
pub(crate) fn type_err(
  name: String,
  expected: Vec<JsonType>,
  Pos { val: actual, pos }: Pos<JsonType>,
) -> JsonpilerErr {
  compilation!(pos, TypeErr { name, expected, actual })
}
fn char_width(char: char) -> usize {
  if char.is_ascii() { 1 } else { 2 }
}
fn wrap_text(string: &str, max_width: usize) -> String {
  let mut result = String::new();
  for line in string.lines() {
    for wrapped in wrap_line(line, max_width) {
      result.push_str(&format!("\n| {wrapped}"));
    }
  }
  result
}
fn wrap_line(string: &str, max_width: usize) -> Vec<String> {
  let mut result = Vec::new();
  let mut current = String::new();
  let mut width = 0;
  let mut last_space_byte = None;
  for char in string.chars() {
    let w = char_width(char);
    if width + w > max_width {
      if let Some(space_pos) = last_space_byte {
        let (line, rest) = current.split_at(space_pos);
        result.push(line.into());
        current = rest.trim_start().into();
        width = current.chars().map(char_width).sum();
      } else {
        result.push(take(&mut current));
        width = 0;
      }
      last_space_byte = None;
    }
    current.push(char);
    width += w;
    if char == ' ' {
      last_space_byte = Some(current.len());
    }
  }
  if !current.is_empty() {
    result.push(current);
  }
  result
}
pub(crate) fn make_header(title: &str) -> String {
  const PREFIX: &str = "\n\u{256d}-";
  const SPACES: usize = 2;
  let base_len = PREFIX.chars().count() + title.chars().count() + SPACES;
  let dash_len = 30usize.saturating_sub(base_len);
  format!("{PREFIX} {title} {}", "-".repeat(dash_len))
}
pub(crate) fn fmt_var(name: &str, kind: NameKind) -> String {
  format!("{kind} `{name}`")
}
pub(crate) fn fmt_ret_val(name: &str) -> String {
  format!("the return value of `{name}`")
}
