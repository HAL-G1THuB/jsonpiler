//! Implementation of the compiler inside the `Jsonpiler`.
use super::{
  Args, AsmFunc, Bind, Builtin, ErrOR, FuncInfo, GVar, JFunc, JObject, JResult, Json, JsonWithPos,
  Jsonpiler, Name, Position,
  Var::{Global, Tmp},
  add, err,
  utility::{imp_call, mn, scope_begin, scope_end},
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
    self.vars = vec![HashMap::new()];
    self.all_register();
    let mut info = FuncInfo::default();
    let result = self.eval(json, &mut info)?;
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
    write!(writer, include_str!("asm/common/prologue.s"), size = info.calc_alloc(8)?)?;
    writer.write_all(include_bytes!("asm/once/startup.s"))?;
    for body in &mut info.body {
      writer.write_all(body.as_bytes())?;
    }
    if let Json::Int(int) = result.value {
      match int {
        Bind::Lit(l_int) => writer.write_all(mn("mov", &["rcx", &l_int.to_string()]).as_bytes()),
        Bind::Var(name) => {
          writer.write_all(mn("mov", &["rcx", &format!("qword{name}")]).as_bytes())
        }
      }
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
  /// Evaluates a JSON object.
  pub(crate) fn eval(&mut self, mut json: JsonWithPos, info: &mut FuncInfo) -> JResult {
    const ERR: &str = "Unreachable (eval)";
    let Json::Array(Bind::Lit(list)) = &mut json.value else {
      let Json::Object(Bind::Lit(object)) = &mut json.value else { return Ok(json) };
      let mut evaluated = JObject::default();
      for kv in object.iter_mut() {
        evaluated.insert(mem::take(&mut kv.0), self.eval(mem::take(&mut kv.1), info)?);
      }
      return Ok(JsonWithPos { value: Json::Object(Bind::Lit(evaluated)), pos: json.pos });
    };
    self.validate_args("function call", true, 1, list.len(), &json.pos)?;
    let raw_first = mem::take(list.first_mut().ok_or(ERR)?);
    let first = self.eval(raw_first.clone(), info)?;
    if let Json::String(Bind::Lit(cmd)) = &first.value {
      if self.builtin.contains_key(cmd) {
        let builtin = self.builtin.get_mut(cmd).ok_or(ERR)?;
        let scoped = builtin.scoped;
        let func = builtin.func;
        let mut tmp = FuncInfo::default();
        if scoped {
          self.vars.push(HashMap::new());
          scope_begin(&mut tmp, info)?;
        }
        let mut args = list.get_mut(1..).unwrap_or(&mut []).to_vec();
        if !builtin.skip_eval {
          args = self.eval_args(args, info)?;
        }
        let result = func(self, &first, args, info)?;
        if scoped {
          scope_end(&mut tmp, info)?;
          self.vars.pop();
        }
        Ok(JsonWithPos { value: result, pos: json.pos })
      } else if let Ok(Json::Function(af)) = self.get_var(cmd, &first.pos) {
        call_func(json.pos, &af, info)
      } else {
        err!(self, first.pos, "The `{cmd}` function is undefined.")
      }
    } else if let Json::Function(af) = &first.value {
      call_func(json.pos, af, info)
    } else {
      self.typ_err(1, "function call", "LString` or `Function", &raw_first)?;
      Ok(JsonWithPos::default())
    }
  }
  /// Evaluate arguments.
  fn eval_args(&mut self, mut args: Args, info: &mut FuncInfo) -> ErrOR<Vec<JsonWithPos>> {
    for arg in &mut args {
      *arg = self.eval(mem::take(arg), info)?;
    }
    Ok(args)
  }
  /// Generates a unique name for internal use.
  pub(crate) fn get_global(&mut self, name: &GVar, value: &str) -> ErrOR<Name> {
    let seed = self.global_seed;
    match name {
      GVar::Bss => self.bss.push(format!("  .lcomm .L{seed:x}, {value}\n")),
      GVar::Str => {
        if let Some(str_seed) = self.str_cache.get(value) {
          return Ok(Name { var: Global, seed: *str_seed });
        }
        self.str_cache.insert(value.to_owned(), seed);
        self.data.push(format!("  .L{seed:x}: .string \"{value}\"\n"));
      }
      GVar::Int => self.data.push(format!("  .L{seed:x}: .quad {value}\n")),
      GVar::Fnc => (),
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
/// Call function and return value.
fn call_func(pos: Position, af: &AsmFunc, info: &mut FuncInfo) -> ErrOR<JsonWithPos> {
  const MOV: &str = "  mov ";
  info.body.push(format!("  call .L{:x}\n", af.name));
  if let Json::Int(_) = *af.ret {
    let name = info.get_local(8)?;
    info.body.push(format!("{MOV}qword{name}, rax\n"));
    Ok(JsonWithPos { value: Json::Int(Bind::Var(Name { var: Tmp, seed: name.seed })), pos })
  } else {
    Ok(JsonWithPos { value: Json::Null, pos })
  }
}
