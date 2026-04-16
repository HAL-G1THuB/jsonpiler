use crate::prelude::*;
built_in! {self, func, _scope, module;
  f_export => {"export", SPECIAL, AtLeast(0), {
    for _ in 1..=func.val.len {
      let name = func.arg()?.into_ident("Function name")?;
      let Some(u_d) = self.user_defined.get(&name.val) else {
        return err!(name.pos, UndefinedFunc(name.val))
      };
      self.parsers[func.pos.file as usize].val.exports.insert(name.val, u_d.clone());
    }
    Ok(Null(Lit(())))
  }},
  f_import => {"import", SP_SCOPE, AtLeast(1), {
    self.import_file(func, _scope)
  }},
}
impl Jsonpiler {
  fn import_file(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    let (file, pos) = {
      let Pos { val: path, pos } = unwrap_arg!(
        self, func.arg()?, "File path", vec![StrT], (Str(Lit(x))) => x
      );
      let folder =
        Path::new(&self.parsers[pos.file as usize].val.file).parent().unwrap_or(Path::new("."));
      let full_path = folder.join(Path::new(&path)).canonicalize();
      (full_path.map_err(|val| pos.with(val))?.to_string_lossy().to_string(), pos)
    };
    let mut imports = BTreeSet::new();
    for _ in 1..func.val.len {
      imports.insert(func.arg()?.into_ident("Function name")?.val);
    }
    if self.parsers[pos.file as usize].val.file == file {
      return err!(pos, RecursiveInclude(file));
    }
    if let Some(file_idx) = self.parsers.iter().position(|parser| parser.val.file == file) {
      for (name, val) in &self.parsers[file_idx].val.exports {
        if let Some(import_func) = imports.take(name) {
          if self.user_defined.get(name).is_none_or(|u_d| u_d.pos.file as usize != file_idx) {
            self.check_defined(name, pos, scope)?;
          }
          self.user_defined.entry(import_func).or_insert(val.clone());
        }
      }
      if !imports.is_empty() {
        return err!(pos, IncludeFuncNotFound(imports));
      }
      return Ok(Null(Lit(())));
    }
    let root_id = self.id();
    let old_globals = take(&mut self.globals);
    let old_user_defined = take(&mut self.user_defined);
    let file_size = fs::metadata(&file).map_err(|val| pos.with(val))?.len();
    if file_size > u64::from(GB) {
      return err!(pos, TooLargeFile);
    }
    let source = fs::read(&file).map_err(|val| pos.with(val))?;
    let file_idx = self.parsers.len();
    self.parsers.push(<Pos<Parser>>::new(source, u32::try_from(file_idx)?, file.clone(), root_id));
    let total_size = self.parsers.iter().map(|parser| parser.val.source.len()).sum::<usize>();
    if total_size > GB as usize {
      return err!(pos, TooLargeFile);
    }
    let old_scope = scope.change(root_id);
    let mut try_include = || -> ErrOR<()> {
      let parsed = match Path::new(&file).extension().map(|ext| ext.to_string_lossy()) {
        Some(ext) if ext == "jspl" => self.parsers[file_idx].parse_jspl(),
        Some(ext) if ext == "json" => self.parsers[file_idx].parse_json(),
        _ => return err!(pos, UnsupportedFile),
      }?;
      let result = self.eval(parsed, scope)?.val;
      self.drop_json(result, scope, false);
      self.drop_all_scope(scope);
      self.drop_global(scope);
      scope.check_free()?;
      Ok(())
    };
    let mut result = try_include();
    if let Err(Compilation(_, pos_vec) | Parse(_, pos_vec)) = &mut result {
      pos_vec.push(pos);
    }
    result?;
    let stack_size = scope.resolve_stack_size()?;
    self.link_function(root_id, &scope.replace(old_scope), stack_size);
    self.use_function(self.parsers[0].val.dep.id, root_id);
    self.startup.push(Call(root_id));
    self.check_unused_functions(&self.parsers[file_idx].val.dep.clone());
    self.globals = old_globals;
    self.user_defined = old_user_defined;
    self.import_functions(imports, file_idx, pos, scope)?;
    Ok(Null(Lit(())))
  }
  fn import_functions(
    &mut self,
    mut imports: BTreeSet<String>,
    file_idx: usize,
    pos: Position,
    scope: &mut Scope,
  ) -> ErrOR<()> {
    for (name, val) in &self.parsers[file_idx].val.exports {
      if let Some(import_func) = imports.take(name) {
        self.check_defined(name, pos, scope)?;
        self.user_defined.insert(import_func, val.clone());
      }
    }
    if imports.is_empty() { Ok(()) } else { err!(pos, IncludeFuncNotFound(imports)) }
  }
}
