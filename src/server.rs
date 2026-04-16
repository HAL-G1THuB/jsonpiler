use crate::prelude::*;
use std::io::{self, BufRead as _, BufReader, Read as _, Write as _};
const MB: u64 = 1 << 20u8;
const INITIALIZE: &str = r#"
"result":{
  "capabilities":{
    "textDocumentSync":2,
    "documentFormattingProvider": true,
    "completionProvider":{ "triggerCharacters":[":"] },
    "definitionProvider":true
  }
}"#;
const TYPE_ANNOTATIONS: [&str; 5] = [
  r#"{"label":"Int","kind":7,"insertText":" Int; "}"#,
  r#"{"label":"Float","kind":7,"insertText":" Float; "}"#,
  r#"{"label":"Bool","kind":7,"insertText":" Bool; "}"#,
  r#"{"label":"Str","kind":7,"insertText":" Str; "}"#,
  r#"{"label":"Null","kind":7,"insertText":" Null; "}"#,
];
#[derive(Debug, Clone)]
pub(crate) struct Server {
  pub sources: HashMap<String, (String, BTreeSet<String>)>,
}
impl Server {
  pub(crate) fn get_source(&self, uri: &str) -> Option<(String, BTreeSet<String>)> {
    let file = uri_to_path(uri);
    self.sources.get(uri).cloned().or_else(|| {
      (fs::metadata(&file).ok()?.len() <= MB)
        .then_some((String::from_utf8(fs::read(&file).ok()?).ok()?, BTreeSet::new()))
    })
  }
  #[expect(clippy::print_stderr)]
  fn handle(&mut self, msg: String) -> Option<()> {
    let mut jsonpiler = Jsonpiler::new();
    let mut json_parser =
      Pos::<Parser>::new(msg.into_bytes(), 0, "server_stdin.json".into(), jsonpiler.id());
    let json = match json_parser.parse_json().map(|parsed| parsed.val) {
      Ok(json) => json,
      Err(err) => {
        jsonpiler.parsers.push(json_parser);
        eprintln!("{}", jsonpiler.format_err(&err.into()));
        return None;
      }
    };
    let method = json.get("method")?.as_str()?;
    eprintln!("{method}");
    match method {
      "initialize" => self.m_initialize(json)?,
      "textDocument/formatting" => self.m_formatting(json)?,
      "textDocument/didChange" => self.m_did_change(json)?,
      "textDocument/didOpen" => self.m_did_open(json)?,
      "textDocument/completion" => self.m_completion(json)?,
      "initialized" => (),
      _ => eprintln!("unknown method"),
    }
    eprintln!("success");
    Some(())
  }
  #[expect(clippy::needless_pass_by_value, clippy::unused_self)]
  pub(crate) fn m_completion(&mut self, json: Json) -> Option<()> {
    let context = json.get("context")?;
    let trigger_kind = context.get_int("triggerKind")?;
    let items = if trigger_kind == 2 {
      match context.get("triggerCharacter")?.as_str()? {
        ":" => TYPE_ANNOTATIONS.to_vec(),
        _ => vec![],
      }
    } else {
      vec![]
    };
    json.response(&format!(r#""result":[{}]"#, items.join(",")))
  }
  pub(crate) fn m_did_change(&mut self, mut json: Json) -> Option<()> {
    let mut params = json.take("params")?;
    let mut t_d = params.take("textDocument")?;
    let uri = t_d.take("uri")?.into_str()?;
    let Array(Lit(changes)) = params.take("contentChanges")? else { return None };
    for mut change in changes {
      let text = change.val.take("text")?.into_str()?;
      let range = change.val.get("range")?;
      if !matches!(range, Object(Lit(_))) {
        let source = (text, BTreeSet::new());
        Jsonpiler::new().publish_diagnostics(&uri, &source, self);
        self.sources.insert(uri, source);
        return Some(());
      }
      let source = self.sources.get_mut(&uri)?;
      let start = range_offset(&source.0, range.get("start")?)?;
      let end = range_offset(&source.0, range.get("end")?)?;
      source.0.replace_range(start..end, &text);
      Jsonpiler::new().publish_diagnostics(&uri, &source.clone(), self);
    }
    Some(())
  }
  pub(crate) fn m_did_open(&mut self, mut json: Json) -> Option<()> {
    let mut params = json.take("params")?;
    let mut t_d = params.take("textDocument")?;
    let uri = t_d.take("uri")?.into_str()?;
    let source = (t_d.take("text")?.into_str()?, BTreeSet::new());
    Jsonpiler::new().publish_diagnostics(&uri, &source, self);
    self.sources.insert(uri, source);
    Some(())
  }
  #[expect(clippy::needless_pass_by_value)]
  pub(crate) fn m_formatting(&mut self, json: Json) -> Option<()> {
    let params = json.get("params")?;
    let t_d = params.get("textDocument")?;
    let uri = t_d.get("uri")?.as_str()?;
    let source = self.get_source(uri).or_else(|| {
      clear_diag(uri);
      None
    })?;
    let file = uri_to_path(uri);
    let mut parser = <Pos<Parser>>::new(source.0.into_bytes(), 0, file, 0);
    let text = match parser.format() {
      Some(text) => {
        format!(
          r#"{{{},"newText":"{}"}}"#,
          format_range((0, 0), (u32::MAX, 0)),
          escape_non_ctrl(&text)
        )
      }
      None => String::new(),
    };
    json.response(&format!(r#""result":[{text}]"#))
  }
  #[expect(clippy::needless_pass_by_value, clippy::unused_self)]
  pub(crate) fn m_initialize(&mut self, json: Json) -> Option<()> {
    json.response(INITIALIZE)
  }
  #[expect(clippy::print_stderr)]
  pub(crate) fn main(&mut self) {
    eprintln!("SERVER STARTED");
    while read().map(|msg| self.handle(msg)).is_some() {}
    eprintln!("SERVER TERMINATED");
  }
  pub(crate) fn new() -> Self {
    Server { sources: HashMap::new() }
  }
}
impl Pos<Parser> {
  pub(crate) fn diagnostic(
    &self,
    pos: Position,
    msg: &str,
    severity: u8,
    unused: bool,
  ) -> Option<String> {
    let len = self.val.source.len();
    let index = (pos.offset as usize).min(len.saturating_sub(1));
    let end_index = (index + pos.size as usize).min(len);
    let start = (0..index).rfind(|i| self.val.source[*i] == b'\n').map_or(0, |st| st + 1);
    let line_slice = str::from_utf8(&self.val.source[start..index]).ok()?;
    let start_col = line_slice.encode_utf16().count();
    let mut end_line = pos.line - 1;
    let mut end_col = start_col;
    let slice = &self.val.source[index..end_index];
    let text = str::from_utf8(slice).ok()?;
    let mut parts = text.split('\n');
    if let Some(first) = parts.next() {
      end_col += first.encode_utf16().count();
    }
    for part in parts {
      end_line += 1;
      end_col = part.encode_utf16().count();
    }
    let json_str = format!(
      r#"{{{},"message":"{}","severity":{severity}{}}}"#,
      format_range((pos.line - 1, start_col), (end_line, end_col)),
      escape_err_msg(msg),
      if unused { r#","tags":[1]"# } else { "" }
    );
    Some(json_str)
  }
}
impl Jsonpiler {
  fn publish_diagnostics(
    &mut self,
    uri: &str,
    (text, reload): &(String, BTreeSet<String>),
    server: &mut Server,
  ) -> Option<()> {
    for reload_uri in reload {
      clear_diag(reload_uri);
      if let Some(source) = server.get_source(reload_uri) {
        Jsonpiler::new().publish_diagnostics(reload_uri, &source, server);
      }
    }
    let file = uri_to_path(uri);
    let mut first_parser = <Pos<Parser>>::new(text.clone().into_bytes(), 0, file, self.id());
    let parsed = first_parser.parse_jspl();
    self.parsers.push(first_parser);
    let mut diag_map = BTreeMap::new();
    match parsed {
      Ok(json) => {
        if let Err(err) = self.compile(json) {
          self.publish_errs(uri, &err, &mut diag_map, server);
        } else {
          diag_map.entry(uri.to_owned()).or_default();
        }
      }
      Err(err) => self.publish_errs(uri, &err.into(), &mut diag_map, server)?,
    }
    for warn in self.parsers.iter().flat_map(|parser| &parser.val.warns) {
      let diag = self.parsers[warn.pos.file as usize]
        .diagnostic(warn.pos, &format!("{}", warn.val), 2, matches!(warn.val, UnusedName(..)))
        .unwrap_or_default();
      let pos_uri = path_to_uri(&self.parsers[warn.pos.file as usize].val.file);
      diag_map.entry(pos_uri).or_default().push(diag);
    }
    for (diag_uri, diags) in diag_map {
      notify(
        "textDocument/publishDiagnostics",
        &format!(r#""uri":"{diag_uri}","diagnostics":[{}]"#, diags.join(",")),
      );
    }
    Some(())
  }
  #[expect(clippy::print_stderr)]
  fn publish_errs(
    &self,
    uri: &str,
    err: &JsonpilerErr,
    diag_map: &mut BTreeMap<String, Vec<String>>,
    server: &mut Server,
  ) -> Option<()> {
    eprintln!("{}", self.format_err(err));
    let err_str = err.to_string();
    let pos_vec = err.pos_vec();
    if pos_vec.is_empty() {
      diag_map.entry(uri.to_owned()).or_default().push(format!(
        r#"{{{},"message":"{}"}}"#,
        format_range((0, 0), (0, 0)),
        escape_err_msg(&err_str)
      ));
      return Some(());
    }
    for pos in pos_vec.iter().rev() {
      let diag =
        self.parsers[pos.file as usize].diagnostic(*pos, &err_str, 1, false).unwrap_or_default();
      let pos_uri = path_to_uri(&self.parsers[pos.file as usize].val.file);
      server.sources.entry(pos_uri.clone()).or_default().1.insert(uri.to_owned());
      diag_map.entry(pos_uri).or_default().push(diag);
    }
    if let Some(issue) = err.issue_msg() {
      notify("window/showMessage", &format!(r#""type":1,"message":"{}""#, escape_err_msg(&issue)))
    } else {
      Some(())
    }
  }
}
impl Json {
  pub(crate) fn response(&self, args: &str) -> Option<()> {
    let id = self.get_int("id")?;
    write(&format!(r#"{{"jsonrpc":"2.0","id":{id},{args}}}"#))
  }
}
pub(crate) fn notify(method: &str, args: &str) -> Option<()> {
  write(&format!(r#"{{"jsonrpc":"2.0","method":"{method}","params":{{{args}}}}}"#))
}
fn read() -> Option<String> {
  let mut stdin = BufReader::new(io::stdin());
  let mut content_length = 0;
  loop {
    let mut line = String::new();
    if stdin.read_line(&mut line).ok()? == 0 {
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
  stdin.read_exact(&mut body).ok()?;
  String::from_utf8(body).ok()
}
#[must_use]
fn write(body: &str) -> Option<()> {
  let mut stdout = io::stdout();
  write!(stdout, "Content-Length: {}\r\n\r\n{body}", body.len()).ok()?;
  stdout.flush().ok()?;
  Some(())
}
fn uri_to_path(uri: &str) -> String {
  let mut file = uri
    .strip_prefix("file:///")
    .unwrap_or(uri)
    .replace("%3A", ":")
    .replace("%20", " ")
    .replace('/', "\\");
  if file.len() >= 2 && file.as_bytes()[1] == b':' {
    let mut chars = file.chars();
    if let Some(first) = chars.next() {
      let rest = chars.as_str().to_owned();
      file = first.to_uppercase().collect::<String>();
      file.push_str(&rest);
    }
  }
  file
}
fn escape_non_ctrl(text: &str) -> String {
  text
    .replace('\\', "\\\\")
    .replace('"', "\\\"")
    .replace('\n', "\\n")
    .replace('\r', "\\r")
    .replace('\t', "\\t")
}
fn percent_encode(input: &str) -> String {
  let mut out = String::with_capacity(input.len() + 16);
  for byte in input.as_bytes() {
    match byte {
      b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' => out.push(*byte as char),
      _ if b"-_.~/:".contains(byte) => out.push(*byte as char),
      _ => out.push_str(&format!("%{byte:02X}")),
    }
  }
  out
}
fn path_to_uri(path: &str) -> String {
  let mut string = path.replace('\\', "/");
  if string.as_bytes().get(1) == Some(&b':')
    && let Some(first) = string.get_mut(0..1)
  {
    first.make_ascii_lowercase();
  }
  format!("file:///{}", percent_encode(&string))
}
fn escape_err_msg(err_msg: &str) -> String {
  err_msg.replace('\\', r"\\").replace('"', "\\\"").replace('\n', " ").replace('\r', "")
}
#[expect(clippy::cast_possible_truncation)]
fn range_offset(text: &str, range: &Json) -> Option<usize> {
  let line = range.get_int("line")?.cast_unsigned() as usize;
  let character = range.get_int("character")?.cast_unsigned() as usize;
  let mut line_start = 0;
  let mut line_str = "";
  for (i, raw_line) in text.split_inclusive('\n').enumerate() {
    if i == line {
      line_str = raw_line.trim_end_matches(['\n', '\r']);
      break;
    }
    line_start += raw_line.len();
  }
  let mut utf16_count = 0;
  for (idx, ch) in line_str.char_indices() {
    utf16_count += ch.len_utf16();
    if utf16_count > character {
      return Some(line_start + idx);
    }
  }
  Some(line_start + line_str.len())
}
fn clear_diag(uri: &str) -> Option<()> {
  notify("textDocument/publishDiagnostics", &format!(r#""uri":"{uri}","diagnostics":[]"#))
}
fn format_range((s_line, s_char): (u32, usize), (e_line, e_char): (u32, usize)) -> String {
  format!(
    r#"
"range":{{
  "start":{{"line":{s_line},"character":{s_char}}},
  "end":{{"line":{e_line},"character":{e_char}}}
}}"#
  )
}
