//! Implementation of the compiler inside the `Jsonpiler`.
use super::{
  AsmBool,
  Bind::{Lit, Var},
  Builtin, ErrOR, FuncInfo, JFunc, Json, JsonWithPos, Jsonpiler, Name, Position, ScopeInfo,
  VarKind::Global,
  add, err, mn, mn_write,
  utility::{get_int_str, imp_call, scope_begin, scope_end},
};
use core::mem::take;
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
    self.label_id = 0;
    self.builtin = HashMap::new();
    self.vars_global = HashMap::new();
    self.vars_local = vec![HashMap::new()];
    self.all_register();
    let mut scope = ScopeInfo::default();
    let result = self.eval(json.value, &mut scope)?;
    let mut writer = io::BufWriter::new(File::create(out_file)?);
    mn_write!(&mut writer, ".intel_syntax", "noprefix")?;
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
      size = format!("{:#x}", scope.calc_alloc(8)?)
    )?;
    writer.write_all(include_bytes!("asm/once/startup.s"))?;
    for body in &mut scope.body {
      writer.write_all(body.as_bytes())?;
    }
    if let Json::Int(int) = result {
      mn_write!(&mut writer, "mov", "rcx", &get_int_str(&int, &mut scope)?)
    } else {
      mn_write!(&mut writer, "xor", "ecx", "ecx")
    }?;
    writer.write_all(imp_call("ExitProcess").as_bytes())?;
    mn_write!(&mut writer, ".seh_endproc")?;
    for text in &mut self.text {
      writer.write_all(text.as_bytes())?;
    }
    writer.write_all(include_bytes!("asm/once/handler.s"))?;
    writer.flush()?;
    Ok(())
  }
  /// Evaluates JSON representation.
  pub(crate) fn eval(&mut self, mut json: Json, scope: &mut ScopeInfo) -> ErrOR<Json> {
    if let Json::Array(Lit(list)) = &mut json {
      Ok(Json::Array(Lit(self.eval_args(take(list), scope)?)))
    } else if let Json::Object(Lit(object)) = &mut json {
      let mut result = Json::Null;
      for (key, val) in object.iter_mut() {
        if let Some(builtin) = self.builtin.get_mut(key) {
          let scoped = builtin.scoped;
          let func = builtin.func;
          let mut tmp = ScopeInfo::default();
          if scoped {
            self.vars_local.push(HashMap::new());
            scope_begin(&mut tmp, scope)?;
          }
          let args = if let Json::Array(Lit(arr)) = &mut val.value {
            let raw_args = take(arr);
            if self.builtin.get(key).is_some_and(|built| built.skip_eval) {
              raw_args
            } else {
              self.eval_args(raw_args, scope)?
            }
          } else {
            self.eval_args(vec![take(val)], scope)?
          };
          result = func(self, FuncInfo { name: take(key), pos: take(&mut val.pos), args }, scope)?;
          if let Some((addr, size)) = result.tmp() {
            scope.free(addr, size)?;
          }
          if scoped {
            scope_end(&mut tmp, scope)?;
            self.vars_local.pop();
          }
        } else if let Ok(Json::Function(asm_func)) = self.get_var(key, &val.pos) {
          scope.body.push(mn!("call", asm_func.name.to_ref()));
          result = if let Json::Int(_) = *asm_func.ret {
            let name = scope.get_tmp(8)?;
            scope.body.push(mn!("mov", &format!("qword{name}"), "rax"));
            Json::Int(Var(name))
          } else {
            Json::Null
          }
        } else {
          return err!(self, val.pos, "Undefined function: `{key}`");
        }
      }
      Ok(result)
    } else {
      Ok(json)
    }
  }
  /// Evaluate arguments.
  fn eval_args(
    &mut self, mut args: Vec<JsonWithPos>, scope: &mut ScopeInfo,
  ) -> ErrOR<Vec<JsonWithPos>> {
    for arg in &mut args {
      let with_pos = take(arg);
      arg.pos = with_pos.pos;
      arg.value = self.eval(with_pos.value, scope)?;
    }
    Ok(args)
  }
  /// Gets a global boolean.
  pub(crate) fn get_global_bool(&mut self) -> ErrOR<AsmBool> {
    for (&addr, bits) in &mut self.global_bool_map {
      for bit in 0u8..8u8 {
        if *bits & (1 << bit) == 0 {
          *bits |= 1 << bit;
          let name = Name { var: Global, id: addr };
          return Ok(AsmBool { bit, name });
        }
      }
    }
    let name = self.get_global_bss(1)?;
    let abs_addr = name.id;
    self.global_bool_map.insert(abs_addr, 0b0000_0001);
    Ok(AsmBool { name, bit: 0 })
  }
  /// Generates a unique name for internal use.
  pub(crate) fn get_global_bss(&mut self, value: u8) -> ErrOR<Name> {
    let label = self.get_global_label()?;
    self.bss.push(mn!(".lcomm", label.to_ref(), value.to_string()));
    Ok(label)
  }
  /// Generates a unique name for internal use.
  pub(crate) fn get_global_float(&mut self, value: u64) -> ErrOR<Name> {
    let label = self.get_global_label()?;
    self.data.push(mn!(".align", "8"));
    self.data.push(label.to_def());
    self.data.push(mn!(".quad", format!("{value:#x}")));
    Ok(label)
  }
  /// Generates a unique name for internal use.
  pub(crate) fn get_global_int(&mut self, value: i64) -> ErrOR<Name> {
    let label = self.get_global_label()?;
    self.data.push(mn!(".align", "8"));
    self.data.push(label.to_def());
    self.data.push(mn!(".quad", format!("{value:#x}")));
    Ok(label)
  }
  /// Generates a unique name for internal use.
  pub(crate) fn get_global_label(&mut self) -> ErrOR<Name> {
    let label = self.label_id;
    self.label_id = add(label, 1)?;
    Ok(Name { id: label, var: Global })
  }
  /// Generates a unique name for internal use.
  pub(crate) fn get_global_str(&mut self, value: &str) -> ErrOR<Name> {
    if let Some(cached_label) = self.str_cache.get(value) {
      return Ok(Name { var: Global, id: *cached_label });
    }
    let label = self.get_global_label()?;
    self.str_cache.insert(value.to_owned(), label.id);
    self.data.push(label.to_def());
    self.data.push(mn!(".string", format!("\"{value}\"")));
    Ok(label)
  }
  /// Gets variable.
  pub(crate) fn get_var(&self, var_name: &str, pos: &Position) -> ErrOR<Json> {
    for scope in self.vars_local.iter().rev() {
      if let Some(val) = scope.get(var_name) {
        return Ok(val.clone());
      }
    }
    if let Some(val) = self.vars_global.get(var_name) {
      return Ok(val.clone());
    }
    err!(self, pos, "Undefined variables: `{var_name}`")
  }
  /// Registers a function in the function table.
  pub(crate) fn register(&mut self, name: &str, (scoped, skip_eval): (bool, bool), func: JFunc) {
    self.builtin.insert(name.into(), Builtin { func, scoped, skip_eval });
  }
}
