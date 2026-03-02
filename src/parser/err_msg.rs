use crate::prelude::*;
use std::{fmt, fs, io, num, path};
pub(crate) enum JsonpilerErr {
  Compilation(CompilationErrKind, Position),
  IO(String),
  Internal(InternalErrKind),
}
pub(crate) enum CompilationErrKind {
  ArityError { name: String, expected: Arity, actual: usize },
  ExistentFunc(FunctionKind, String),
  ExistentVar(String),
  ExpectedIdent,
  ExpectedToken(TokenKind),
  IncludeFuncNotFound(Vec<String>),
  IncludeIOError(io::Error),
  IntegerOutOfRange,
  InvalidChar,
  OutSideError { kind: String, place: &'static str },
  ParentDirNotFound,
  ParseError(&'static str),
  RecursiveInclude(String),
  StartsWithZero,
  TooLargeFile,
  TypeError { name: String, expected: String, typ: String },
  UndefinedFn(String),
  UndefinedVar(String),
  UnexpectedLiteral,
  UnexpectedToken(TokenKind),
  UnknownType(String),
  UnsupportedFile,
  UnsupportedType(String),
  UnterminatedLiteral,
  ZeroDivision,
}
pub(crate) enum TokenKind {
  Char(char),
  Eof,
  Esc(char),
  Separate,
}
pub(crate) enum InternalErrKind {
  CastError,
  InvalidInst(String),
  NonExistentArg(String, usize),
  OverFlow,
  UnbalancedStack,
  UnknownLabel,
}
pub(crate) enum FunctionKind {
  Builtin,
  UserDefined,
}
impl From<num::TryFromIntError> for JsonpilerErr {
  fn from(_: num::TryFromIntError) -> Self {
    Internal(CastError)
  }
}
impl From<io::IntoInnerError<io::BufWriter<fs::File>>> for JsonpilerErr {
  fn from(err: io::IntoInnerError<io::BufWriter<fs::File>>) -> Self {
    IO(format!("{err}"))
  }
}
impl From<io::Error> for JsonpilerErr {
  fn from(err: io::Error) -> Self {
    IO(format!("{err}"))
  }
}
impl From<WithPos<io::Error>> for JsonpilerErr {
  fn from(err: WithPos<io::Error>) -> Self {
    Compilation(IncludeIOError(err.val), err.pos)
  }
}
impl Parser {
  #[must_use]
  pub(crate) fn err_info(&self, pos: Position, file: &str) -> (String, String, String, String) {
    let mut root =
      path::Path::new(file).parent().map_or(String::new(), |dir| dir.to_string_lossy().to_string());
    root.push(path::MAIN_SEPARATOR);
    let find_ln = |i: &usize| self.source[*i] == b'\n';
    let len = self.source.len();
    let index = pos.offset.min(len.saturating_sub(1));
    let start = (0..index).rfind(&find_ln).map_or(0, |st| st + 1);
    let end = (index..len).find(&find_ln).unwrap_or(len);
    let line = String::from_utf8_lossy(&self.source[start..end]);
    let carets_offset = index - start;
    let carets = pos.size.min(end - index).max(1);
    let file_path = self.file.strip_prefix(&root).unwrap_or(&self.file).into();
    (
      file_path,
      format!(":{}:{}", pos.line, carets_offset + 1),
      format!("{line}\n"),
      format!("{}{}", " ".repeat(carets_offset), "^".repeat(carets)),
    )
  }
}
impl fmt::Display for CompilationErrKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      UnexpectedLiteral => write!(f, "Unexpected literal in non-final position inside block"),
      UnsupportedType(typ) => write!(f, "Unsupported type:\n|   {typ}"),
      UnknownType(typ) => write!(f, "Unknown type:\n|   {typ}"),
      IntegerOutOfRange => write!(f, "Integer out of range"),
      UndefinedVar(var) => write!(f, "Undefined variable:\n|   {var}"),
      UndefinedFn(func) => write!(f, "Undefined function:\n|   {func}"),
      ExpectedToken(token) => write!(f, "Expected {token}"),
      UnexpectedToken(token) => write!(f, "Unexpected {token}"),
      UnterminatedLiteral => write!(f, "Unterminated string literal"),
      UnsupportedFile => write!(f, "Unsupported file:\n|   .json or .jspl expected"),
      RecursiveInclude(file) => write!(f, "Recursive include:\n|   {file}"),
      ExistentFunc(kind, name) => write!(f, "{kind} function `{name}` already exists"),
      ExistentVar(name) => write!(f, "Variable `{name}` already exists"),
      ExpectedIdent => write!(f, "Expected identifier"),
      StartsWithZero => write!(f, "Number cannot start with zero"),
      OutSideError { kind, place } => write!(f, "`{kind}` can only be used inside a {place}"),
      TypeError { name, expected, typ } => {
        write!(f, "`{name}` expected type `{expected}`,\n|   but got `{typ}`")
      }
      ArityError { name, expected, actual } => {
        let be = if *actual == 1 { "is" } else { "are" };
        write!(f, "`{name}` requires {expected},\n|   but {} {be} supplied", Exactly(*actual))
      }
      InvalidChar => write!(f, "Non-printable or invalid character"),
      ParseError(typ) => write!(f, "Failed to parse `{typ}`"),
      ZeroDivision => write!(f, "Division by zero"),
      IncludeIOError(err) => write!(f, "IOError:\n|   {err}"),
      IncludeFuncNotFound(funcs) => {
        write!(f, "function not found:\n|   - {}", funcs.join("\n|   - "))
      }
      TooLargeFile => write!(f, "Input file size exceeds 1GB.\n|   Please provide a smaller file."),
      ParentDirNotFound => write!(f, "Parent directory not found"),
    }
  }
}
impl fmt::Display for InternalErrKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      OverFlow => write!(f, "Overflow"),
      UnknownLabel => write!(f, "Unknown label"),
      InvalidInst(inst) => write!(f, "Invalid instruction:\n|   {inst}"),
      NonExistentArg(name, nth) => write!(f, "Non-existent {nth} argument of `{name}`"),
      CastError => write!(f, "Cast error"),
      UnbalancedStack => write!(f, "stack is not fully released"),
    }
  }
}
impl fmt::Display for TokenKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      TokenKind::Esc(esc) => write!(f, "escape sequence: `\\{esc}`"),
      TokenKind::Char(ch) => write!(f, "character: `{ch}`"),
      TokenKind::Eof => write!(f, "EOF"),
      TokenKind::Separate => write!(f, "newline or semicolon"),
    }
  }
}
impl fmt::Display for FunctionKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(match self {
      Builtin => "Builtin",
      UserDefined => "User-defined",
    })
  }
}
impl fmt::Display for Arity {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{} argument{}", self.range(), self.plural())
  }
}
impl Arity {
  fn plural(&self) -> &'static str {
    match self {
      Exactly(1) | AtLeast(1) | AtMost(1) => "",
      Zero | Any | AtLeast(_) | AtMost(_) | Exactly(_) | Range(..) => "s",
    }
  }
  fn range(&self) -> String {
    match self {
      Exactly(n) => n.to_string(),
      AtLeast(min) => format!("at least {min}"),
      AtMost(max) => format!("at most {max}"),
      Range(min, max) => format!("between {min} to {max}"),
      Zero => "0".into(),
      Any => "any".into(),
    }
  }
}
impl InternalErrKind {
  pub(crate) fn err_code(&self) -> String {
    match self {
      OverFlow => "C0000",
      UnknownLabel => "C0001",
      InvalidInst(_) => "C0002",
      NonExistentArg(..) => "C0003",
      CastError => "C0004",
      UnbalancedStack => "C0005",
    }
    .into()
  }
}
pub(crate) fn args_type_err(
  nth: usize,
  name: &str,
  expected: String,
  json: &WithPos<Json>,
) -> JsonpilerErr {
  let suffix = match nth % 10 {
    _ if (11usize..=13).contains(&(nth % 100)) => "th",
    1 => "st",
    2 => "nd",
    3 => "rd",
    _ => "th",
  };
  type_err(format!("{nth}{suffix} argument` of `{name}"), expected, json)
}
pub(crate) fn type_err(name: String, expected: String, json: &WithPos<Json>) -> JsonpilerErr {
  let typ = json.val.describe();
  Compilation(TypeError { name, expected, typ }, json.pos)
}
