//! Implementation of the compiler inside the `Jsonpiler`.
use super::{
  Bind::{Lit, Var},
  Builtin, ErrOR, FuncInfo, GlobalKind, JFunc, Json, JsonWithPos, Jsonpiler, Name, Position,
  ScopeInfo,
  VarKind::Global,
  add, err,
  utility::{get_int_str, imp_call, mn, scope_begin, scope_end},
};
use core::mem;
use std::{
  collections::{HashMap, HashSet},
  fs::File,
  io::{self, Write as _},
};
impl Jsonpiler {
  /// Builds the assembly code from the parsed JSON.
  /// This function is the main entry point for the compilation.
  /// It takes the parsed JSON, sets up the initial function table,
  /// evaluates the JSON, and writes the resulting assembly code to a file.
  /// # Arguments
  /// * `source` - The JSON string.
  /// * `json_file` - The name of the original JSON file.
  /// * `out_file` - The name of the file to write the assembly code to.
  /// # Returns
  /// * `Ok(())`
  /// * `Err(Box<dyn Error>)` - If an error occurred during the compilation.
  /// # Errors
  /// * `Box<dyn Error>` - If an error occurred during the compilation.
  #[inline]
  pub fn build(&mut self, source: String, out_file: &str) -> ErrOR<()> {
    let json = self.parse(source)?;
    self.include_flag = HashSet::new();
    self.text = vec![];
    self.bss = vec![];
    self.data = vec![];
    self.global_seed = 0;
    self.builtin = HashMap::new();
    self.vars = vec![HashMap::new(), HashMap::new()];
    self.all_register();
    let mut info = ScopeInfo::default();
    let result = self.eval(json.value, &mut info)?;
    let mut writer = io::BufWriter::new(File::create(out_file)?);
    writer.write_all(mn(".intel_syntax", &["noprefix"]).as_bytes())?;
    writer.write_all(include_bytes!("asm/once/data.s"))?;
    for data in &mut self.data {
      writer.write_all(data.as_bytes())?;
    }
    writer.write_all(include_bytes!("asm/once/bss.s"))?;
    for bss in &mut self.bss {
      writer.write_all(bss.as_bytes())?;
    }
    writer.write_all(include_bytes!("asm/once/main.s"))?;
    write!(
      writer,
      include_str!("asm/common/prologue.s"),
      size = format!("0x{:x}", info.calc_alloc(8)?)
    )?;
    writer.write_all(include_bytes!("asm/once/startup.s"))?;
    for body in &mut info.body {
      writer.write_all(body.as_bytes())?;
    }
    if let Json::Int(int) = result {
      writer.write_all(mn("mov", &["rcx", &get_int_str(&int, &mut info)?]).as_bytes())
    } else {
      writer.write_all(mn("xor", &["ecx", "ecx"]).as_bytes())
    }?;
    writer.write_all(imp_call("ExitProcess").as_bytes())?;
    writer.write_all(mn(".seh_endproc", &[]).as_bytes())?;
    for text in &mut self.text {
      writer.write_all(text.as_bytes())?;
    }
    writer.write_all(include_bytes!("asm/once/handler.s"))?;
    writer.flush()?;
    Ok(())
  }
  /// Evaluate JSON representation.
  pub(crate) fn eval(&mut self, mut json: Json, info: &mut ScopeInfo) -> ErrOR<Json> {
    if let Json::Array(Lit(list)) = &mut json {
      Ok(Json::Array(Lit(self.eval_args(mem::take(list), info)?)))
    } else if let Json::Object(Lit(object)) = &mut json {
      let mut result = Json::Null;
      for (key, val) in object.iter_mut() {
        if let Some(builtin) = self.builtin.get_mut(key) {
          let scoped = builtin.scoped;
          let func = builtin.func;
          let mut tmp = ScopeInfo::default();
          if scoped {
            self.vars.push(HashMap::new());
            scope_begin(&mut tmp, info)?;
          }
          let args = if let Json::Array(Lit(arr)) = &mut val.value {
            let raw_args = mem::take(arr);
            if self.builtin.get(key).is_some_and(|built| built.skip_eval) {
              raw_args
            } else {
              self.eval_args(raw_args, info)?
            }
          } else {
            self.eval_args(vec![mem::take(val)], info)?
          };
          result = func(
            self,
            FuncInfo { name: mem::take(key), pos: mem::take(&mut val.pos), args },
            info,
          )?;
          if let Some((addr, size)) = result.tmp() {
            info.free(addr, size)?;
          }
          if scoped {
            scope_end(&mut tmp, info)?;
            self.vars.pop();
          }
        } else if let Ok(Json::Function(func)) = self.get_var(key, &val.pos) {
          info.body.push(mn("call", &[&format!(".L{:x}", func.name)]));
          result = if let Json::Int(_) = *func.ret {
            let name = info.get_tmp(8)?;
            info.body.push(mn("mov", &[&format!("qword{name}"), "rax"]));
            Json::Int(Var(name))
          } else {
            Json::Null
          }
        } else {
          return err!(self, val.pos, "The `{key}` function is undefined.");
        }
      }
      Ok(result)
    } else {
      Ok(json)
    }
  }
  /// Evaluate arguments.
  fn eval_args(
    &mut self, mut args: Vec<JsonWithPos>, info: &mut ScopeInfo,
  ) -> ErrOR<Vec<JsonWithPos>> {
    for arg in &mut args {
      let with_pos = mem::take(arg);
      arg.pos = with_pos.pos;
      arg.value = self.eval(with_pos.value, info)?;
    }
    Ok(args)
  }
  /// Generates a unique name for internal use.
  pub(crate) fn get_global(&mut self, name: &GlobalKind, value: &str) -> ErrOR<Name> {
    let seed = self.global_seed;
    match name {
      GlobalKind::Bss => self.bss.push(format!("  .lcomm .L{seed:x}, {value}\n")),
      GlobalKind::Str => {
        if let Some(str_seed) = self.str_cache.get(value) {
          return Ok(Name { var: Global, seed: *str_seed });
        }
        self.str_cache.insert(value.to_owned(), seed);
        self.data.push(format!("  .L{seed:x}: .string \"{value}\"\n"));
      }
      GlobalKind::Int | GlobalKind::Float => {
        self.data.push(format!("  .align 8\n  .L{seed:x}: .quad {value}\n"));
      }
      GlobalKind::Func => (),
    }
    self.global_seed = add(self.global_seed, 1)?;
    Ok(Name { var: Global, seed })
  }
  /// Gets variable.
  pub(crate) fn get_var(&self, var_name: &str, pos: &Position) -> ErrOR<Json> {
    for scope in self.vars.iter().rev() {
      if let Some(val) = scope.get(var_name) {
        return Ok(val.clone());
      }
    }
    err!(self, pos, "Undefined variables: `{var_name}`")
  }
  /// Registers a function in the function table.
  pub(crate) fn register(&mut self, name: &str, flag: (bool, bool), func: JFunc) {
    self.builtin.insert(name.into(), Builtin { skip_eval: flag.0, scoped: flag.1, func });
  }
}
