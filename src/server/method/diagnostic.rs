use super::super::*;
use crate::prelude::*;
impl Pos<Parser> {
  pub(crate) fn diagnostic(
    &self,
    pos: Position,
    msg: &str,
    severity: u8,
    unused: bool,
  ) -> JsonNoPos {
    let index = floor_char_boundary(&self.val.text, pos.offset as usize);
    let end = floor_char_boundary(&self.val.text, pos.end() as usize);
    let start = (0..index).rfind(|i| self.val.text.as_bytes()[*i] == b'\n').map_or(0, |st| st + 1);
    let line_slice = &self.val.text.get(start..index).unwrap_or_default();
    let start_col = line_slice.encode_utf16().count();
    let mut end_line = pos.line;
    let mut end_col = start_col;
    let text = &self.val.text.get(index..end).unwrap_or_default();
    let mut parts = text.split('\n');
    if let Some(first) = parts.next() {
      end_col += first.encode_utf16().count();
    }
    for part in parts {
      end_line += 1;
      end_col = part.encode_utf16().count();
    }
    let mut key_vals = vec![
      ("message".into(), escape_err_msg(msg)),
      ("range".into(), format_range((pos.line, start_col), (end_line, end_col))),
      ("severity".into(), IntN(severity as i64)),
    ];
    if unused {
      key_vals.push(("tags".into(), ArrayN(vec![IntN(1)])));
    }
    ObjectN(key_vals)
  }
}
impl Server {
  #[expect(clippy::print_stderr)]
  pub(crate) fn publish_errs(
    &mut self,
    uri: &str,
    err: &JsonpilerErr,
    diag_map: &mut BTreeMap<String, Vec<JsonNoPos>>,
    jsonpiler: &Jsonpiler,
  ) {
    eprintln!("{}", jsonpiler.format_err(err));
    let err_str = err.to_string();
    let pos_vec = err.pos_vec();
    if pos_vec.is_empty() {
      diag_map.entry(uri.to_owned()).or_default().push(ObjectN(vec![
        ("message".into(), escape_err_msg(&err_str)),
        ("range".into(), format_range((0, 0), (0, 0))),
      ]));
      return;
    }
    for pos in pos_vec.iter().rev() {
      let diag = jsonpiler.parsers[pos.file as usize].diagnostic(*pos, &err_str, 1, false);
      let pos_uri = path2uri(&jsonpiler.parsers[pos.file as usize].val.file);
      if let Some(dep_source) = self.sources.get_mut(&pos_uri) {
        dep_source.reload.insert(uri.to_owned());
      }
      diag_map.entry(pos_uri).or_default().push(diag);
    }
    if let Some(issue) = err.issue_msg() {
      self.notify(
        "window/showMessage",
        ObjectN(vec![("message".into(), StrN(issue)), ("type".into(), IntN(1))]),
      );
    }
  }
  pub(crate) fn update_source(&mut self, uri: &str) {
    let Some(mut source) = self.get_source(uri) else {
      self.clear_diag(uri.to_owned());
      self.sources.remove(uri);
      return;
    };
    let mut jsonpiler = Jsonpiler::new(true);
    for reload_uri in take(&mut source.reload) {
      if Path::new(&uri2path(&reload_uri)).exists() {
        self.update_source(&reload_uri)
      }
    }
    let Ok(first_parser) = jsonpiler.push_parser(take(&mut source.text), uri2path(uri)) else {
      self.clear_diag(uri.to_owned());
      self.sources.remove(uri);
      return;
    };
    let parsed = first_parser.parse_jspl();
    let mut diag_map = BTreeMap::new();
    match parsed {
      Ok(json) => {
        source.parsed = Some(json.clone());
        if let Err(err) = jsonpiler.compile(json) {
          self.publish_errs(uri, &err, &mut diag_map, &jsonpiler);
        } else {
          diag_map.entry(uri.to_owned()).or_default();
        }
        source.analysis = take(&mut jsonpiler.analysis);
      }
      Err(err) => {
        source.parsed = None;
        source.analysis = None;
        self.publish_errs(uri, &err.into(), &mut diag_map, &jsonpiler)
      }
    }
    for warn in jsonpiler.parsers.iter().flat_map(|parser| &parser.val.warns) {
      let diag = jsonpiler.parsers[warn.pos.file as usize].diagnostic(
        warn.pos,
        &format!("{}", warn.val),
        2,
        matches!(warn.val, UnusedName(..)),
      );
      let pos_uri = path2uri(&jsonpiler.parsers[warn.pos.file as usize].val.file);
      diag_map.entry(pos_uri).or_default().push(diag);
    }
    for (diag_uri, diags) in diag_map {
      self.notify(
        "textDocument/publishDiagnostics",
        ObjectN(vec![
          ("uri".into(), StrN(diag_uri.clone())),
          ("diagnostics".into(), ArrayN(diags)),
        ]),
      );
    }
    let Ok(first_parser_mut) = jsonpiler.first_parser_mut() else {
      self.sources.remove(uri);
      return;
    };
    source.text = take(&mut first_parser_mut.val.text);
    self.sources.insert(uri.to_owned(), source);
  }
}
fn escape_err_msg(err_msg: &str) -> JsonNoPos {
  StrN(err_msg.replace("\n", " ").replace("  ", ""))
}
