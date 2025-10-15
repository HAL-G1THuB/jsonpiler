use crate::{
  Arity::AtLeast, Bind::Lit, CompilationErrKind::*, ErrOR, FuncInfo, Json, Jsonpiler,
  JsonpilerErr::*, Parser, ScopeInfo, WithPos, built_in, err, take_arg,
};
use core::mem::{replace, take};
use std::{collections::HashMap, env, fs, path::Path};
built_in! {self, func, scope, file;
  include => {"include", SCOPE, AtLeast(1), {
    let path = take_arg!(self, func, "String (Literal)", Json::String(Lit(x)) => x);
    let mut includes = vec![];
    for _ in 1..func.len {
      includes.push(take_arg!(self, func, "String (Literal)", Json::String(Lit(x)) => x).value);
    }
    let old_locals = scope.replace_locals(vec![HashMap::new()]);
    let globals = take(&mut self.globals);
    let user_defined = take(&mut self.user_defined);
    let file = Path::new(&path.value);
    let cwd = env::current_dir().map_err(|err| {
      WithPos{value: err, pos: path.pos}
    })?;
    let Some(folder) = Path::new(self.parser[path.pos.file].get_file()).parent() else {
      return err!(self, path.pos, ParentDirNotFound)
    };
    let full_path = cwd.join(folder).join(file);
    let abs_path = full_path.canonicalize().map_err(|err| {
      WithPos{value: err, pos: path.pos}
    })?;
    if self.parser[path.pos.file].get_file() == abs_path.to_string_lossy() {
      return err!(self, path.pos, RecursiveInclude(path.value));
    }
    if let Some(file_idx) = self.parser
      .iter()
      .position(|parser| Path::new(parser.get_file()) == abs_path)
    {
      #[expect(clippy::iter_over_hash_type)]
      for (name, value) in &self.files[file_idx] {
        if includes.contains(name) {
          if self.builtin.contains_key(name) {
            return err!(self, path.pos, ExistentBuiltin(name.clone()));
          }
          let other_idx_opt = self.user_defined.get(name).map(|asm_func| asm_func.file);
          if other_idx_opt != Some(file_idx) {
            return err!(self, path.pos, ExistentUserDefined(name.clone()));
          }
          if other_idx_opt.is_none(){
            self.user_defined.insert(name.clone(), value.clone());
          }
          includes.retain(|na| na != name);
        }
      }
      if !includes.is_empty() {
        return err!(self, path.pos, IncludeFuncNotFound(includes));
      }
      scope.replace_locals(old_locals);
      self.globals = globals;
      self.user_defined = user_defined;
      return Ok(Json::Null);
    }
    let metadata = fs::metadata(&abs_path).map_err(|err| {
      WithPos{value: err, pos: path.pos}
    })?;
    if metadata.len() > 1 << 30u8 {
      return err!(self, path.pos, TooLargeFile);
    }
    let bytes = fs::read(&abs_path).map_err(|err| {
      WithPos{value: err, pos: path.pos}
    })?;
    let file_idx = self.files.len();
    let mut new_parser = Parser::from(bytes, file_idx, abs_path.to_string_lossy().to_string());
    let is_jspl = match abs_path.extension() {
      Some(ext) if ext == "jspl" => true,
      Some(ext) if ext == "json" => false,
      _ => return err!(self, path.pos, UnsupportedExtension),
    };
    self.files.push(HashMap::new());
    let new_json = new_parser.parse(is_jspl)?;
    self.parser.push(new_parser);
    let ret_val = self.eval(new_json, scope)?;
    scope.drop_json(ret_val)?;
    scope.replace_locals(old_locals);
    self.globals = globals;
    #[expect(clippy::iter_over_hash_type)]
    for (name, value) in replace(&mut self.user_defined, user_defined) {
      if includes.contains(&name) {
        if self.builtin.contains_key(&name) {
          return err!(self, path.pos, ExistentBuiltin(name));
        }
        if self.user_defined.contains_key(&name) {
          return err!(self, path.pos, ExistentUserDefined(name));
        }
        self.user_defined.insert(name.clone(), value.clone());
          includes.retain(|na| na != &name);
      }
      self.files[file_idx].insert(name, value);
    }
    if !includes.is_empty() {
      return err!(self, path.pos, IncludeFuncNotFound(includes));
    }
    Ok(Json::Null)
  }},
}
