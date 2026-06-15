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
    let start_col = count_utf16(line_slice);
    let mut end_line = pos.line;
    let mut end_col = start_col;
    let text = &self.val.text.get(index..end).unwrap_or_default();
    let mut parts = text.split('\n');
    if let Some(first) = parts.next() {
      end_col += count_utf16(first);
    }
    for part in parts {
      end_line += 1;
      end_col = count_utf16(part);
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
  pub(crate) fn publish_errs(
    &mut self,
    uri: &str,
    err: &JsonpilerErr,
    diag_map: &mut BTreeMap<String, Vec<JsonNoPos>>,
    jsonpiler: &Jsonpiler,
  ) {
    let log =
      LogMsg::new(MsgType::Warning, format!("Error in {uri}"), Some(jsonpiler.format_err(err)));
    self.log(log);
    let err_str = err.kind.to_string();
    if err.refs.is_empty() {
      diag_map.entry(uri.into()).or_default().push(ObjectN(vec![
        ("message".into(), escape_err_msg(&err_str)),
        ("range".into(), format_range((0, 0), (0, 0))),
      ]));
      return;
    }
    for pos in err.refs.iter().rev() {
      let diag = jsonpiler.files[pos.file].parser.diagnostic(*pos, &err_str, 1, false);
      let pos_uri = path2uri(&jsonpiler.files[pos.file].path);
      if let Some(dep_source) = self.sources.get_mut(&pos_uri) {
        dep_source.reload.insert(uri.into());
      }
      diag_map.entry(pos_uri).or_default().push(diag);
    }
    if let Some(issue) = err.kind.issue_msg() {
      self.n_show_message(MsgType::Error, issue);
    }
  }
  pub(crate) fn update_source(&mut self, uri: &str) {
    let Some(mut source) = self.get_source(uri) else {
      self.close_uri(uri);
      return;
    };
    let mut jsonpiler = Jsonpiler::new(true);
    for reload_uri in take(&mut source.reload) {
      if Path::new(&uri2path(&reload_uri)).exists() {
        self.update_source(&reload_uri)
      }
    }
    let Ok(first_file) = jsonpiler.push_file(take(&mut source.text), uri2path(uri)) else {
      self.close_uri(uri);
      return;
    };
    let parsed = first_file.parse_jspl();
    let mut diag_map = BTreeMap::new();
    match parsed {
      Ok(json) => {
        source.parsed = Some(json.clone());
        if let Err(err) = jsonpiler.compile(json) {
          self.publish_errs(uri, &err, &mut diag_map, &jsonpiler);
        } else {
          diag_map.entry(uri.into()).or_default();
        }
        source.analysis = take(&mut jsonpiler.analysis);
      }
      Err(err) => {
        source.parsed = None;
        source.analysis = None;
        self.publish_errs(uri, &err.into(), &mut diag_map, &jsonpiler)
      }
    }
    for warn in jsonpiler.files.iter().flat_map(|file| &file.parser.val.warns) {
      let diag = jsonpiler.files[warn.pos.file].parser.diagnostic(
        warn.pos,
        &warn.val.to_string(),
        2,
        matches!(warn.val, UnusedName(..)),
      );
      let pos_uri = path2uri(&jsonpiler.files[warn.pos.file].path);
      diag_map.entry(pos_uri).or_default().push(diag);
    }
    if !diag_map.contains_key(uri) {
      self.n_publish_diagnostics(uri, vec![]);
    }
    for (diag_uri, diags) in diag_map {
      self.n_publish_diagnostics(&diag_uri, diags);
    }
    let Ok(first_file_mut) = jsonpiler.first_file_mut() else {
      self.close_uri(uri);
      return;
    };
    source.text = take(&mut first_file_mut.parser.val.text);
    self.sources.insert(uri.into(), source);
  }
}
fn escape_err_msg(err_msg: &str) -> JsonNoPos {
  StrN(err_msg.replace("\n", " "))
}
fn count_utf16(str: &str) -> u32 {
  str.encode_utf16().count() as u32
}
