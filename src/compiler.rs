//! Implementation of the compiler inside the `Jsonpiler`.
use {
  super::{
    AsmFunc, BuiltinFunc, ErrOR, ErrorInfo, FuncInfo, JFunc, JFuncResult, JObject, JResult, JValue,
    Json, Jsonpiler, Section, functions::gen_json,
  },
  core::fmt::Write as _,
  std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{self, Write as _},
  },
};
/// Macro to include assembly files only once.
macro_rules! include_once {
  ($self:ident, $name:literal) => {{
    if !$self.include_flag.contains($name) {
      $self.include_flag.insert($name.into());
      write!($self.sect.text, include_str!(concat!("asm/", $name, ".s")))?;
    }
  }};
}
impl Jsonpiler {
  /// Assert condition.
  fn assert(&self, cond: bool, text: &str, info: &ErrorInfo) -> ErrOR<()> {
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
    self.vars = HashMap::new();
    self.register("=", true, Jsonpiler::f_local_set);
    self.register("$", true, Jsonpiler::f_local_get);
    self.register("+", true, Jsonpiler::f_plus);
    self.register("-", true, Jsonpiler::f_minus);
    self.register("message", true, Jsonpiler::f_message);
    self.register("begin", true, Jsonpiler::f_begin);
    let mut start = FuncInfo::default();
    let result = self.eval(&json, &mut start)?;
    writeln!(
      start.body,
      "  {}
  call [qword ptr __imp_ExitProcess[rip]]
  .seh_endproc",
      if let JValue::LInt(int) = result.value {
        format!("mov rcx, {int}")
      } else if let JValue::VInt(var) = &result.value {
        format!("mov rcx, qword ptr {var}[rip]")
      } else {
        "xor ecx, ecx".into()
      }
    )?;
    self.write_file(&start.body, filename, json_file)?;
    Ok(())
  }
  /// Evaluates a JSON object.
  fn eval(&mut self, json: &Json, func: &mut FuncInfo) -> JResult {
    const ERR: &str = "Unreachable (eval)";
    let JValue::LArray(list) = &json.value else {
      let JValue::LObject(object) = &json.value else { return Ok(json.clone()) };
      let mut evaluated = JObject::default();
      for kv in object.iter() {
        evaluated.insert(kv.0.clone(), self.eval(&kv.1, func)?);
      }
      return Ok(gen_json(JValue::LObject(evaluated), json.info.clone()));
    };
    let first_elem =
      list.first().ok_or(self.fmt_err("An function call cannot be an empty list.", &json.info))?;
    let first = &self.eval(first_elem, func)?;
    if let JValue::LString(cmd) = &first.value {
      if cmd == "lambda" {
        Ok(gen_json(JValue::Function(self.eval_lambda(json)?), first.info.clone()))
      } else if self.f_table.contains_key(cmd.as_str()) {
        let args = if self.f_table.get_mut(cmd.as_str()).ok_or(ERR)?.evaluated {
          &self.eval_args(list.get(1..).unwrap_or(&[]), func)?
        } else {
          list.get(1..).unwrap_or(&[])
        };
        Ok(gen_json(
          (self.f_table.get_mut(cmd.as_str()).ok_or(ERR)?.func)(self, first, args, func)?,
          first.info.clone(),
        ))
      } else {
        Err(self.fmt_err(&format!("Function '{cmd}' is undefined."), &first.info).into())
      }
    } else if let JValue::Function(AsmFunc { name: n, ret: re, .. }) = &first.value {
      writeln!(func.body, "  call {n}")?;
      if let JValue::VInt(_) | JValue::LInt(_) = **re {
        let na = self.get_name("INT")?;
        writeln!(self.sect.bss, "  .lcomm {na}, 8")?;
        writeln!(func.body, "  mov qword ptr {na}[rip], rax")?;
        Ok(gen_json(JValue::VInt(na), first.info.clone()))
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
  /// Evaluates a lambda function definition.
  fn eval_lambda(&mut self, json: &Json) -> ErrOR<AsmFunc> {
    const ERR: &str = "Unreachable (eval_lambda)";
    let tmp = self.vars.clone();
    let mut func = FuncInfo::default();
    let JValue::LArray(func_list) = &json.value else {
      return Err(self.fmt_err("Invalid function definition.", &json.info).into());
    };
    self.assert(func_list.len() >= 3, "Invalid function definition.", &json.info)?;
    let lambda = func_list.first().ok_or(ERR)?;
    self.assert(
      matches!(&lambda.value, JValue::LString(st) if st == "lambda"),
      r#"The first element of a lambda list requires "lambda"."#,
      &lambda.info,
    )?;
    let params_json = func_list.get(1).ok_or(ERR)?;
    let JValue::LArray(params) = &params_json.value else {
      return Err(
        self
          .fmt_err(
            "The second element of a lambda list requires an argument list.",
            &params_json.info,
          )
          .into(),
      );
    };
    self.assert(params.is_empty(), "PARAMS ISN'T IMPLEMENTED.", &params_json.info)?;
    let name = self.get_name("FNC")?;
    let mut ret = JValue::Null;
    for arg in func_list.get(2..).ok_or("Empty lambda body.")? {
      ret = self.eval(arg, &mut func)?.value;
    }
    let mut registers: Vec<&String> = func.using_reg.iter().collect();
    registers.sort();
    writeln!(self.sect.text, ".seh_proc {name}\n{name}:")?;
    for &reg in &registers {
      writeln!(self.sect.text, "  push {reg}\n  .seh_pushreg {reg}")?;
    }
    self.sect.text.push_str(
      "  push rbp
  .seh_pushreg rbp
  mov rbp, rsp
  .seh_setframe rbp, 0
  sub rsp, 32
  .seh_stackalloc 32
  .seh_endprologue
  .seh_handler .L__SEH_HANDLER, @except\n",
    );
    self.sect.text.push_str(&func.body);
    if let JValue::LInt(int) = ret {
      writeln!(self.sect.text, "  mov rax, {int}")?;
    } else if let JValue::VInt(var) = &ret {
      writeln!(self.sect.text, "  mov rax, qword ptr {var}[rip]")?;
    } else {
      self.sect.text.push_str("  xor eax, eax\n");
    }
    self.sect.text.push_str("  add rsp, 32\n  leave\n");
    registers.reverse();
    for reg in &registers {
      writeln!(self.sect.text, "  pop {reg}")?;
    }
    self.sect.text.push_str("  ret\n.seh_endproc\n");
    self.vars = tmp;
    Ok(AsmFunc { name, params: params.clone(), ret: Box::new(ret) })
  }
  /// Evaluates a 'begin' block.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_begin(&mut self, first: &Json, args: &[Json], _: &mut FuncInfo) -> JFuncResult {
    args.last().map_or_else(
      || Err(self.fmt_err("'begin' requires at least one arguments.", &first.info).into()),
      |last| Ok(last.value.clone()),
    )
  }
  /// Utility functions for binary operations
  fn f_binary_op(
    &mut self, first: &Json, args: &[Json], func: &mut FuncInfo, mn: &str, op: &str,
  ) -> JFuncResult {
    let mut f_binary_mn = |json: &Json, mne: &str| -> ErrOR<()> {
      if let JValue::LInt(int) = json.value {
        Ok(writeln!(func.body, "  {mne} rax, {int}")?)
      } else if let JValue::VInt(var) = &json.value {
        Ok(writeln!(func.body, "  {mne} rax, qword ptr {var}[rip]")?)
      } else {
        Err(
          self
            .fmt_err(
              &format!("'{op}' requires integer operands, but got {}", json.value),
              &json.info,
            )
            .into(),
        )
      }
    };
    let operand_r = args
      .first()
      .ok_or(self.fmt_err(&format!("'{op}' requires at least one arguments."), &first.info))?;
    f_binary_mn(operand_r, "mov")?;
    for operand_l in args.get(1..).unwrap_or(&[]) {
      f_binary_mn(operand_l, mn)?;
    }
    let ret = self.get_name("INT")?;
    writeln!(self.sect.bss, "  .lcomm {ret}, 8")?;
    writeln!(func.body, "  mov qword ptr {ret}[rip], rax")?;
    Ok(JValue::VInt(ret))
  }
  /// Gets the value of a local variable.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_local_get(&mut self, first: &Json, args: &[Json], _: &mut FuncInfo) -> JFuncResult {
    self.assert(args.len() == 1, "'$' requires one argument.", &first.info)?;
    let json1 = args.first().ok_or("Unreachable (f_set_local)")?;
    let JValue::LString(var_name) = &json1.value else {
      return Err(self.fmt_err("Variable name must be a string literal.", &json1.info).into());
    };
    match self.vars.get(var_name) {
      Some(value) => Ok(value.clone()),
      None => Err(self.fmt_err(&format!("Undefined variables: '{var_name}'"), &json1.info).into()),
    }
  }
  /// Sets a local variable.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_local_set(&mut self, first: &Json, args: &[Json], _: &mut FuncInfo) -> JFuncResult {
    self.assert(args.len() == 2, "'=' requires two arguments.", &first.info)?;
    let json1 = args.first().ok_or("Unreachable (f_set_local)")?;
    let JValue::LString(variable) = &json1.value else {
      return Err(self.fmt_err("Variable name must be a string literal.", &json1.info).into());
    };
    let json2 = args.get(1).ok_or("Unreachable (f_set_local)")?;
    match &json2.value {
      JValue::LString(st) => {
        let name = self.get_name("STR")?;
        writeln!(self.sect.data, "  {name}: .string \"{st}\"")?;
        self.vars.insert(variable.clone(), JValue::VString(name.clone()))
      }
      JValue::Null => self.vars.insert(variable.clone(), JValue::Null),
      JValue::LInt(int) => {
        let name = self.get_name("INT")?;
        writeln!(self.sect.data, "  {name}: .quad 0x{int:x}")?;
        self.vars.insert(variable.clone(), JValue::VInt(name.clone()))
      }
      JValue::VString(_)
      | JValue::VInt(_)
      | JValue::Function { .. }
      | JValue::VArray(_)
      | JValue::VBool(..)
      | JValue::VFloat(_)
      | JValue::VObject(_) => self.vars.insert(variable.clone(), json2.value.clone()),
      JValue::LArray(_) | JValue::LBool(_) | JValue::LFloat(_) | JValue::LObject(_) => {
        return Err(self.fmt_err("Assignment to an unimplemented type.", &json2.info).into());
      }
    }
    .map_or(Ok(()), |_| Err(self.fmt_err("Reassignment not implemented.", &first.info)))?;
    Ok(JValue::Null)
  }
  /// Displays a message box.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_message(&mut self, first: &Json, args: &[Json], func: &mut FuncInfo) -> JFuncResult {
    self.assert(args.len() == 2, "'message' requires two arguments.", &first.info)?;
    func.using_reg.insert("rdi".into());
    func.using_reg.insert("rsi".into());
    let title = self.string2var(args.first().ok_or("Unreachable (f_message)")?, "title")?;
    let msg = self.string2var(args.get(1).ok_or("Unreachable (f_message)")?, "text")?;
    let ret = self.get_name("INT")?;
    writeln!(self.sect.bss, "  .lcomm {ret}, 8")?;
    include_once!(self, "func/U8TO16");
    write!(func.body, include_str!("asm/caller/message.s"), msg = msg, title = title, ret = ret,)?;
    Ok(JValue::VInt(ret))
  }
  /// Performs subtraction.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_minus(&mut self, first: &Json, args: &[Json], func: &mut FuncInfo) -> JFuncResult {
    self.f_binary_op(first, args, func, "sub", "-")
  }
  /// Performs addition.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_plus(&mut self, first: &Json, args: &[Json], func: &mut FuncInfo) -> JFuncResult {
    self.f_binary_op(first, args, func, "add", "+")
  }
  /// Generates a unique name for internal use.
  fn get_name(&mut self, name: &str) -> Result<String, &'static str> {
    let seed = self
      .symbol_seeds
      .get(name)
      .map_or(Ok(0), |current| current.checked_add(1).ok_or("SeedOverflowError"))?;
    self.symbol_seeds.insert(name.to_owned(), seed);
    Ok(format!(".L{name}{seed:x}"))
  }
  /// Registers a function in the function table.
  fn register(&mut self, name: &str, ev: bool, fu: JFunc) {
    self.f_table.insert(name.into(), BuiltinFunc { evaluated: ev, func: fu });
  }
  /// Convert `JValue::` (`StringVar` or `String`) to `StringVar`, otherwise return `Err`
  fn string2var(&mut self, json: &Json, ctx: &str) -> ErrOR<String> {
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
