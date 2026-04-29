mod method;
pub(crate) mod sync;
mod time_stamp;
mod utility;
use self::time_stamp::{format_micros, time_stamp};
pub(crate) use self::utility::*;
use crate::prelude::*;
use std::{
  io::{self, BufRead as _, BufReader, Read as _, Write as _},
  process::exit,
  time::Instant,
};
const MB: u64 = 1 << 20u8;
pub(crate) struct Server {
  channel: Channel,
  pending: HashMap<String, Vec<JsonNoPos>>,
  pub requests: BTreeMap<IdKind, (String, Instant)>,
  scheduler: Scheduler,
  shutdown: bool,
  pub sources: HashMap<String, Source>,
  stdin: BufReader<io::Stdin>,
  stdout: io::Stdout,
}
#[derive(Debug, Clone, Default, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub(crate) enum IdKind {
  IntI(i64),
  #[default]
  NullI,
  StrI(String),
}
impl From<IdKind> for JsonNoPos {
  fn from(id: IdKind) -> Self {
    match id {
      IdKind::NullI => NullN,
      IdKind::IntI(int) => IntN(int),
      IdKind::StrI(string) => StrN(string),
    }
  }
}
impl TryFrom<JsonNoPos> for IdKind {
  type Error = ();
  fn try_from(json: JsonNoPos) -> Result<Self, Self::Error> {
    match json {
      NullN => Ok(NullI),
      IntN(int) => Ok(IntI(int)),
      StrN(string) => Ok(StrI(string)),
      ArrayN(_) | BoolN(_) | FloatN(_) | ObjectN(_) => Err(()),
    }
  }
}
impl fmt::Display for IdKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let json_no_pos: JsonNoPos = self.clone().into();
    json_no_pos.fmt(f)
  }
}
#[derive(Debug, Clone, Default)]
pub(crate) struct Source {
  pub analysis: Option<Analysis>,
  pub parsed: Option<Pos<Json>>,
  pub reload: BTreeSet<String>,
  pub text: String,
}
impl Source {
  pub(crate) fn new(text: String) -> Self {
    Source { text, reload: BTreeSet::new(), parsed: None, analysis: None }
  }
}
impl Server {
  pub(crate) fn clear_diag(&mut self, uri: String) {
    self.notify(
      "textDocument/publishDiagnostics",
      ObjectN(vec![("uri".into(), StrN(uri)), ("diagnostics".into(), ArrayN(vec![]))]),
    )
  }
  pub(crate) fn get_source(&mut self, uri: &str) -> Option<Source> {
    let file = uri2path(uri);
    self.sources.get(uri).cloned().or_else(|| {
      if fs::metadata(&file).ok()?.len() <= MB {
        let source = Source::new(fs::read_to_string(&file).ok()?);
        self.sources.insert(uri.to_owned(), source);
        self.update_source(uri);
        self.sources.get(uri).cloned()
      } else {
        None
      }
    })
  }
  #[expect(clippy::print_stderr)]
  fn handle(&mut self, msg: String) {
    let mut jsonpiler = Jsonpiler::new(false);
    let mut json_parser = Pos::<Parser>::new(msg, 0, "server_stdin.json".into(), jsonpiler.id());
    let mut json = match json_parser.parse_json().map(|parsed| parsed.val) {
      Ok(json) => json.delete_pos(),
      Err(err) => {
        jsonpiler.parsers.push(json_parser);
        eprintln!("{}", jsonpiler.format_err(&err.into()));
        self.error(NullI, -32700, "Parse error");
        return;
      }
    };
    let params = json.take("params").unwrap_or(ObjectN(vec![]));
    let id_opt = if let Some(id) = json.take("id") {
      let Ok(id_kind) = id.try_into() else {
        self.error(NullI, -32600, "Invalid Request");
        return;
      };
      Some(id_kind)
    } else {
      None
    };
    let Some(method) = json.get("method").and_then(|method| method.as_str()) else {
      self.error(id_opt.unwrap_or(NullI), -32600, "Invalid Request");
      return;
    };
    if method == "$/setTrace" {
      return;
    }
    let start_stamp = time_stamp();
    let start_micros = Instant::now();
    if let Some(id) = &id_opt {
      self.requests.insert(id.clone(), (method.to_owned(), start_micros));
    }
    eprintln!(
      "{} [{}] --> {method} {params}",
      start_stamp,
      if let Some(id) = &id_opt { format!("request({id})") } else { "notify".into() },
    );
    if self.shutdown {
      if let Some(id) = id_opt {
        self.error(id, -32600, "Server is shutdown");
      } else {
        self.log_only_error(start_micros, "Server is shutdown");
      }
      return;
    }
    self.eval_method(method, params, id_opt.clone());
    if id_opt.is_none() {
      let stamp = time_stamp();
      eprintln!(
        "{stamp} [handled] ({}) {method}",
        format_micros(start_micros.elapsed().as_micros())
      );
    }
  }
  pub(crate) fn main(&mut self) -> ! {
    loop {
      let Some(msg) = self.read() else {
        self.log_only_error(Instant::now(), "Failed to read message");
        exit(1);
      };
      self.handle(msg);
      while let Ok(uri) = self.channel.rx.try_recv() {
        self.flush(uri);
      }
    }
  }
  pub(crate) fn new() -> Self {
    let channel = Channel::new();
    Server {
      shutdown: false,
      sources: HashMap::new(),
      pending: HashMap::new(),
      scheduler: Scheduler::new(channel.tx.clone()),
      channel,
      stdin: BufReader::new(io::stdin()),
      stdout: io::stdout(),
      requests: BTreeMap::new(),
    }
  }
  fn read(&mut self) -> Option<String> {
    let mut content_length = 0;
    loop {
      let mut line = String::new();
      if self.stdin.read_line(&mut line).ok()? == 0 {
        return None;
      }
      if line == "\r\n" || line == "\n" {
        break;
      }
      if line.to_ascii_lowercase().starts_with("content-length:") {
        content_length = line.split_once(':')?.1.trim().parse::<usize>().ok()?;
      }
    }
    if content_length == 0 {
      return None;
    }
    let mut body = vec![0u8; content_length];
    self.stdin.read_exact(&mut body).ok()?;
    String::from_utf8(body).ok()
  }
  #[expect(clippy::print_stderr)]
  fn write(&mut self, body: &str) {
    if write!(self.stdout, "Content-Length: {}\r\n\r\n{body}", body.len())
      .map(|_| self.stdout.flush())
      .is_err()
    {
      eprintln!("Failed to write response");
      exit(1);
    }
  }
}
impl Server {
  #[expect(clippy::print_stderr)]
  pub(crate) fn error(&mut self, id: IdKind, code: i64, message: &str) {
    let (method, start_micros) =
      self.requests.remove(&id).unwrap_or_else(|| (String::new(), Instant::now()));
    let stamp = time_stamp();
    eprintln!(
      "{stamp} [error({id})] <-({})- {method} {code}: {message}",
      format_micros(start_micros.elapsed().as_micros())
    );
    self.write(
      &ObjectN(vec![
        ("jsonrpc".into(), StrN("2.0".into())),
        ("id".into(), id.into()),
        (
          "error".into(),
          ObjectN(vec![("code".into(), IntN(code)), ("message".into(), StrN(message.to_owned()))]),
        ),
      ])
      .to_string(),
    )
  }
  #[expect(clippy::print_stderr)]
  pub(crate) fn log_only_error(&mut self, start_micros: Instant, message: &str) {
    let stamp = time_stamp();
    eprintln!(
      "{stamp} [error] <-({})- error: {message}",
      format_micros(start_micros.elapsed().as_micros())
    );
  }
  #[expect(clippy::print_stderr)]
  pub(crate) fn notify(&mut self, method: &str, args: JsonNoPos) {
    let stamp = time_stamp();
    eprintln!("{stamp} [notify] <-- {method} {args}");
    self.write(
      &ObjectN(vec![
        ("jsonrpc".into(), StrN("2.0".into())),
        ("method".into(), StrN(method.into())),
        ("params".into(), args),
      ])
      .to_string(),
    )
  }
  #[expect(clippy::print_stderr)]
  pub(crate) fn response(&mut self, id: IdKind, result: JsonNoPos) {
    let (method, start_micros) =
      self.requests.remove(&id).unwrap_or_else(|| (String::new(), Instant::now()));
    let stamp = time_stamp();
    eprintln!(
      "{stamp} [response({id})] <-({})- {method} {result}",
      format_micros(start_micros.elapsed().as_micros())
    );
    self.write(
      &ObjectN(vec![
        ("jsonrpc".into(), StrN("2.0".into())),
        ("id".into(), id.into()),
        ("result".into(), result),
      ])
      .to_string(),
    )
  }
}
