//! Implementation of the compiler inside the `Jsonpiler`.
use super::{
  Args, Builtin, ErrOR, FResult, FuncInfo, JFunc, JObject, JResult, Json, JsonWithPos, Jsonpiler,
  Position, Section, err,
};
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
    let result = self.eval(&json, &mut info)?;
    let mut writer = io::BufWriter::new(File::create(out_file)?);
    writer.write_all(format!(".file \"{json_file}\"\n.intel_syntax noprefix\n").as_bytes())?;
    writer.write_all(include_bytes!("asm/sect/data.s"))?;
    for data in &mut self.sect.data {
      writer.write_all(data.as_bytes())?;
    }
    writer.write_all(include_bytes!("asm/sect/bss.s"))?;
    for bss in &mut self.sect.bss {
      writer.write_all(bss.as_bytes())?;
    }
    writer.write_all(include_bytes!("asm/sect/start.s"))?;
    writer.write_all(
      format!(include_str!("asm/common/prologue.s"), size = info.calc_alloc(8)?).as_bytes(),
    )?;
    writer.write_all(include_bytes!("asm/sect/startup.s"))?;
    for body in &mut info.body {
      writer.write_all(body.as_bytes())?;
    }
    if let Json::LInt(int) = result.value {
      writer.write_all(format!("  mov rcx, {int}\n").as_bytes())
    } else if let Json::VInt(var) = &result.value {
      writer.write_all(format!("  mov rcx, {var}\n").as_bytes())
    } else {
      writer.write_all(b"  xor ecx, ecx\n")
    }?;
    writer.write_all(b"  call [qword ptr __imp_ExitProcess[rip]]\n.seh_endproc\n")?;
    writer.write_all(include_bytes!("asm/sect/handler.s"))?;
    for text in &mut self.sect.text {
      writer.write_all(text.as_bytes())?;
    }
    writer.flush()?;
    Ok(())
  }
  /// Evaluates a JSON object.
  pub(crate) fn eval(&mut self, json: &JsonWithPos, info: &mut FuncInfo) -> JResult {
    const ERR: &str = "Unreachable (eval)";
    let Json::LArray(list) = &json.value else {
      let Json::LObject(object) = &json.value else { return Ok(json.clone()) };
      let mut evaluated = JObject::default();
      for kv in object.iter() {
        evaluated.insert(kv.0.clone(), self.eval(&kv.1, info)?);
      }
      return Ok(JsonWithPos { value: Json::LObject(evaluated), pos: json.pos.clone() });
    };
    self.validate_args("function call", true, 1, list.len(), &json.pos)?;
    let first_elem = list.first().ok_or(ERR)?;
    let first = &self.eval(first_elem, info)?;
    if let Json::LString(cmd) = &first.value {
      if let Ok(Json::Function(af)) = self.get_var(cmd, &first.pos) {
        info.body.push(format!("  call {}\n", af.name));
        if let Json::VInt(_) | Json::LInt(_) = *af.ret {
          let name = self.get_name("BSS", "8")?;
          info.body.push(format!("  mov {name}, rax\n"));
          Ok(JsonWithPos { value: Json::VInt(name), pos: first.pos.clone() })
        } else {
          Ok(JsonWithPos::default())
        }
      } else if self.builtin.contains_key(cmd.as_str()) {
        let builtin = self.builtin.get_mut(cmd.as_str()).ok_or(ERR)?;
        let scoped = builtin.scoped;
        let func = builtin.func;
        if scoped {
          self.vars.push(HashMap::new());
        }
        let rest = list.get(1..).unwrap_or(&[]);
        let args = if builtin.skip_eval { rest } else { &self.eval_args(rest, info)? };
        let result = func(self, first, args, info)?;
        if scoped {
          self.vars.pop();
        }
        Ok(JsonWithPos { value: result, pos: first.pos.clone() })
      } else {
        Err(self.fmt_err(&format!("The `{cmd}` function is undefined."), &first.pos).into())
      }
    } else if let Json::Function(af) = &first.value {
      info.body.push(format!("  call {}", af.name));
      if let Json::VInt(_) | Json::LInt(_) = *af.ret {
        let name = self.get_name("BSS", "8")?;
        info.body.push(format!("  mov {name}, rax\n"));
        Ok(JsonWithPos { value: Json::VInt(name), pos: first.pos.clone() })
      } else {
        Ok(JsonWithPos::default())
      }
    } else {
      self.typ_err(1, "function call", "LString` or `Function", first_elem)?;
      Ok(JsonWithPos::default())
    }
  }
  /// Evaluate arguments.
  fn eval_args(&mut self, args: &Args, info: &mut FuncInfo) -> ErrOR<Vec<JsonWithPos>> {
    let mut result = vec![];
    for arg in args {
      result.push(self.eval(arg, info)?);
    }
    Ok(result)
  }
  /// Generates a unique name for internal use.
  pub(crate) fn get_name(&mut self, name: &str, value: &str) -> Result<String, String> {
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
      _ => return Err(format!("Internal Error: Unrecognized name: {name}")),
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
  /// Converts `JValue` (`StringVar` or `String`) to `StringVar`, otherwise return `Err`
  pub(crate) fn string2var(
    &mut self, json: &JsonWithPos, ordinal: usize, func_name: &str,
  ) -> ErrOR<String> {
    if let Json::LString(l_str) = &json.value {
      let name = self.get_name("STR", l_str)?;
      Ok(name)
    } else if let Json::VString(v_str) = &json.value {
      Ok(v_str.clone())
    } else {
      self.typ_err(ordinal, func_name, "String", json)?;
      Ok(String::new())
    }
  }
  /// Generates a type error.
  pub(crate) fn typ_err(
    &self, ordinal: usize, name: &str, expected: &str, json: &JsonWithPos,
  ) -> FResult {
    let suffix = match ordinal % 100 {
      11..=13 => "th",
      _ => match ordinal % 10 {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
      },
    };
    let typ = json.value.type_name();
    err!(
      self,
      &json.pos,
      "The {ordinal}{suffix} argument to `{name}` must be of a type `{expected}`, \
      but a value of type `{typ}` was provided."
    )
  }
  /// Generate an error.
  pub(crate) fn validate_args(
    &self, name: &str, at_least: bool, expected: usize, supplied: usize, pos: &Position,
  ) -> ErrOR<()> {
    let fmt_require = |text: &str| -> ErrOR<()> {
      let (plural, be) = if expected == 1 { ("", "is") } else { ("s", "are") };
      err!(
        self,
        pos,
        "`{name}` requires {text} {expected} argument{plural}, \
        but {supplied} argument{plural} {be} supplied.",
      )
    };
    if at_least {
      if supplied >= expected { Ok(()) } else { fmt_require("at least") }
    } else if expected == supplied {
      Ok(())
    } else {
      fmt_require("exactly")
    }
  }
}
