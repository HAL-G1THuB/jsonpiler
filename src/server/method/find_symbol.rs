use super::super::*;
use crate::prelude::*;
impl Server {
  pub(crate) fn m_definition(&mut self, params: JsonNoPos, id: IdKind) {
    let definition = (|| {
      let (jsonpiler, offset) = self.prepare_symbol_lookup(params)?;
      Some(jsonpiler.pos2location(jsonpiler.analysis.as_ref()?.find_symbol(offset)?.definition?))
    })();
    self.response(id, definition.unwrap_or(NullN));
  }
  pub(crate) fn m_hover(&mut self, params: JsonNoPos, id: IdKind) {
    let hover = (|| {
      let (jsonpiler, offset) = self.prepare_symbol_lookup(params)?;
      let info = jsonpiler.analysis.as_ref()?.find_symbol(offset)?;
      let cursor_pos = if let Some(definition) = info.definition
        && definition.contains_inclusive(0, offset as u32)
      {
        definition
      } else {
        *info.refs.iter().find(|use_pos| use_pos.contains_inclusive(0, offset as u32))?
      };
      let content = if info.kind == BuiltInFunc {
        self
          .docs
          .as_ref()
          .and_then(|docs| docs.get(&info.name).cloned())
          .unwrap_or_else(|| format!("No documentation for `{}`", info.name))
      } else {
        format!(
          "```jspl\n{}\n```\n{}\n",
          match info.kind {
            Argument => format!("{{ {}: {} }}", info.name, info.json_type),
            GlobalVar => format!("global({}: {} = _)", info.name, info.json_type),
            LocalVar => format!("let({}: {} = _)", info.name, info.json_type),
            BuiltInFunc | UserDefinedFunc => format!(
              "define({}{})",
              info.name,
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
              }
            ),
          },
          info.kind
        )
      };
      Some(ObjectN(vec![
        (
          "contents".into(),
          ObjectN(vec![("kind".into(), StrN("markdown".into())), ("value".into(), StrN(content))]),
        ),
        (
          "range".into(),
          pos2range(&jsonpiler.parsers[cursor_pos.file as usize].val.text, cursor_pos),
        ),
      ]))
    })();
    self.response(id, hover.unwrap_or(NullN));
  }
  pub(crate) fn m_references(&mut self, params: JsonNoPos, id: IdKind) {
    let includes_decl =
      params.get("context").and_then(|ctx| ctx.get_bool("includeDeclaration")).unwrap_or(true);
    let refs = (|| {
      let (jsonpiler, offset) = self.prepare_symbol_lookup(params)?;
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
  fn prepare_symbol_lookup(&mut self, mut params: JsonNoPos) -> Option<(Jsonpiler, usize)> {
    let uri = params.take("textDocument")?.take("uri")?.into_str()?;
    let position = params.take("position")?;
    self.flush(uri.clone());
    let source = self.get_source(&uri)?;
    let offset = range2offset(&source.text, &position)?;
    let mut jsonpiler = Jsonpiler::new(true);
    jsonpiler.push_parser(source.text, uri2path(&uri));
    let parsed = jsonpiler.first_parser_mut().ok()?.parse_jspl().ok()?;
    jsonpiler.compile(parsed).ok()?;
    Some((jsonpiler, offset))
  }
}
impl Jsonpiler {
  pub(crate) fn pos2location(&self, pos: Position) -> JsonNoPos {
    let file = &self.parsers[pos.file as usize];
    ObjectN(vec![
      ("uri".into(), StrN(path2uri(&file.val.file))),
      ("range".into(), pos2range(&file.val.text, pos)),
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
