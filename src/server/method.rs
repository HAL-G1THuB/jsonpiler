mod diagnostic;
mod find_symbol;
mod text_document;
use crate::prelude::*;
use std::process::exit;
impl Server {
  #[expect(clippy::print_stderr)]
  pub(crate) fn eval_method(&mut self, method: &str, params: JsonNoPos, id_opt: Option<IdKind>) {
    macro_rules! unwrap_id {
      ($id_opt:expr) => {{
        let Some(id) = $id_opt else {
          eprintln!("Missing id");
          return;
        };
        id
      }};
    }
    match method {
      "initialize" => self.m_initialize(unwrap_id!(id_opt)),
      "initialized" => (),
      "textDocument/didOpen" => self.m_did_open(params),
      "textDocument/didChange" => self.m_did_change(params),
      "textDocument/didSave" => (),
      "textDocument/completion" => self.m_completion(params, unwrap_id!(id_opt)),
      "textDocument/definition" => self.m_definition(params, unwrap_id!(id_opt)),
      "textDocument/references" => self.m_references(params, unwrap_id!(id_opt)),
      "textDocument/formatting" => self.m_formatting(params, unwrap_id!(id_opt)),
      "textDocument/hover" => self.m_hover(params, unwrap_id!(id_opt)),
      "textDocument/didClose" => self.m_did_close(params),
      "shutdown" => self.m_shutdown(unwrap_id!(id_opt)),
      "exit" => exit(if self.shutdown { 0 } else { 1 }),
      _ => {
        eprintln!("Method not found");
        if let Some(id) = id_opt {
          self.error(id, -32601, "Method not found");
        }
      }
    }
  }
  pub(crate) fn m_completion(&mut self, mut params: JsonNoPos, id: IdKind) {
    let Some(_uri) = (|| params.take("textDocument")?.take("uri")?.into_str())() else {
      self.error(id, -32602, "Invalid params");
      return;
    };
    let Some(_position) = params.get("position") else {
      self.error(id, -32602, "Invalid params");
      return;
    };
    let (trigger_kind, trigger_character) = params
      .get("context")
      .and_then(|context| {
        context.get_int("triggerKind").map(|trigger_kind| {
          (trigger_kind, context.get("triggerCharacter").and_then(JsonNoPos::as_str))
        })
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
          .collect::<Vec<_>>(),
        _ => vec![],
      }
    } else {
      vec![]
    });
    self.response(id, items);
  }
  pub(crate) fn m_initialize(&mut self, id: IdKind) {
    use super::build_doc_cache;
    self.docs = Some(build_doc_cache());
    let mut capabilities = vec![
      ("textDocumentSync".into(), IntN(2)),
      (
        "completionProvider".into(),
        ObjectN(vec![("triggerCharacters".into(), ArrayN(vec![StrN(":".into())]))]),
      ),
    ];
    const PROVIDERS: [&str; 4] = ["documentFormatting", "hover", "references", "definition"];
    for provider in PROVIDERS {
      capabilities.push((format!("{}Provider", provider), BoolN(true)));
    }
    self.response(id, ObjectN(vec![("capabilities".into(), ObjectN(capabilities))]));
  }
  pub(crate) fn m_shutdown(&mut self, id: IdKind) {
    self.response(id, NullN);
    self.shutdown = true;
  }
}
