mod method;
pub(crate) mod sync;
mod time_stamp;
mod utility;
use self::time_stamp::{format_micros, time_stamp};
pub(crate) use self::utility::*;
use crate::prelude::*;
use std::{
  collections::hash_map::Entry,
  io::{BufRead as _, BufReader, Read as _, Write as _},
  process::exit,
  time::Instant,
};
const MB: u64 = 1 << 20u8;
pub(crate) struct Server {
  channel: Channel,
  pub docs: Option<HashMap<String, String>>,
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
  pub pending: Vec<JsonNoPos>,
  pub reload: BTreeSet<String>,
  pub text: String,
}
impl Source {
  pub(crate) fn new(text: String) -> Self {
    Source { text, reload: BTreeSet::new(), parsed: None, analysis: None, pending: vec![] }
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
  pub(crate) fn get_source_mut(&mut self, uri: &str) -> Option<&mut Source> {
    let file = uri2path(uri);
    if let Entry::Vacant(entry) = self.sources.entry(uri.to_owned()) {
      if fs::metadata(&file).ok()?.len() > MB {
        return None;
      }
      entry.insert(Source::new(fs::read_to_string(&file).ok()?));
      self.update_source(uri);
    }
    self.sources.get_mut(uri)
  }
  #[expect(clippy::print_stderr)]
  fn handle(&mut self, msg: String) {
    let mut jsonpiler = Jsonpiler::new(false);
    let mut json = match (|| -> ErrOR<Pos<Json>> {
      jsonpiler.push_parser(msg, "server_stdin.json".into())?.parse_json().map_err(Into::into)
    })() {
      Ok(json) => json.val.delete_pos(),
      Err(err) => {
        eprintln!("{}", jsonpiler.format_err(&err));
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
    let Some(method) = (|| json.get("method")?.as_str())() else {
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
        continue;
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
      scheduler: Scheduler::new(channel.tx.clone()),
      channel,
      stdin: BufReader::new(io::stdin()),
      stdout: io::stdout(),
      docs: None,
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
pub(crate) fn build_doc_cache() -> HashMap<String, String> {
  let mut map = HashMap::new();
  let Some(exe_dir) =
    env::current_exe().ok().and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
  else {
    return map;
  };
  let dir = exe_dir.join("../docs/functions");
  let Ok(entries) = fs::read_dir(&dir) else {
    return map;
  };
  for entry in entries.flatten() {
    let path = entry.path();
    if path.file_name().and_then(|os_str| os_str.to_str()) == Some("README.md") {
      continue;
    }
    if path.extension().and_then(|os_str| os_str.to_str()) != Some("md") {
      continue;
    }
    let Ok(content) = fs::read_to_string(&path) else {
      continue;
    };
    parse_markdown_into(&content, &mut map);
  }
  map
}
#[expect(clippy::else_if_without_else)]
fn parse_markdown_into(content: &str, map: &mut HashMap<String, String>) {
  let mut current_name: Option<String> = None;
  let mut current_body = String::new();
  for line in content.lines() {
    if let Some(name) = line.strip_prefix("## ") {
      if let Some(prev) = current_name.take() {
        map.insert(prev, unescape_hash(current_body.trim()));
        current_body.clear();
      }
      current_name = Some(name.trim().to_owned());
    } else if current_name.is_some() {
      current_body.push_str(line);
      current_body.push('\n');
    }
  }
  if let Some(last) = current_name {
    map.insert(last, unescape_hash(current_body.trim()));
  }
}
fn unescape_hash(string: &str) -> String {
  let mut out = String::with_capacity(string.len());
  let mut chars = string.chars();
  while let Some(char) = chars.next() {
    if char == '\\' {
      if let Some(next) = chars.next() {
        out.push(next);
      }
    } else {
      out.push(char);
    }
  }
  out
}
