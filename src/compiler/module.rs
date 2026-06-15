use crate::prelude::*;
built_in! {self, func, _scope, module;
  {"export", SPECIAL, AtLeast(0),
    f_export => { self.export(func, _scope) },
    f_export_a => { self.export(func, _scope) }
  },
  {"import", SP_SCOPE, AtLeast(1),
    f_import => { self.import_file(func, _scope) },
    f_import_a => { self.import_file(func, _scope) }
  },
}
impl Jsonpiler {
  fn export(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    for _ in 1..=func.val.len {
      let name = func.arg()?.into_ident("Function name")?;
      let Some(u_d) = self.user_defined.get(&name.val) else {
        return err!(name.pos, UndefinedFunc(name.val));
      };
      self.files[func.pos.file].exports.insert(name.val, u_d.clone());
      let id = u_d.val.dep.id;
      self.use_u_d(scope.id, id, name.pos)?;
    }
    Ok(Null(Lit(())))
  }
  fn import(
    &mut self,
    mut imports: BTreeMap<String, Vec<Position>>,
    file_idx: usize,
    pos: Position,
    scope: &mut Scope,
  ) -> ErrOR<()> {
    for (name, mut u_d) in self.files[file_idx].exports.clone() {
      if let Some(refs) = imports.remove(&name) {
        let Some(before) = self.user_defined.get_mut(&name) else {
          self.check_defined(&u_d.pos.with(name.clone()), pos, UserDefinedFunc, scope)?;
          u_d.val.refs.extend_from_slice(&refs);
          self.user_defined.insert(name, u_d);
          continue;
        };
        if before.pos.file == file_idx {
          before.val.refs.extend_from_slice(&refs);
          continue;
        }
        self.check_defined(&u_d.pos.with(name), pos, UserDefinedFunc, scope)?;
      }
    }
    if imports.is_empty() {
      Ok(())
    } else {
      err!(pos, IncludeFuncNotFound(imports.into_keys().collect()))
    }
  }
  fn import_file(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    let file_name = {
      let path = arg!(func, (Str(Lit(x))) => x);
      let folder = Path::new(&self.files[path.pos.file].path).parent().unwrap_or(Path::new("."));
      let full_path =
        folder.join(Path::new(&path.val)).canonicalize().map_err(|val| path.pos.with(val))?;
      path.pos.with(full_path.to_string_lossy().to_string())
    };
    let mut imports: BTreeMap<String, Vec<Position>> = BTreeMap::new();
    for _ in 1..func.val.len {
      let import_func = func.arg()?.into_ident("Function name")?;
      imports.entry(import_func.val.clone()).or_default().push(import_func.pos);
    }
    if self.files[file_name.pos.file].path == file_name.val {
      return err!(file_name.pos, RecursiveInclude(file_name.val));
    }
    if let Some(file_idx) = self.files.iter().position(|file| file.path == file_name.val) {
      self.import(imports, file_idx, file_name.pos, scope)?;
      return Ok(Null(Lit(())));
    }
    let old_globals = take(&mut self.globals);
    let old_user_defined = take(&mut self.user_defined);
    let file_size = fs::metadata(&file_name.val).map_err(|val| file_name.pos.with(val))?.len();
    let total_size: usize = self.files.iter().map(|file| file.parser.val.text.len()).sum();
    if total_size as u64 + file_size > u64::from(GB) {
      return err!(file_name.pos, TooLargeFile);
    }
    let is_jspl = match Path::new(&file_name.val).extension().map(|ext| ext.to_string_lossy()) {
      Some(ext) if ext == "jspl" => true,
      Some(ext) if ext == "json" => false,
      _ => return err!(file_name.pos, UnsupportedFile),
    };
    let source = fs::read_to_string(&file_name.val).map_err(|val| file_name.pos.with(val))?;
    let file_idx = self.files.len();
    let file = self.push_file(source, file_name.val.clone())?;
    let root_id = file.dep.id;
    let old_scope = scope.change(root_id);
    let map_refs = |mut err: JsonpilerErr| {
      err.refs.push(file_name.pos);
      err
    };
    let parsed = if is_jspl { file.parse_jspl() } else { file.parse_json() }
      .map_err(Into::into)
      .map_err(map_refs)?;
    let result = self.eval(parsed, scope).map_err(map_refs)?.val;
    self.drop_all(result, scope).map_err(map_refs)?;
    scope.check_free().map_err(map_refs)?;
    self.use_func(self.first_file()?.dep.id, root_id);
    if self.flags.a64 {
      let stack_size = scope.resolve_a64_stack_size()?;
      self.link_func_a(root_id, scope.replace(old_scope).a64()?, stack_size);
      self.startup.a64_mut()?.push(vec![Bl(root_id)]);
    } else {
      let stack_size = scope.resolve_x64_stack_size()?;
      self.link_func_x(root_id, scope.replace(old_scope).x64()?, stack_size);
      self.startup.x64_mut()?.push(vec![Call(root_id)]);
    }
    self.check_unused_functions(file_idx)?;
    self.globals = old_globals;
    self.user_defined = old_user_defined;
    self.import(imports, file_idx, file_name.pos, scope)?;
    Ok(Null(Lit(())))
  }
}
