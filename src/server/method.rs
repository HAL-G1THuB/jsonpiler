mod diagnostic;
pub(crate) mod notification;
mod symbol;
mod text_document;
use super::utility::*;
use crate::prelude::*;
use std::{process::exit, time::Instant};
impl Server {
  pub(crate) fn eval_method(
    &mut self,
    method: &str,
    params: JsonNoPos,
    id_opt: Option<IdKind>,
    start: Instant,
  ) {
    macro_rules! id {
      ($id_opt:expr) => {{
        let Some(id) = $id_opt else {
          let log = LogMsg::new(MsgType::Error, "Missing id".into(), None);
          self.log_only_error(start, log);
          return;
        };
        id
      }};
    }
    if self.shutdown {
      if method == "exit" {
        exit(0);
      }
      let log = LogMsg::new(MsgType::Error, "Server is shutdown".into(), None);
      if let Some(id) = id_opt {
        self.error(id, -32600, log);
      } else {
        self.log_only_error(start, log);
      }
      return;
    }
    match method {
      "initialize" => self.m_initialize(params, id!(id_opt)),
      "initialized" => (),
      "textDocument/didOpen" => self.m_did_open(params),
      "textDocument/didChange" => self.m_did_change(params),
      "textDocument/didSave" => (),
      "textDocument/completion" => self.m_completion(params, id!(id_opt)),
      "textDocument/definition" => self.m_definition(params, id!(id_opt)),
      "textDocument/references" => self.m_references(params, id!(id_opt)),
      "textDocument/rename" => self.m_rename(params, id!(id_opt)),
      "textDocument/prepareRename" => self.m_prepare_rename(params, id!(id_opt)),
      "textDocument/formatting" => self.m_formatting(params, id!(id_opt)),
      "textDocument/hover" => self.m_hover(params, id!(id_opt)),
      "textDocument/didClose" => self.m_did_close(params),
      "textDocument/foldingRange" => self.m_folding_range(params, id!(id_opt)),
      "shutdown" => self.m_shutdown(id!(id_opt)),
      "exit" => exit(1),
      "$/setTrace" => self.trace = params.get_str("trace").map(Into::into).unwrap_or(Trace::Off),
      _ => {
        let log = LogMsg::new(MsgType::Error, "Method not found".into(), Some(method.into()));
        if let Some(id) = id_opt {
          self.error(id, -32601, log);
        } else {
          self.log_only_error(start, log);
        }
      }
    }
  }
  pub(crate) fn m_completion(&mut self, mut params: JsonNoPos, id: IdKind) {
    let Some((_uri, _position)) =
      (|| Some((params.take("textDocument")?.take_str("uri")?, params.take("position")?)))()
    else {
      let log = LogMsg::new(MsgType::Error, "Invalid params".into(), None);
      self.error(id, -32602, log);
      return;
    };
    let (trigger_kind, trigger_character) = params
      .get("context")
      .and_then(|context| {
        context.get_int("triggerKind").map(|t_kind| (t_kind, context.get_str("triggerCharacter")))
      })
      .unwrap_or((1, None));
    let items = ArrayN(if trigger_kind == 2 {
      match trigger_character {
        Some(":") => ["Int", "Float", "Bool", "Str", "Null"]
          .into_iter()
          .map(|ty| {
            ObjectN(vec![
              ("label".into(), StrN(ty.into())),
              ("kind".into(), IntN(7)),
              ("insertText".into(), StrN(format!(" {}; ", ty))),
            ])
          })
          .collect(),
        _ => vec![],
      }
    } else {
      vec![]
    });
    self.response(id, items);
  }
  pub(crate) fn m_initialize(&mut self, params: JsonNoPos, id: IdKind) {
    use super::build_doc::build_doc;
    self.trace = params.get_str("trace").map(Into::into).unwrap_or(Trace::Off);
    self.docs = build_doc();
    let mut capabilities = vec![("textDocumentSync".into(), IntN(2))];
    let providers = [
      ("completion", ObjectN(vec![("triggerCharacters".into(), ArrayN(vec![StrN(":".into())]))])),
      ("rename", ObjectN(vec![("prepareProvider".into(), BoolN(true))])),
      ("documentFormatting", BoolN(true)),
      ("hover", BoolN(true)),
      ("references", BoolN(true)),
      ("definition", BoolN(true)),
      ("foldingRange", BoolN(true)),
    ];
    for (name, capability) in providers {
      capabilities.push((format!("{name}Provider"), capability));
    }
    self.response(id, ObjectN(vec![("capabilities".into(), ObjectN(capabilities))]));
  }
  pub(crate) fn m_shutdown(&mut self, id: IdKind) {
    self.response(id, NullN);
    self.shutdown = true;
  }
}
