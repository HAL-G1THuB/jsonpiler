use super::super::*;
use crate::prelude::*;
use std::time::Duration;
impl Server {
  pub(crate) fn flush(&mut self, uri: String) {
    self.scheduler.cancel(&uri);
    let Some(source) = self.sources.get_mut(&uri) else {
      return;
    };
    for mut change in take(&mut source.pending) {
      let Some(text) = change.take("text").and_then(JsonNoPos::into_str) else {
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
    self.update_source(&uri)
  }
  pub(crate) fn m_did_change(&mut self, mut params: JsonNoPos) {
    let Some(uri) = (|| params.take("textDocument")?.take("uri")?.into_str())() else {
      return;
    };
    let Some(content_changes) = params.take("contentChanges") else {
      return;
    };
    let Some(source) = self.get_source_mut(&uri) else {
      return;
    };
    if let ArrayN(vec) = content_changes {
      source.pending.extend(vec);
    }
    self.scheduler.cancel(&uri);
    self.scheduler.schedule(uri, Duration::from_millis(100));
  }
  pub(crate) fn m_did_close(&mut self, mut params: JsonNoPos) {
    let Some(uri) = (|| params.take("textDocument")?.take("uri")?.into_str())() else {
      return;
    };
    self.scheduler.cancel(&uri);
    self.sources.remove(&uri);
    self.clear_diag(uri);
  }
  pub(crate) fn m_did_open(&mut self, mut params: JsonNoPos) {
    let Some(mut document) = params.take("textDocument") else {
      return;
    };
    let Some(uri) = document.take("uri").and_then(JsonNoPos::into_str) else {
      return;
    };
    let Some(text) = document.take("text").and_then(JsonNoPos::into_str) else {
      return;
    };
    self.sources.insert(uri.clone(), Source::new(text));
    self.update_source(&uri)
  }
  pub(crate) fn m_formatting(&mut self, mut params: JsonNoPos, id: IdKind) {
    let Some(uri) = (|| params.take("textDocument")?.take("uri")?.into_str())() else {
      self.error(id, -32602, "Invalid params");
      return;
    };
    self.flush(uri.clone());
    let file = uri2path(&uri);
    let text_edit = self
      .get_source(&uri)
      .and_then(|source| <Pos<Parser>>::new(source.text, 0, file, 0).format())
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
