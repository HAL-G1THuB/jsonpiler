//! Implementation of the compiler inside the `Jsonpiler`.
use {
  super::{
    AsmFunc, BuiltinFunc, ErrOR, ErrorInfo, FResult, FuncInfo, JFunc, JObject, JResult, JValue,
    Json, Jsonpiler, Section,
  },
  std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{self, Write as _},
  },
};
/// Generate error.
macro_rules! err {
  ($self:ident, $info: expr, $($arg: tt)*) => {
    Err($self.fmt_err(&format!($($arg)*), &$info).into())
  };
}
impl Jsonpiler {
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
    self.builtin = HashMap::new();
    self.vars = vec![HashMap::new()];
    self.all_register();
    let mut func = FuncInfo::default();
    let result = self.eval(&json, &mut func)?;
    let mut writer = io::BufWriter::new(File::create(filename)?);
    writer.write_all(format!(".file \"{json_file}\"\n.intel_syntax noprefix\n").as_bytes())?;
    writer.write_all(include_bytes!("asm/sect/data.s"))?;
    for data in &mut self.sect.data {
      writer.write_all(data.as_bytes())?;
      writer.write_all(b"\n")?;
    }
    writer.write_all(include_bytes!("asm/sect/bss.s"))?;
    for bss in &mut self.sect.bss {
      writer.write_all(bss.as_bytes())?;
      writer.write_all(b"\n")?;
    }
    writer.write_all(include_bytes!("asm/sect/start.s"))?;
    writer.write_all(
      format!(include_str!("asm/common/prologue.s"), size = func.calc_alloc(8)?).as_bytes(),
    )?;
    writer.write_all(b"\n")?;
    writer.write_all(include_bytes!("asm/sect/startup.s"))?;
    for body in &mut func.body {
      writer.write_all(body.as_bytes())?;
      writer.write_all(b"\n")?;
    }
    if let JValue::LInt(int) = result.value {
      writer.write_all(format!("  mov rcx, {int}\n").as_bytes())
    } else if let JValue::VInt(var) = &result.value {
      writer.write_all(format!("  mov rcx, qword ptr {var}[rip]\n").as_bytes())
    } else {
      writer.write_all(b"  xor ecx, ecx\n")
    }?;
    writer.write_all(b"  call [qword ptr __imp_ExitProcess[rip]]\n.seh_endproc\n")?;
    writer.write_all(include_bytes!("asm/sect/handler.s"))?;
    for text in &mut self.sect.text {
      writer.write_all(text.as_bytes())?;
      writer.write_all(b"\n")?;
    }
    writer.flush()?;
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
    let Some(first_elem) = list.first() else {
      self.require("A function call", "list with name of function as the first element", json)?;
      return Ok(Json::default());
    };
    let first = &self.eval(first_elem, func)?;
    if let JValue::LString(cmd) = &first.value {
      if self.builtin.contains_key(cmd.as_str()) {
        let builtin = self.builtin.get_mut(cmd.as_str()).ok_or(ERR)?;
        let scoped = builtin.scoped;
        let ptr = builtin.func;
        if scoped {
          self.vars.push(HashMap::new());
        }
        let rest = list.get(1..).unwrap_or(&[]);
        let args = if builtin.do_not_eval { rest } else { &self.eval_args(rest, func)? };
        let result = Ok(Json { value: ptr(self, first, args, func)?, info: first.info.clone() });
        if scoped {
          self.vars.pop();
        }
        result
      } else {
        Err(self.fmt_err(&format!("Function `{cmd}` is undefined."), &first.info).into())
      }
    } else if let JValue::Function(AsmFunc { name: n, ret: re, .. }) = &first.value {
      func.body.push(format!("  call {n}"));
      if let JValue::VInt(_) | JValue::LInt(_) = **re {
        let na = self.get_name("INT")?;
        self.sect.bss.push(format!("  .lcomm {na}, 8"));
        func.body.push(format!("  mov qword ptr {na}[rip], rax"));
        Ok(Json { value: JValue::VInt(na), info: first.info.clone() })
      } else {
        Ok(Json::default())
      }
    } else {
      self.require("1st element of a function call", "function or built-in function", json)?;
      Ok(Json::default())
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
      .map_or(Ok(0), |current| current.checked_add(1).ok_or("Seed Overflow"))?;
    self.symbol_seeds.insert(name.to_owned(), seed);
    Ok(format!(".L{name}{seed:x}"))
  }
  /// Registers a function in the function table.
  pub(crate) fn register(&mut self, name: &str, flg: (bool, bool), fu: JFunc) {
    self.builtin.insert(name.into(), BuiltinFunc { do_not_eval: flg.0, scoped: flg.1, func: fu });
  }
  /// Generate an error.
  pub(crate) fn require(&self, name: &str, expected: &str, json: &Json) -> FResult {
    err!(self, &json.info, "{name} requires {expected}, but got {}.", &json.value)
  }
  /// Convert `JValue::` (`StringVar` or `String`) to `StringVar`, otherwise return `Err`
  pub(crate) fn string2var(&mut self, json: &Json, ctx: &str) -> ErrOR<String> {
    if let JValue::LString(st) = &json.value {
      let name = self.get_name("STR")?;
      self.sect.data.push(format!("  {name}: .string \"{st}\""));
      Ok(name)
    } else if let JValue::VString(var) = &json.value {
      Ok(var.clone())
    } else {
      err!(self, &json.info, "`{ctx}` must be a string.")
    }
  }
  /// Generate an error.
  pub(crate) fn validate(
    &self, name: &str, at_least: bool, expected: usize, got: usize, info: &ErrorInfo,
  ) -> ErrOR<()> {
    let plural = if expected == 1 { "" } else { "s" };
    if at_least {
      if got >= expected {
        Ok(())
      } else {
        err!(self, info, "`{name}` requires at least {expected} argument{plural}, but got {got}.")
      }
    } else if expected == got {
      Ok(())
    } else {
      err!(self, info, "`{name}` requires exactly {expected} argument{plural}, but got {got}.")
    }
  }
}
