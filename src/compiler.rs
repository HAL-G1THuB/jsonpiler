//! Implementation of the compiler inside the `Jsonpiler`.
use super::{
  Align, Args, AsmFunc, Bind, Builtin, ErrOR, FuncInfo, JFunc, JObject, JResult, Json, JsonWithPos,
  Jsonpiler, Position, Section, err,
  utility::{fmt_local, scope_begin, scope_end},
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
  pub fn build(&mut self, source: String, json_file: &str, out_file: &str) -> ErrOR<()> {
    let json = self.parse(source)?;
    self.include_flag = HashSet::new();
    self.sect = Section::default();
    self.symbol_seeds = HashMap::new();
    self.builtin = HashMap::new();
    self.vars = vec![HashMap::new()];
    self.all_register();
    let mut info = FuncInfo::default();
    let result = self.eval(json, &mut info)?;
    let mut writer = io::BufWriter::new(File::create(out_file)?);
    writeln!(writer, ".file \"{json_file}\"\n.intel_syntax noprefix")?;
    writer.write_all(include_bytes!("asm/once/data.s"))?;
    for data in &mut self.sect.data {
      writer.write_all(data.as_bytes())?;
    }
    writer.write_all(include_bytes!("asm/once/bss.s"))?;
    for bss in &mut self.sect.bss {
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
        Bind::Lit(l_int) => writeln!(writer, "  mov rcx, {l_int}"),
        Bind::Var(var) => writeln!(writer, "  mov rcx, {var}"),
        Bind::Local(local) | Bind::Tmp(local) => {
          writeln!(writer, "  mov rcx, {}", fmt_local("qword", local))
        }
      }
    } else {
      writeln!(writer, "  xor ecx, ecx")
    }?;
    writer.write_all(b"  call [qword ptr __imp_ExitProcess[rip]]\n.seh_endproc\n")?;
    for text in &mut self.sect.text {
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
    let first_elem = mem::take(list.first_mut().ok_or(ERR)?);
    let first = self.eval(first_elem.clone(), info)?;
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
        Ok(JsonWithPos { value: result, pos: first_elem.pos })
      } else if let Ok(Json::Function(af)) = self.get_var(cmd, &first.pos) {
        call_func(first_elem.pos, &af, info)
      } else {
        err!(self, first.pos, "The `{cmd}` function is undefined.")
      }
    } else if let Json::Function(af) = &first.value {
      call_func(first_elem.pos, af, info)
    } else {
      self.typ_err(1, "function call", "LString` or `Function", &first_elem)?;
      Ok(JsonWithPos::default())
    }
  }
  /// Evaluate arguments.
  fn eval_args(&mut self, args: Args, info: &mut FuncInfo) -> ErrOR<Vec<JsonWithPos>> {
    let mut result = vec![];
    for arg in args {
      result.push(self.eval(arg, info)?);
    }
    Ok(result)
  }
  /// Generates a unique name for internal use.
  pub(crate) fn get_global(&mut self, name: &str, value: &str) -> ErrOR<String> {
    let seed = self
      .symbol_seeds
      .get(name)
      .map_or(Ok(0), |current| current.checked_add(1).ok_or("Seed Overflow"))?;
    self.symbol_seeds.insert(name.to_owned(), seed);
    let l_name = format!(".L{name}{seed:x}");
    let name_fmt = format!("qword ptr {l_name}[rip]");
    match name {
      "BSS" => self.sect.bss.push(format!("  .lcomm {l_name}, {value}\n")),
      "STR" => {
        if let Some(str_seed) = self.str_cache.get(value) {
          return Ok(format!("qword ptr .LSTR{str_seed:x}[rip]"));
        }
        self.str_cache.insert(value.to_owned(), self.symbol_seeds.len());
        self.sect.data.push(format!("  {l_name}: .string \"{value}\"\n"));
      }
      "INT" => self.sect.data.push(format!("  {l_name}: .quad {value}\n")),
      "FNC" => return Ok(l_name),
      _ => return Err(format!("Internal Error: Unrecognized name: {name}").into()),
    }
    Ok(name_fmt)
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
  pub(crate) fn register(&mut self, name: &str, flag: (bool, bool), j_func: JFunc) {
    self.builtin.insert(name.into(), Builtin { skip_eval: flag.0, scoped: flag.1, func: j_func });
  }
  /// Converts `Json` (`StringVar` or `String`) to `StringVar`, otherwise return `Err`
  pub(crate) fn string2var(
    &mut self, json: JsonWithPos, ordinal: usize, func_name: &str,
  ) -> ErrOR<String> {
    if let Json::String(st) = json.value {
      match st {
        Bind::Lit(l_str) => self.get_global("STR", &l_str),
        Bind::Var(var) => Ok(var),
        Bind::Local(local) | Bind::Tmp(local) => Ok(fmt_local("qword", local)),
      }
    } else {
      self.typ_err(ordinal, func_name, "String", &json)?;
      Ok(String::new())
    }
  }
}
/// Call function and return value.
fn call_func(pos: Position, af: &AsmFunc, info: &mut FuncInfo) -> ErrOR<JsonWithPos> {
  info.body.push(format!("  call {}\n", af.name));
  if let Json::Int(_) = *af.ret {
    let offset = info.get_local(Align::U64)?;
    info.body.push(format!("  mov {}, rax\n", fmt_local("qword", offset)));
    Ok(JsonWithPos { value: Json::Int(Bind::Tmp(offset)), pos })
  } else {
    Ok(JsonWithPos { value: Json::Null, pos })
  }
}
