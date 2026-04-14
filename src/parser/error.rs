use crate::prelude::*;
use std::{io::Error, num, path};
pub(crate) type ParseErrOR<T> = Result<T, WithPos<ParseErr>>;
pub(crate) type ErrOR<T> = Result<T, JsonpilerErr>;
#[derive(Debug, Clone)]
pub(crate) enum JsonpilerErr {
  Compilation(CompilationErr, Vec<Position>),
  IO(String),
  Internal(InternalErr),
  Parse(ParseErr, Vec<Position>),
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
  ArityError { name: String, expected: Arity, actual: u32 },
  DuplicateName(NameKind, String),
  IncludeFuncNotFound(BTreeSet<String>),
  IncludeIOError(String),
  OutSideError { kind: String, place: &'static str },
  Overflow,
  RecursiveInclude(String),
  TooLargeFile,
  TooLargeShift,
  TypeError { name: String, expected: Vec<JsonType>, actual: JsonType },
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
  ArgNotFound(String, u32),
  CastError,
  DuplicateLabel,
  InternalOverFlow,
  InvalidInst(String),
  StackLeak,
  UnknownLabel,
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum NameKind {
  BuiltInFunc,
  GlobalVar,
  LocalVar,
  UserDefinedFunc,
}
impl From<WithPos<ParseErr>> for JsonpilerErr {
  fn from(WithPos { val: err, pos }: WithPos<ParseErr>) -> Self {
    Parse(err, vec![pos])
  }
}
impl From<num::TryFromIntError> for JsonpilerErr {
  fn from(_: num::TryFromIntError) -> Self {
    Internal(CastError)
  }
}
impl From<io::Error> for JsonpilerErr {
  fn from(err: io::Error) -> Self {
    IO(err.to_string())
  }
}
impl From<WithPos<io::Error>> for JsonpilerErr {
  fn from(err: WithPos<io::Error>) -> Self {
    Compilation(IncludeIOError(err.val.to_string()), vec![err.pos])
  }
}
impl fmt::Display for JsonpilerErr {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Compilation(kind, _) => write!(f, "{kind}"),
      Parse(kind, _) => write!(f, "{kind}"),
      Internal(kind) => write!(f, "{kind}"),
      IO(err_str) => write!(f, "{err_str}"),
    }
  }
}
impl JsonpilerErr {
  pub(crate) fn issue_msg(&self) -> Option<String> {
    match self {
      Compilation(..) | Parse(..) | IO(_) => None,
      Internal(kind) => Some(format!("{ISSUE}{}`", kind.err_code())),
    }
  }
  pub(crate) fn pos_vec(&self) -> Vec<Position> {
    match self {
      Parse(_, pos_vec) | Compilation(_, pos_vec) => pos_vec.clone(),
      IO(_) | Internal(_) => vec![],
    }
  }
  pub(crate) fn title(&self) -> String {
    make_header(match self {
      Compilation(..) => "CompilationError",
      Parse(..) => "ParseError",
      Internal(_) => "InternalError",
      IO(_) => "IOError",
    })
  }
}
impl Jsonpiler {
  pub(crate) fn format_err(&self, err: &JsonpilerErr) -> String {
    let mut err_str = err.title().clone();
    err_str.push_str(&wrap_text(&err.to_string(), 28));
    let pos_vec = err.pos_vec();
    if !pos_vec.is_empty() {
      for pos in pos_vec.iter().rev() {
        let (file_str, l_c, code, carets) = self.parsers[pos.file as usize].err_info(*pos);
        err_str.push_str(&format!("{ERR_SEPARATE}{file_str}{l_c}{ERR_SEPARATE}{code}| {carets}"));
      }
    }
    err_str.push_str(ERR_END);
    if let Some(issue_msg) = err.issue_msg() {
      err_str.push_str(&issue_msg);
    }
    err_str
  }
  #[expect(clippy::needless_pass_by_value)]
  pub(crate) fn io_err(&self, err: Error) -> String {
    self.format_err(&IO(err.to_string()))
  }
}
impl Parser {
  #[must_use]
  pub(crate) fn err_info(&self, pos: Position) -> (String, String, String, String) {
    let mut root =
      Path::new(&self.root_file).parent().unwrap_or(Path::new("C:")).to_string_lossy().to_string();
    root.push(path::MAIN_SEPARATOR);
    let find_ln = |i: &usize| self.source[*i] == b'\n';
    let len = self.source.len();
    let index = (pos.offset as usize).min(len);
    let start = (0..index).rfind(&find_ln).map_or(0, |st| st + 1);
    let end = (index..len).find(&find_ln).unwrap_or(len);
    let line = String::from_utf8_lossy(&self.source[start..end]);
    let carets_offset = index - start;
    let carets = (pos.size as usize).min(end - index).max(1);
    let file_path = self.file.strip_prefix(&root).unwrap_or(&self.file).into();
    (
      file_path,
      format!(":{}:{}", pos.line, carets_offset + 1),
      format!("{line}\n"),
      format!("{}{}", " ".repeat(carets_offset), "^".repeat(carets)),
    )
  }
}
impl fmt::Display for CompilationErr {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Overflow => write!(f, "Overflow"),
      UnsupportedType(typ) => write!(f, "Unsupported type:\n  {typ}"),
      UnknownType(typ) => write!(f, "Unknown type:\n  {typ}"),
      UndefinedVar(var) => write!(f, "Undefined variable:\n  {var}"),
      UndefinedFunc(func) => write!(f, "Undefined function:\n  {func}"),
      UnsupportedFile => write!(f, "Unsupported file:\n  .json or .jspl expected"),
      RecursiveInclude(file) => write!(f, "Recursive include:\n  {file}"),
      DuplicateName(kind, name) => write!(f, "`{name}` is already defined as a {kind}"),
      OutSideError { kind, place } => write!(f, "`{kind}` is not allowed outside a {place}"),
      TypeError { name, expected, actual: typ } => {
        write!(
          f,
          "{name} expected type `{}`,\n  but got `{typ}`",
          expected.iter().map(JsonType::name).collect::<Vec<_>>().join("` or `")
        )
      }
      ArityError { name, expected, actual } => {
        let be = if *actual == 1 { "is" } else { "are" };
        write!(f, "`{name}` requires {expected},\n  but {actual} {be} supplied")
      }
      ZeroDivision => write!(f, "{ZERO_DIVISION}"),
      IncludeIOError(err) => write!(f, "IOError:\n  {err}"),
      IncludeFuncNotFound(funcs) => {
        write!(f, "Function is either private or not found:")?;
        for func in funcs {
          write!(f, "\n- {func}")?;
        }
        Ok(())
      }
      TooLargeFile => {
        write!(f, "Input file size exceeds 1 GB.\n  Please provide a smaller file.")
      }
      TooLargeShift => write!(f, "{TOO_LARGE_SHIFT}"),
    }
  }
}
impl fmt::Display for InternalErr {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      InternalOverFlow => write!(f, "Overflow"),
      DuplicateLabel => write!(f, "Duplicate label"),
      UnknownLabel => write!(f, "Unknown label"),
      InvalidInst(inst) => write!(f, "Invalid instruction:\n  {inst}"),
      ArgNotFound(name, nth) => write!(f, "The {nth} argument of `{name}` does not exist"),
      CastError => write!(f, "Cast error"),
      StackLeak => write!(f, "Stack is not fully released"),
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
      CastError => "CAST_ERROR",
      StackLeak => "STACK_LEAK",
    }
  }
}
impl Jsonpiler {
  pub(crate) fn warn(&mut self, pos: Position, err: Warning) {
    self.parsers[pos.file as usize].warn(pos, err);
  }
}
impl Parser {
  #[expect(clippy::print_stderr)]
  pub(crate) fn warn(&mut self, pos: Position, err: Warning) {
    let (file, l_c, code, carets) = self.err_info(pos);
    eprintln!("{WARNING}\n| {err}{ERR_SEPARATE}{file}{l_c}{ERR_SEPARATE}{code}| {carets}{ERR_END}");
    self.warns.push(pos.with(err));
  }
}
impl BuiltIn {
  pub(crate) fn validate_args(&self, expected: Arity) -> ErrOR<()> {
    let name = self.name.clone();
    let actual = self.len;
    if match expected {
      Exact(n) => actual == n,
      AtLeast(min) => min <= actual,
      AtMost(max) => actual <= max,
      Range(min, max) => min <= actual && actual <= max,
    } {
      Ok(())
    } else {
      err!(self.pos, ArityError { name, expected, actual })
    }
  }
}
pub(crate) fn args_type_err(
  nth: u32,
  name: &str,
  expected: Vec<JsonType>,
  json_type: WithPos<JsonType>,
) -> JsonpilerErr {
  let suffix = match nth % 10 {
    _ if (11..=13).contains(&(nth % 100)) => "th",
    1 => "st",
    2 => "nd",
    3 => "rd",
    _ => "th",
  };
  type_err(format!("{nth}{suffix} argument of `{name}`"), expected, json_type)
}
pub(crate) fn type_err(
  name: String,
  expected: Vec<JsonType>,
  json_type: WithPos<JsonType>,
) -> JsonpilerErr {
  Compilation(TypeError { name, expected, actual: json_type.val }, vec![json_type.pos])
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
      #[expect(clippy::assigning_clones)]
      if let Some(space_pos) = last_space_byte {
        let (line, rest) = current.split_at(space_pos);
        result.push(line.to_owned());
        current = rest.trim_start().to_owned();
        width = current.chars().map(char_width).sum();
      } else {
        result.push(current.clone());
        current.clear();
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
