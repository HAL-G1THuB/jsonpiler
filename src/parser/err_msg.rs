use crate::{
  Arity::{Any, AtLeast, AtMost, Exactly, NoArgs, Range},
  CompilationErrKind::{self, *},
  InternalErrKind::{self, *},
  Parser, Position,
};
use std::fmt;
impl Parser {
  #[must_use]
  // No risk of overflow
  pub(crate) fn fmt_err(&self, err: &str, pos: Position) -> String {
    let gen_err = |column: usize, msg: &str| -> String {
      format!(
        "{err}\nError at {} line: {} column: {column}\nError position:\n{msg}",
        self.file, pos.line
      )
    };
    if self.source.is_empty() {
      return gen_err(0, "\n^");
    }
    let find_ln = |i: &usize| self.source[*i] == b'\n';
    let len = self.source.len();
    let index = pos.offset.min(len);
    let start = (0..index).rfind(&find_ln).map_or(0, |st| st + 1);
    let end = (index..len).find(&find_ln).unwrap_or(len);
    let line = &self.source[start..end];
    let line_str = String::from_utf8_lossy(line);
    let caret_start = index - start;
    let caret_len = pos.size.max(1).min(end - index);
    let ws = " ".repeat(caret_start);
    let carets = "^".repeat(caret_len);
    gen_err(pos.offset - start, &format!("{line_str}\n{ws}{carets}"))
  }
}
impl fmt::Display for CompilationErrKind {
  #[expect(clippy::min_ident_chars)]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      UnexpectedLiteral => write!(f, "Unexpected literal in non-final position inside block"),
      UnsupportedType(typ) => write!(f, "Unsupported type: {typ}"),
      UnknownType(typ) => write!(f, "Unknown type: {typ}"),
      InvalidUnicodeEsc => write!(f, "Invalid Unicode escape sequence"),
      IntegerOutOfRange => write!(f, "Integer out of range"),
      InvalidEsc(char) => write!(f, "Invalid escape sequence: \\{char}"),
      UndefinedVar(var) => write!(f, "Undefined variable: {var}"),
      UndefinedFn(func) => write!(f, "Undefined function: {func}"),
      ExpectedTokenError(token) => write!(f, "Expected {token}"),
      UnexpectedTokenError(token) => write!(f, "Unexpected {token}"),
      UnterminatedLiteral => write!(f, "Unterminated string literal"),
      RecursiveInclude(file) => write!(f, "Recursive include: {file}"),
      InvalidIdentifier => write!(f, "Invalid identifier"),
      ExistentBuiltin(name) => write!(f, "Builtin function `{name}` already exists"),
      ExistentUserDefined(name) => write!(f, "User defined function `{name}` already exists"),
      ExistentVar(name) => write!(f, "Variable `{name}` already exists"),
      StartsWithZero => write!(f, "Number cannot start with zero"),
      OutSideError { kind, place } => write!(f, "`{kind}` can only be used inside a {place}"),
      TypeError { name, expected, typ } => {
        write!(f, "`{name}` expected type `{expected}`, but got `{typ}`")
      }
      ArityError { name, expected, supplied } => {
        let (text, cond) = match *expected {
          Exactly(n) => ("exactly", format!("{n} argument{}", plural(n))),
          AtLeast(min) => ("at least", format!("{min} argument{}", plural(min))),
          AtMost(max) => ("at most", format!("{max} argument{}", plural(max))),
          Range(min, max) => ("between", format!("{min} and {max} arguments")),
          NoArgs => ("exactly", "0 arguments".to_owned()),
          Any => ("any", "unreachable number of arguments".to_owned()),
        };
        let plural = plural(*supplied);
        let be = if *supplied == 1 { "is" } else { "are" };
        write!(f, "`{name}` requires {text} {cond}, but {supplied} argument{plural} {be} supplied")
      }
      InvalidChar => write!(f, "Non-printable or invalid character"),
      ParseError(typ) => write!(f, "Failed to parse `{typ}`"),
      ZeroDivisionError => write!(f, "Division by zero"),
      UnsupportedExtension => write!(f, "Input file must be .json or .jspl"),
      IOError(err) => write!(f, "IOError: {err}"),
      IncludeFuncNotFound(funcs) => {
        write!(f, "IncludeError: function not found:\n- {}", funcs.join("\n- "))
      }
      TooLargeFile => write!(f, "Input file size exceeds 1GB. Please provide a smaller file."),
      ParentDirNotFound => write!(f, "Parent directory not found"),
    }
  }
}
impl fmt::Display for InternalErrKind {
  #[expect(clippy::min_ident_chars)]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Overflow => write!(f, "Overflow"),
      Underflow => write!(f, "Underflow"),
      UnknownLabel => write!(f, "Unknown label"),
      InvalidInst(inst) => write!(f, "Invalid instruction: {inst}"),
      InternalIOError(err) => write!(f, "IOError: {err}"),
      TooLargeSection => write!(f, "Section too large"),
      MismatchReassignment => write!(f, "Mismatch reassignment"),
      NonExistentArg => write!(f, "Non-existent argument"),
      InvalidScope => write!(f, "Invalid scope"),
      CastError => write!(f, "Cast error"),
    }
  }
}
fn plural(num: usize) -> &'static str {
  if num == 1 { "" } else { "s" }
}
