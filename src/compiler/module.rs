use crate::prelude::*;
built_in! {self, func, _scope, module;
  f_export => {"export", SPECIAL, AtLeast(0), {
    for _ in 1..=func.len {
      let name = func.arg()?.into_ident("Function name")?;
      let Some(u_d) = self.user_defined.get(&name.val) else {
        return err!(name.pos, UndefinedFunc(name.val))
      };
      self.parsers[func.pos.file as usize].exports.insert(name.val, u_d.clone());
    }
    Ok(Null(Lit(())))
  }},
  f_import => {"import", SP_SCOPE, AtLeast(1), {
    self.import_file(func, _scope)
  }},
}
impl Jsonpiler {
  fn import_file(&mut self, func: &mut BuiltIn, scope: &mut Scope) -> ErrOR<Json> {
    let (file, pos) = {
      let WithPos { val: path, pos } = unwrap_arg!(
        self, func.arg()?, "File path", vec![StrT], (Str(Lit(x))) => x
      );
      let folder =
        Path::new(&self.parsers[pos.file as usize].file).parent().unwrap_or(Path::new("."));
      let full_path = folder.join(Path::new(&path)).canonicalize();
      (full_path.map_err(|val| pos.with(val))?.to_string_lossy().to_string(), pos)
    };
    let mut includes = BTreeSet::new();
    for _ in 1..func.len {
      let arg = func.arg()?.into_ident("Function name")?;
      includes.insert(arg.val);
    }
    if self.parsers[pos.file as usize].file == file {
      return err!(pos, RecursiveInclude(file));
    }
    if let Some(file_idx) = self.parsers.iter().position(|parser| parser.file == file) {
      for (name, val) in &self.parsers[file_idx].exports {
        if includes.contains(name) {
          if self.user_defined.get(name).is_none_or(|u_d| u_d.pos.file as usize != file_idx) {
            self.check_defined(name, pos, scope)?;
          }
          self.user_defined.entry(name.to_owned()).or_insert(val.clone());
          includes.remove(name);
        }
      }
      if !includes.is_empty() {
        return err!(pos, IncludeFuncNotFound(includes));
      }
      return Ok(Null(Lit(())));
    }
    let root_id = self.id();
    let old_globals = take(&mut self.globals);
    let old_user_defined = take(&mut self.user_defined);
    self.root_id.push((root_id, vec![]));
    if fs::metadata(&file).map_err(|val| pos.with(val))?.len() > u64::from(GB) {
      return err!(pos, TooLargeFile);
    }
    let is_jspl = match Path::new(&file).extension().map(|ext| ext.to_string_lossy()) {
      Some(ext) if ext == "jspl" => true,
      Some(ext) if ext == "json" => false,
      _ => return err!(pos, UnsupportedFile),
    };
    let source = fs::read(&file).map_err(|val| pos.with(val))?;
    let file_idx = self.parsers.len();
    self.parsers.push(Parser::new(
      source,
      u32::try_from(file_idx)?,
      file,
      self.parsers[0].file.clone(),
    ));
    let mut total_size = 0;
    for parser in &self.parsers {
      total_size += parser.source.len();
    }
    if total_size > GB as usize {
      return err!(pos, TooLargeFile);
    }
    let mut try_include = || -> ErrOR<()> {
      let parsed = self.parsers[file_idx].parse(is_jspl)?;
      let old_scope = scope.change(root_id);
      let epilogue = self.id();
      let result = self.eval(parsed, scope)?.val;
      let stack_size = scope.resolve_stack_size()?;
      self.drop_json(result, scope, false);
      self.drop_all_scope(scope);
      self.drop_global(scope);
      scope.check_free()?;
      let mut insts = vec![];
      insts.extend_from_slice(&scope.replace(old_scope));
      insts.push(Lbl(epilogue));
      self.use_function(self.root_id[0].0, root_id);
      self.link_function(root_id, &insts, stack_size);
      self.startup.push(Call(root_id));
      Ok(())
    };
    let mut result = try_include();
    if let Err(Compilation(_, pos_vec)) = &mut result {
      pos_vec.push(pos);
    }
    result?;
    if let Some((_, root_uses)) = self.root_id.pop() {
      self.check_unused_functions(root_uses);
    }
    self.globals = old_globals;
    self.user_defined = old_user_defined;
    for (name, val) in &self.parsers[file_idx].exports {
      if includes.contains(name) {
        self.check_defined(name, pos, scope)?;
        self.user_defined.insert(name.to_owned(), val.clone());
        includes.remove(name);
      }
    }
    if !includes.is_empty() {
      return err!(pos, IncludeFuncNotFound(includes));
    }
    Ok(Null(Lit(())))
  }
}
