mod build_doc;
mod method;
mod rpc_msg;
pub(crate) mod sync;
mod time_stamp;
mod utility;
use self::time_stamp::{format_micros, time_stamp};
pub(crate) use self::utility::*;
use crate::prelude::*;
use std::{
  io::{BufRead as _, BufReader, Read as _, Write as _},
  process::exit,
  time::Instant,
};
const MB: u64 = 1 << 20u8;
pub(crate) struct Server {
  channel: Channel,
  pub docs: HashMap<String, String>,
  pub requests: BTreeMap<IdKind, (String, Instant)>,
  scheduler: Scheduler,
  shutdown: bool,
  pub sources: HashMap<String, Source>,
  stdin: BufReader<io::Stdin>,
  stdout: io::Stdout,
  trace: Trace,
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
  pub(crate) fn get_source(&mut self, uri: &str) -> Option<Source> {
    if !self.sources.contains_key(uri) {
      self.read_source(uri)?;
    }
    self.sources.get(uri).cloned()
  }
  pub(crate) fn get_source_mut(&mut self, uri: &str) -> Option<&mut Source> {
    if !self.sources.contains_key(uri) {
      self.read_source(uri)?;
    }
    self.sources.get_mut(uri)
  }
  pub(crate) fn read_source(&mut self, uri: &str) -> Option<()> {
    let file = uri2path(uri);
    if fs::metadata(&file).ok()?.len() >= MB {
      let msg = format!("File is too large: {file}\nThis file is ignored by LSP");
      self.n_show_message(MsgType::Warning, msg);
      return None;
    }
    let source = Source::new(fs::read_to_string(&file).ok()?);
    self.sources.insert(uri.into(), source);
    self.update_source(uri);
    Some(())
  }
}
impl Server {
  fn handle(&mut self, msg: String) {
    let mut jsonpiler = Jsonpiler::new(false);
    let mut json = match (|| -> ErrOR<Pos<Json>> {
      jsonpiler.push_file(msg, "server_stdin.json".into())?.parse_json().map_err(Into::into)
    })() {
      Ok(json) => json.val.delete_pos(),
      Err(err) => {
        let log =
          LogMsg::new(MsgType::Error, "Parse error".into(), Some(jsonpiler.format_err(&err)));
        self.error(IdKind::Null, -32700, log);
        return;
      }
    };
    let params = json.take("params").unwrap_or(ObjectN(vec![]));
    let id_opt = if let Some(id) = json.take("id") {
      let Ok(id_kind) = id.try_into() else {
        let log = LogMsg::new(MsgType::Error, "Invalid Request".into(), None);
        self.error(IdKind::Null, -32600, log);
        return;
      };
      Some(id_kind)
    } else {
      None
    };
    let Some(method) = json.get_str("method") else {
      let log = LogMsg::new(MsgType::Error, "Invalid Request".into(), None);
      self.error(id_opt.unwrap_or(IdKind::Null), -32600, log);
      return;
    };
    let start_stamp = time_stamp();
    let start = Instant::now();
    if let Some(id) = &id_opt {
      self.requests.insert(id.clone(), (method.into(), start));
    }
    let msg_recv = format!(
      "{start_stamp} [{}] --> {method}",
      if let Some(id) = &id_opt { format!("request({id})") } else { "notify".into() },
    );
    self.log(LogMsg::new(MsgType::Info, msg_recv, Some(params.to_string())));
    self.eval_method(method, params, id_opt.clone(), start);
    if id_opt.is_none() {
      let stamp = time_stamp();
      let msg_handled = format!("{stamp} [handled] {method} in {}", format_micros(start));
      self.log(LogMsg::new(MsgType::Info, msg_handled, None));
    }
  }
  pub(crate) fn main(&mut self) -> ! {
    loop {
      let Some(msg) = self.read() else {
        self.log_only_error(
          Instant::now(),
          LogMsg::new(MsgType::Error, "Failed to read message".into(), None),
        );
        exit(1);
      };
      self.handle(msg);
      while let Ok(uri) = self.channel.rx.try_recv() {
        self.flush(&uri);
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
      trace: Trace::default(),
      stdin: BufReader::new(io::stdin()),
      stdout: io::stdout(),
      docs: HashMap::new(),
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
      if let Some((key, length)) = line.split_once(':')
        && key.eq_ignore_ascii_case("content-length")
      {
        content_length = length.trim().parse::<usize>().ok()?;
      }
    }
    let mut body = vec![0u8; content_length];
    self.stdin.read_exact(&mut body).ok()?;
    String::from_utf8(body).ok()
  }
  fn write(&mut self, body: &str) {
    if write!(self.stdout, "Content-Length: {}\r\n\r\n{body}", body.len()).is_err()
      || self.stdout.flush().is_err()
    {
      self.log(LogMsg::new(MsgType::Error, "Failed to write response".into(), None));
      exit(1);
    }
  }
}
