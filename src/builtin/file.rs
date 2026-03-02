use crate::prelude::*;
use std::{env, fs, path::Path};
built_in! {self, func, scope, file;
  include => {"include", SCOPE, AtLeast(1), {
    let (file, pos) = {
      let WithPos { val: path, pos } = arg!(self, func, (Str(Lit(x))) => x);
      let folder = or_err!((Path::new(&self.parser[pos.file].file).parent()), pos, ParentDirNotFound)?;
      let full_path = env::current_dir().map_err(|val| pos.with(val))?.join(folder).join(Path::new(&path));
      (full_path.canonicalize().map_err(|val| pos.with(val))?.to_string_lossy().to_string(), pos)
    };
    let mut includes = vec![];
    for _ in 1..func.len {
      let arg = arg!(self, func, (Str(Lit(x))) => x);
      includes.push(arg.val);
    }
    if self.parser[pos.file].file == file {
      return err!(pos, RecursiveInclude(file));
    }
    if let Some(file_idx) = self.parser.iter().position(|parser| parser.file == file)
    {
      #[expect(clippy::iter_over_hash_type)]
      for (name, val) in &self.files[file_idx] {
        if includes.contains(name) {
          if self.builtin.contains_key(name) {
            return err!(pos, ExistentFunc(Builtin, name.clone()));
          }
          if self.user_defined.get(name).is_some_and(|func| func.pos.file != file_idx) {
            return err!(pos, ExistentFunc(UserDefined, name.clone()))
          }
          self.user_defined.entry(name.clone()).or_insert(val.clone());
          includes.retain(|na| na != name);
        }
      }
      if !includes.is_empty() {
        return err!(pos, IncludeFuncNotFound(includes));
      }
      return Ok(Null);
    }
    let old_local_top = take(&mut scope.local_top);
    let old_locals = take(&mut scope.locals);
    let old_globals = take(&mut self.globals);
    let old_user_defined = take(&mut self.user_defined);
    if fs::metadata(&file).map_err(|val| pos.with(val))?.len() > 1 << 30u8 {
      return err!(pos, TooLargeFile);
    }
    let file_idx = self.files.len();
    self.files.push(HashMap::new());
    let is_jspl = match Path::new(&file).extension().map(|ext| ext.to_string_lossy()) {
      Some(ext) if ext == "jspl" => true,
      Some(ext) if ext == "json" => false,
      _ => return err!(pos, UnsupportedFile),
    };
    let source = fs::read(&file).map_err(|val| pos.with(val))?;
    self.parser.push(Parser::from(source, file_idx, file));
    let new_json = self.parser[file_idx].parse(is_jspl)?;
    let result = self.eval(new_json, scope)?;
    self.drop_json(result, scope, false)?;
    for local in replace(&mut scope.local_top,  old_local_top).into_values() {
      self.drop_json(local, scope, true)?;
    }
    for locals in replace(&mut scope.locals, old_locals) {
      for local in locals.into_values() {
        self.drop_json(local, scope, true)?;
      }
    }
    scope.check_free()?;
    self.globals = old_globals;
    #[expect(clippy::iter_over_hash_type)]
    for (name, val) in replace(&mut self.user_defined, old_user_defined) {
      if includes.contains(&name) {
        if self.builtin.contains_key(&name) {
          return err!(pos, ExistentFunc(Builtin, name));
        }
        if self.user_defined.contains_key(&name) {
          return err!(pos, ExistentFunc(UserDefined, name));
        }
        self.user_defined.insert(name.clone(), val.clone());
        includes.retain(|na| na != &name);
      }
      self.files[file_idx].insert(name, val);
    }
    if !includes.is_empty() {
      return err!(pos, IncludeFuncNotFound(includes));
    }
    Ok(Null)
  }},
}
