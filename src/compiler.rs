//! Implementation of the compiler inside the `Jsonpiler`.
use {
  super::{
    AsmFunc, BuiltinFunc, ErrOR, ErrorInfo, FuncInfo, JFunc, JObject, JResult, JValue, Json,
    Jsonpiler, Section,
  },
  core::fmt::Write as _,
  std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{self, Write as _},
  },
};
impl Jsonpiler {
  /// Assert condition.
  pub(crate) fn assert(&self, cond: bool, text: &str, info: &ErrorInfo) -> ErrOR<()> {
    cond.then_some(()).ok_or_else(|| self.fmt_err(text, info).into())
  }
  /// Builds the assembly code from the parsed JSON.
  /// This function is the main entry point for the compilation process.
  /// It takes the parsed JSON, sets up the initial function table,
  /// evaluates the JSON, and writes the resulting assembly code to a file.
  /// # Arguments
  /// * `source` - The JSON String.
  /// * `json_file` - The name of the original JSON file.
  /// * `filename` - The name of the file to write the assembly code to.
  /// # Returns
  /// * `Ok(Json)` - The result of the evaluation.
  /// * `Err(Box<dyn Error>)` - If an error occurred during the compilation process.
  /// # Errors
  /// * `Box<dyn Error>` - If an error occurred during the compilation process.
  #[inline]
  pub fn build(&mut self, source: String, json_file: &str, filename: &str) -> ErrOR<()> {
    let json = self.parse(source)?;
    self.include_flag = HashSet::new();
    self.sect = Section::default();
    self.symbol_seeds = HashMap::new();
    self.f_table = HashMap::new();
    self.vars = vec![HashMap::new()];
    self.all_register();
    let mut start = FuncInfo::default();
    let result = self.eval(&json, &mut start)?;
    if let JValue::LInt(int) = result.value {
      writeln!(start.body, "  mov rcx, {int}")?;
    } else if let JValue::VInt(var) = &result.value {
      writeln!(start.body, "mov rcx, qword ptr {var}[rip]")?;
    } else {
      start.body.push_str("xor ecx, ecx\n");
    }
    start.body.push_str("  call [qword ptr __imp_ExitProcess[rip]]\n  .seh_endproc\n");
    self.write_file(&start.body, filename, json_file)?;
    Ok(())
  }
  /// Evaluates a JSON object.
  pub(crate) fn eval(&mut self, json: &Json, func: &mut FuncInfo) -> JResult {
    const ERR: &str = "Unreachable (eval)";
    let JValue::LArray(list) = &json.value else {
      let JValue::LObject(object) = &json.value else { return Ok(json.clone()) };
      let mut evaluated = JObject::default();
      for kv in object.iter() {
        evaluated.insert(kv.0.clone(), self.eval(&kv.1, func)?);
      }
      return Ok(Json { value: JValue::LObject(evaluated), info: json.info.clone() });
    };
    let first_elem =
      list.first().ok_or(self.fmt_err("An function call cannot be an empty list.", &json.info))?;
    let first = &self.eval(first_elem, func)?;
    if let JValue::LString(cmd) = &first.value {
      if self.f_table.contains_key(cmd.as_str()) {
        if self.f_table.get_mut(cmd.as_str()).ok_or(ERR)?.scoped {
          self.vars.push(HashMap::new());
        }
        let args = if self.f_table.get_mut(cmd.as_str()).ok_or(ERR)?.do_not_eval {
          list.get(1..).unwrap_or(&[])
        } else {
          &self.eval_args(list.get(1..).unwrap_or(&[]), func)?
        };
        let result = Ok(Json {
          value: (self.f_table.get_mut(cmd.as_str()).ok_or(ERR)?.func)(self, first, args, func)?,
          info: first.info.clone(),
        });
        if self.f_table.get_mut(cmd.as_str()).ok_or(ERR)?.scoped {
          self.vars.pop();
        }
        result
      } else {
        Err(self.fmt_err(&format!("Function '{cmd}' is undefined."), &first.info).into())
      }
    } else if let JValue::Function(AsmFunc { name: n, ret: re, .. }) = &first.value {
      writeln!(func.body, "  call {n}")?;
      if let JValue::VInt(_) | JValue::LInt(_) = **re {
        let na = self.get_name("INT")?;
        writeln!(self.sect.bss, "  .lcomm {na}, 8")?;
        writeln!(func.body, "  mov qword ptr {na}[rip], rax")?;
        Ok(Json { value: JValue::VInt(na), info: first.info.clone() })
      } else {
        Ok(Json::default())
      }
    } else {
      Err(self.fmt_err("Expected a function or lambda as the first element.", &json.info).into())
    }
  }
  /// Evaluate arguments.
  fn eval_args(&mut self, args: &[Json], function: &mut FuncInfo) -> ErrOR<Vec<Json>> {
    let mut result = vec![];
    for arg in args {
      result.push(self.eval(arg, function)?);
    }
    Ok(result)
  }
  /// Generates a unique name for internal use.
  pub(crate) fn get_name(&mut self, name: &str) -> Result<String, &'static str> {
    let seed = self
      .symbol_seeds
      .get(name)
      .map_or(Ok(0), |current| current.checked_add(1).ok_or("SeedOverflowError"))?;
    self.symbol_seeds.insert(name.to_owned(), seed);
    Ok(format!(".L{name}{seed:x}"))
  }
  /// Registers a function in the function table.
  pub(crate) fn register(&mut self, name: &str, flg: (bool, bool), fu: JFunc) {
    self.f_table.insert(name.into(), BuiltinFunc { do_not_eval: flg.0, scoped: flg.1, func: fu });
  }
  /// Convert `JValue::` (`StringVar` or `String`) to `StringVar`, otherwise return `Err`
  pub(crate) fn string2var(&mut self, json: &Json, ctx: &str) -> ErrOR<String> {
    if let JValue::LString(st) = &json.value {
      let name = self.get_name("STR")?;
      writeln!(self.sect.data, "  {name}: .string \"{st}\"")?;
      Ok(name)
    } else if let JValue::VString(var) = &json.value {
      Ok(var.clone())
    } else {
      Err(self.fmt_err(&format!("'{ctx}' must be a string."), &json.info).into())
    }
  }
  /// Writes the compiled assembly code to a file.
  fn write_file(&self, start: &str, filename: &str, json_file: &str) -> io::Result<()> {
    let mut writer = io::BufWriter::new(File::create(filename)?);
    writer.write_all(format!(".file \"{json_file}\"\n.intel_syntax noprefix\n").as_bytes())?;
    writer.write_all(include_bytes!("asm/sect/data.s"))?;
    writer.write_all(self.sect.data.as_bytes())?;
    writer.write_all(include_bytes!("asm/sect/bss.s"))?;
    writer.write_all(self.sect.bss.as_bytes())?;
    writer.write_all(include_bytes!("asm/sect/start.s"))?;
    writer.write_all(start.as_bytes())?;
    writer.write_all(include_bytes!("asm/sect/text.s"))?;
    writer.write_all(self.sect.text.as_bytes())?;
    writer.flush()?;
    Ok(())
  }
}
