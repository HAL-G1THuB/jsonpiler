use super::super::*;
use crate::prelude::*;
impl Server {
  pub(crate) fn m_definition(&mut self, params: JsonNoPos, id: IdKind) {
    let definition = (|| {
      let (jsonpiler, info, _) = self.prepare_symbol_lookup(params)?;
      Some(jsonpiler.pos2location(info.def?))
    })();
    self.response(id, definition.unwrap_or(NullN));
  }
  pub(crate) fn m_hover(&mut self, params: JsonNoPos, id: IdKind) {
    let hover = (|| {
      let (jsonpiler, info, pos) = self.prepare_symbol_lookup(params)?;
      let content = if info.kind == BuiltInFunc {
        self
          .docs
          .get(&info.name)
          .cloned()
          .unwrap_or_else(|| format!("No documentation for `{}`", info.name))
      } else {
        format!(
          "```jspl\n{}\n```\n{}\n",
          match info.kind {
            Argument => format!("{{ {}: {} }}", info.name, info.json_type),
            GlobalVar => format!("global({}: {}) = _", info.name, info.json_type),
            LocalVar => format!("let({}: {}) = _", info.name, info.json_type),
            BuiltInFunc | UserDefinedFunc =>
              if let FuncT(sig) = &info.json_type {
                let args = sig
                  .params
                  .iter()
                  .map(|(name, json_type)| format!("{name}: {json_type}"))
                  .collect::<Vec<_>>()
                  .join(", ");
                format!("{}({args}) -> {}", info.name, sig.ret_type)
              } else {
                format!("{}(?) -> ?", info.name)
              },
          },
          info.kind
        )
      };
      let range = pos2range(&jsonpiler.files[pos.file].parser.val.text, pos);
      Some(ObjectN(vec![
        (
          "contents".into(),
          ObjectN(vec![("kind".into(), StrN("markdown".into())), ("value".into(), StrN(content))]),
        ),
        ("range".into(), range),
      ]))
    })();
    self.response(id, hover.unwrap_or(NullN));
  }
  pub(crate) fn m_prepare_rename(&mut self, params: JsonNoPos, id: IdKind) {
    let definition = (|| {
      let (jsonpiler, info, pos) = self.prepare_symbol_lookup(params)?;
      if info.kind == BuiltInFunc {
        let log =
          LogMsg::new(MsgType::Warning, "Built-in function cannot be renamed.".into(), None);
        return Some(Err((-32602, log)));
      }
      let file = &jsonpiler.files[pos.file];
      Some(Ok(ObjectN(vec![
        ("range".into(), pos2range(&file.parser.val.text, pos)),
        ("placeholder".into(), StrN(info.name.clone())),
      ])))
    })()
    .unwrap_or(Ok(NullN));
    match definition {
      Ok(def) => self.response(id, def),
      Err((code, log)) => self.error(id, code, log),
    }
  }
  pub(crate) fn m_references(&mut self, params: JsonNoPos, id: IdKind) {
    let includes_decl =
      (|| params.get("context")?.get_bool("includeDeclaration"))().unwrap_or(true);
    let refs = (|| {
      let (jsonpiler, info, _) = self.prepare_symbol_lookup(params)?;
      let refs = if includes_decl { info.def_refs() } else { info.refs.clone() }
        .into_iter()
        .map(|pos| jsonpiler.pos2location(pos))
        .collect();
      Some(refs)
    })()
    .unwrap_or_default();
    self.response(id, ArrayN(refs));
  }
  pub(crate) fn m_rename(&mut self, mut params: JsonNoPos, id: IdKind) {
    let result = (|| {
      let new_name = params.take_str("newName")?;
      if validate_new_name(&new_name).is_none() {
        let log = LogMsg::new(MsgType::Warning, "Invalid name".into(), Some(new_name));
        return Some(Err((-32602, log)));
      }
      let (jsonpiler, info, _) = self.prepare_symbol_lookup(params)?;
      let mut changes: BTreeMap<String, Vec<JsonNoPos>> = BTreeMap::new();
      for pos in info.def_refs() {
        let file = &jsonpiler.files[pos.file];
        let uri = path2uri(&file.path);
        let change = ObjectN(vec![
          ("range".into(), pos2range(&file.parser.val.text, pos)),
          ("newText".into(), StrN(new_name.clone())),
        ]);
        changes.entry(uri).or_default().push(change);
      }
      Some(Ok(ObjectN(vec![(
        "changes".into(),
        ObjectN(changes.into_iter().map(|(uri, edits)| (uri, ArrayN(edits))).collect()),
      )])))
    })()
    .unwrap_or(Ok(NullN));
    match result {
      Ok(edit) => self.response(id, edit),
      Err((code, log)) => self.error(id, code, log),
    }
  }
  fn prepare_symbol_lookup(
    &mut self,
    mut params: JsonNoPos,
  ) -> Option<(Jsonpiler, SymbolInfo, Position)> {
    let uri = params.take("textDocument")?.take_str("uri")?;
    let position = params.take("position")?;
    self.flush(&uri);
    let source = self.get_source(&uri)?;
    let offset = range2offset(&source.text, &position)?;
    let mut jsonpiler = Jsonpiler::new(true);
    let file = jsonpiler.push_file(source.text, uri2path(&uri)).ok()?;
    let parsed = file.parse_jspl().ok()?;
    jsonpiler.compile(parsed).ok()?;
    let info = jsonpiler.analysis.as_ref()?.find_symbol(offset)?.clone();
    let pos = *info.def_refs().iter().find(|pos| pos.contains_inclusive(0, offset as u32))?;
    Some((jsonpiler, info, pos))
  }
}
impl Jsonpiler {
  pub(crate) fn pos2location(&self, pos: Position) -> JsonNoPos {
    let file = &self.files[pos.file];
    ObjectN(vec![
      ("uri".into(), StrN(path2uri(&file.path))),
      ("range".into(), pos2range(&file.parser.val.text, pos)),
    ])
  }
}
impl Analysis {
  pub(crate) fn find_symbol(&self, offset: usize) -> Option<&SymbolInfo> {
    self.symbols.iter().find(|info| {
      info.def_refs().iter().any(|use_pos| use_pos.contains_inclusive(0, offset as u32))
        && !(info.name == "$" && info.kind == BuiltInFunc)
    })
  }
}
fn pos2range(text: &str, pos: Position) -> JsonNoPos {
  format_range(offset2range(text, pos.offset), offset2range(text, pos.end()))
}
fn validate_new_name(new_name: &str) -> Option<()> {
  let mut parser = <Pos<Parser>>::new(new_name.into(), 0);
  let parsed = parser.parse_jspl("newName.jspl").ok()?;
  if let Object(Lit(obj)) = parsed.val
    && obj.len() == 1
    && obj[0].0.val == "$"
  {
    if let Str(Lit(name)) = &obj[0].1.val {
      (name == new_name).then_some(())
    } else if let Array(_, Lit(array)) = &obj[0].1.val
      && let Str(Lit(name)) = &array[0].val
      && name == new_name
    {
      Some(())
    } else {
      None
    }
  } else {
    None
  }
}
