use super::super::*;
use crate::prelude::*;
use std::env;
impl Server {
  pub(crate) fn m_definition(&mut self, mut params: JsonNoPos, id: IdKind) {
    let Some(uri) = (|| params.take("textDocument")?.take("uri")?.into_str())() else {
      self.error(id, -32602, "Invalid params");
      return;
    };
    let Some(position) = params.get("position") else {
      self.error(id, -32602, "Invalid params");
      return;
    };
    let Some((_source, jsonpiler, offset)) = self.prepare_symbol_lookup(uri, position) else {
      self.response(id, NullN);
      return;
    };
    let def_opt = jsonpiler
      .analysis
      .as_ref()
      .and_then(|analysis| analysis.find_symbol(offset))
      .and_then(|info| info.definition)
      .map(|definition| jsonpiler.pos2location(definition));
    self.response(id, def_opt.unwrap_or(NullN));
  }
  pub(crate) fn m_hover(&mut self, mut params: JsonNoPos, id: IdKind) {
    let Some(uri) = (|| params.take("textDocument")?.take("uri")?.into_str())() else {
      self.error(id, -32602, "Invalid params");
      return;
    };
    let Some(position) = params.get("position") else {
      self.error(id, -32602, "Invalid params");
      return;
    };
    let hover = (|| {
      let (source, jsonpiler, offset) = self.prepare_symbol_lookup(uri, position)?;
      let analysis = jsonpiler.analysis.as_ref()?;
      let info = analysis.find_symbol(offset)?;
      let cursor_pos = if let Some(definition) = info.definition
        && definition.contains_inclusive(0, offset as u32)
      {
        definition
      } else {
        *info.refs.iter().find(|use_pos| use_pos.contains_inclusive(0, offset as u32))?
      };
      let content = if info.kind == BuiltInFunc {
        load_function_doc(&info.name)
      } else {
        format!(
          "## {}
```jspl
{}({}{})
```
{}
",
          escape_hash(&info.name),
          match info.kind {
            GlobalVar => "global",
            LocalVar => "let",
            BuiltInFunc | UserDefinedFunc => "define",
          },
          info.name,
          match info.kind {
            GlobalVar | LocalVar => format!(": {} = _", info.json_type),
            BuiltInFunc | UserDefinedFunc =>
              if let FuncT(func_params, ret_type) = &info.json_type {
                format!(
                  ", {{ {} }}, {}, {{ _ }}",
                  func_params
                    .iter()
                    .map(|(param_name, json_type)| format!("{param_name}: {json_type}"))
                    .collect::<Vec<_>>()
                    .join("; "),
                  ret_type
                )
              } else {
                String::new()
              },
          },
          info.kind
        )
      };
      Some(ObjectN(vec![
        (
          "contents".into(),
          ObjectN(vec![("kind".into(), StrN("markdown".into())), ("value".into(), StrN(content))]),
        ),
        ("range".into(), pos2range(&source.text, cursor_pos)),
      ]))
    })();
    self.response(id, hover.unwrap_or(NullN));
  }
  pub(crate) fn m_references(&mut self, mut params: JsonNoPos, id: IdKind) {
    let Some(uri) = (|| params.take("textDocument")?.take("uri")?.into_str())() else {
      self.error(id, -32602, "Invalid params");
      return;
    };
    let Some(position) = params.get("position") else {
      self.error(id, -32602, "Invalid params");
      return;
    };
    let includes_decl =
      params.get("context").and_then(|ctx| ctx.get_bool("includeDeclaration")).unwrap_or(true);
    let refs = (|| {
      let (_source, jsonpiler, offset) = self.prepare_symbol_lookup(uri, position)?;
      let analysis = jsonpiler.analysis.as_ref()?;
      let info = analysis.find_symbol(offset)?;
      let mut refs = vec![];
      if includes_decl && let Some(definition) = info.definition {
        refs.push(jsonpiler.pos2location(definition));
      }
      for ref_pos in &info.refs {
        refs.push(jsonpiler.pos2location(*ref_pos));
      }
      Some(refs)
    })()
    .unwrap_or_default();
    self.response(id, ArrayN(refs));
  }
  fn prepare_symbol_lookup(
    &mut self,
    uri: String,
    position: &JsonNoPos,
  ) -> Option<(Source, Jsonpiler, usize)> {
    self.cancel_timer(&uri);
    self.flush(uri.clone());
    let source = self.get_source(&uri)?;
    let offset = range2offset(&source.text, position)?;
    let mut jsonpiler = Jsonpiler::new(true);
    jsonpiler.push_parser(source.text.clone(), uri2path(&uri));
    let parsed = jsonpiler.first_parser_mut().ok()?.parse_jspl().ok()?;
    jsonpiler.compile(parsed).ok()?;
    Some((source, jsonpiler, offset))
  }
}
impl Jsonpiler {
  pub(crate) fn pos2location(&self, pos: Position) -> JsonNoPos {
    let file = &self.parsers[pos.file as usize];
    ObjectN(vec![
      ("uri".into(), StrN(path2uri(&file.val.file))),
      ("range".into(), pos2range(&file.val.source, pos)),
    ])
  }
}
impl Analysis {
  pub(crate) fn find_symbol(&self, offset: usize) -> Option<&SymbolInfo> {
    self.symbols.iter().find(|info| {
      let mut def_refs = info.refs.iter().chain(info.definition.iter());
      def_refs.any(|use_pos| use_pos.contains_inclusive(0, offset as u32))
        && !(info.name == "$" && info.kind == BuiltInFunc)
    })
  }
}
fn pos2range(text: &str, pos: Position) -> JsonNoPos {
  format_range(offset2range(text, pos.offset as usize), offset2range(text, pos.end() as usize))
}
fn escape_hash(string: &str) -> String {
  let chars = r#"\`*_[]"#;
  let mut out = String::with_capacity(string.len());
  for char in string.chars() {
    if chars.contains(char) {
      out.push('\\');
    }
    out.push(char);
  }
  out
}
#[expect(clippy::string_slice, clippy::print_stderr)]
pub fn load_function_doc(name: &str) -> String {
  let Some(exe_dir) =
    env::current_exe().ok().and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
  else {
    return String::new();
  };
  let dir = exe_dir.join("../docs/functions");
  let Ok(entries) = fs::read_dir(&dir) else {
    eprintln!("Failed to read docs/functions directory: {}", dir.display());
    return String::new();
  };
  let needle = format!("## {}", escape_hash(name));
  for entry in entries.flatten() {
    let path = entry.path();
    if path.file_name().and_then(|os_str| os_str.to_str()) == Some("README.md") {
      continue;
    }
    if path.extension().and_then(|os_str| os_str.to_str()) != Some("md") {
      continue;
    }
    let Ok(content) = fs::read_to_string(&path) else {
      eprintln!("Failed to read function documentation file: {}", path.display());
      continue;
    };
    if let Some(start) = content.find(&needle) {
      let bytes = content.as_bytes();
      let pos = start + needle.len();
      let ok = bytes.get(pos) == Some(&b'\n') || (bytes.get(pos..pos + 1) == Some(b"\r\n"));
      if !ok {
        continue;
      }
      let after = &content[start..];
      if let Some(end) = after[needle.len()..].find("\n## ") {
        let mut end_pos = needle.len() + end;
        if after.as_bytes().get(end_pos - 1) == Some(&b'\r') {
          end_pos -= 1;
        }
        return after[..end_pos].to_owned();
      } else {
        return after.to_owned();
      }
    }
  }
  String::new()
}
