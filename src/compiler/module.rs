use crate::prelude::*;
built_in! {self, func, _scope, module;
  f_export => {"export", SPECIAL, AtLeast(0), {
    for _ in 1..=func.val.len {
      let name = func.arg()?.into_ident("Function name")?;
      let Some(u_d) = self.user_defined.get_mut(&name.val) else {
        return err!(name.pos, UndefinedFunc(name.val))
      };
      u_d.val.refs.push(name.pos);
      self.parsers[func.pos.file as usize].val.exports.insert(name.val, u_d.clone());
    }
    Ok(Null(Lit(())))
  }},
  f_import => {"import", SP_SCOPE, AtLeast(1), { self.import_file(func, _scope) }},
}
impl Jsonpiler {
  fn import(
    &mut self,
    mut imports: BTreeMap<String, Vec<Position>>,
    file_idx: usize,
    pos: Position,
    scope: &mut Scope,
  ) -> ErrOR<()> {
    for (name, mut u_d) in self.parsers[file_idx].val.exports.clone() {
      if let Some(refs) = imports.remove(&name) {
        let Some(before) = self.user_defined.get_mut(&name) else {
          self.check_defined(&u_d.pos.with(name.clone()), pos, scope)?;
          u_d.val.refs.extend(refs);
          self.user_defined.insert(name.clone(), u_d.clone());
          continue;
        };
        if before.pos.file as usize == file_idx {
          before.val.refs.extend(refs);
          continue;
        }
        self.check_defined(&u_d.pos.with(name.clone()), pos, scope)?;
      }
    }
    if imports.is_empty() {
      Ok(())
    } else {
      err!(pos, IncludeFuncNotFound(imports.into_keys().collect()))
    }
  }
  fn import_file(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    let file = {
      let path = arg!(func, (Str(Lit(x))) => x);
      let folder = Path::new(&self.parsers[path.pos.file as usize].val.file)
        .parent()
        .unwrap_or(Path::new("."));
      let full_path = folder.join(Path::new(&path.val)).canonicalize();
      path.pos.with(full_path.map_err(|val| path.pos.with(val))?.to_string_lossy().to_string())
    };
    let mut imports: BTreeMap<String, Vec<Position>> = BTreeMap::new();
    for _ in 1..func.val.len {
      let import_func = func.arg()?.into_ident("Function name")?;
      imports.entry(import_func.val.clone()).or_default().push(import_func.pos);
    }
    if self.parsers[file.pos.file as usize].val.file == file.val {
      return err!(file.pos, RecursiveInclude(file.val));
    }
    if let Some(file_idx) = self.parsers.iter().position(|parser| parser.val.file == file.val) {
      self.import(imports, file_idx, file.pos, scope)?;
      return Ok(Null(Lit(())));
    }
    let old_globals = take(&mut self.globals);
    let old_user_defined = take(&mut self.user_defined);
    let file_size = fs::metadata(&file.val).map_err(|val| file.pos.with(val))?.len();
    let total_size: usize = self.parsers.iter().map(|parser| parser.val.text.len()).sum();
    if total_size as u64 + file_size > u64::from(GB) {
      return err!(file.pos, TooLargeFile);
    }
    let is_jspl = match Path::new(&file.val).extension().map(|ext| ext.to_string_lossy()) {
      Some(ext) if ext == "jspl" => true,
      Some(ext) if ext == "json" => false,
      _ => return err!(file.pos, UnsupportedFile),
    };
    let source = fs::read_to_string(&file.val).map_err(|val| file.pos.with(val))?;
    let file_idx = self.parsers.len();
    let parser = self.push_parser(source, file.val.clone())?;
    let root_id = parser.val.dep.id;
    let old_scope = scope.change(root_id);
    let map_pos_vec = |mut err| {
      if let Compilation(_, pos_vec) | Parse(_, pos_vec) = &mut err {
        pos_vec.push(file.pos);
      }
      err
    };
    let parsed = if is_jspl { parser.parse_jspl() } else { parser.parse_json() }
      .map_err(|err| map_pos_vec(err.into()))?;
    let result = self.eval(parsed, scope).map_err(map_pos_vec)?.val;
    self.drop_all(result, scope).map_err(map_pos_vec)?;
    scope.check_free().map_err(map_pos_vec)?;
    let stack_size = scope.resolve_stack_size()?;
    self.link_function(root_id, &scope.replace(old_scope), stack_size);
    self.use_function(self.first_parser()?.val.dep.id, root_id);
    self.startup.push(Call(root_id));
    self.check_unused_functions(file_idx)?;
    self.globals = old_globals;
    self.user_defined = old_user_defined;
    self.import(imports, file_idx, file.pos, scope)?;
    Ok(Null(Lit(())))
  }
}
