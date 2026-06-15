use super::super::*;
use crate::prelude::*;
use std::time::Duration;
impl Server {
  pub(crate) fn close_uri(&mut self, uri: &str) {
    self.scheduler.cancel(uri);
    self.sources.remove(uri);
    self.n_publish_diagnostics(uri, vec![]);
  }
  pub(crate) fn flush(&mut self, uri: &str) {
    self.scheduler.cancel(uri);
    let Some(source) = self.sources.get_mut(uri) else {
      return;
    };
    for mut change in take(&mut source.pending) {
      let Some(text) = change.take_str("text") else {
        continue;
      };
      let Some(range) = change.get("range") else {
        *source = Source::new(text);
        continue;
      };
      let Some(start) = range.get("start").and_then(|start| range2offset(&source.text, start))
      else {
        continue;
      };
      let Some(end) = range.get("end").and_then(|end| range2offset(&source.text, end)) else {
        continue;
      };
      if start > end {
        continue;
      }
      source.text.replace_range(start..end, &text);
    }
    self.update_source(uri)
  }
  pub(crate) fn m_did_change(&mut self, mut params: JsonNoPos) {
    let Some(uri) = (|| params.take("textDocument")?.take_str("uri"))() else {
      return;
    };
    let Some(content_changes) = params.take("contentChanges") else {
      return;
    };
    let Some(source) = self.get_source_mut(&uri) else {
      return;
    };
    if let ArrayN(vec) = content_changes {
      source.pending.extend_from_slice(&vec);
    }
    self.scheduler.cancel(&uri);
    self.scheduler.schedule(uri, Duration::from_millis(100));
  }
  pub(crate) fn m_did_close(&mut self, mut params: JsonNoPos) {
    let Some(uri) = (|| params.take("textDocument")?.take_str("uri"))() else {
      return;
    };
    self.scheduler.cancel(&uri);
    self.sources.remove(&uri);
  }
  pub(crate) fn m_did_open(&mut self, mut params: JsonNoPos) {
    if (|| {
      let mut document = params.take("textDocument")?;
      let uri = document.take_str("uri")?;
      let text = document.take_str("text")?;
      self.sources.insert(uri.clone(), Source::new(text));
      self.update_source(&uri);
      Some(())
    })()
    .is_none()
    {
      let log = LogMsg::new(MsgType::Error, "Invalid params".into(), None);
      self.log_only_error(Instant::now(), log);
    }
  }
  pub(crate) fn m_folding_range(&mut self, mut params: JsonNoPos, id: IdKind) {
    let Some(uri) = (|| params.take("textDocument")?.take_str("uri"))() else {
      let log = LogMsg::new(MsgType::Error, "Invalid params".into(), None);
      self.error(id, -32602, log);
      return;
    };
    self.flush(&uri);
    let path = uri2path(&uri);
    let folding_ranges = self
      .get_source(&uri)
      .and_then(|source| {
        let parsed = match source.parsed {
          Some(json) => json,
          None => {
            let mut parser = <Pos<Parser>>::new(source.text, 0);
            match parser.parse_jspl(&path) {
              Ok(json) => json,
              Err(err) => {
                let log = LogMsg::new(
                  MsgType::Warning,
                  format!("Error in {uri}"),
                  Some(parser.format_err(&err.into(), &path)),
                );
                self.log(log);
                return None;
              }
            }
          }
        };
        let mut ranges = vec![];
        match &parsed.val {
          Object(Lit(obj)) => {
            for value in obj.iter().map(|(_, value)| value) {
              folding_range(value, &mut ranges);
            }
          }
          Array(_, Lit(vec)) => {
            for value in vec {
              folding_range(value, &mut ranges);
            }
          }
          Array(..) | Bool(_) | Float(_) | Int(_) | Null(_) | Object(_) | Str(_) => {}
        }
        Some(ranges)
      })
      .unwrap_or_default();
    self.response(
      id,
      ArrayN(
        folding_ranges
          .into_iter()
          .filter_map(|pos| pos2folding_range(pos, &self.sources[&uri].text))
          .collect(),
      ),
    );
  }
  pub(crate) fn m_formatting(&mut self, mut params: JsonNoPos, id: IdKind) {
    let Some(uri) = (|| params.take("textDocument")?.take_str("uri"))() else {
      let log = LogMsg::new(MsgType::Error, "Invalid params".into(), None);
      self.error(id, -32602, log);
      return;
    };
    self.flush(&uri);
    let file = uri2path(&uri);
    let text_edit = self
      .get_source(&uri)
      .and_then(|source| <Pos<Parser>>::new(source.text, 0).format(&file))
      .map(|text| {
        vec![ObjectN(vec![
          ("range".into(), format_range((0, 0), (u32::MAX, 0))),
          ("newText".into(), StrN(text)),
        ])]
      })
      .unwrap_or_default();
    self.response(id, ArrayN(text_edit));
  }
}
fn folding_range(json: &Pos<Json>, ranges: &mut Vec<Position>) {
  let children = json.val.children();
  if !children.is_empty() {
    ranges.push(json.pos);
  }
  for child in children {
    folding_range(child, ranges);
  }
}
fn pos2folding_range(pos: Position, text: &str) -> Option<JsonNoPos> {
  let (start_line, start_col) = offset2range(text, pos.offset);
  let (end_line, end_col) = offset2range(text, pos.end());
  if start_line >= end_line {
    return None;
  }
  Some(ObjectN(vec![
    ("startLine".into(), IntN(start_line as i64)),
    ("startCharacter".into(), IntN(start_col as i64)),
    ("endLine".into(), IntN(end_line as i64)),
    ("endCharacter".into(), IntN(end_col as i64)),
  ]))
}
