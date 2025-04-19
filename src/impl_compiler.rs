//! Implementation of the compiler inside the `Jsonpiler`.
use super::{
  BuiltinFunc, ErrOR, JFunc, JFuncResult, JObject, JResult, JValue, Json, Jsonpiler, Section,
  functions::obj_json,
};
use core::fmt::Write as _;
use std::{
  fs::File,
  io::{self, BufWriter, Write as _},
};
impl Jsonpiler {
  /// Assert condition.
  fn assert(&self, cond: bool, text: &str, obj: &Json) -> ErrOR<()> {
    cond.then_some(()).ok_or_else(|| self.fmt_err(text, &obj.info).into())
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
    self.seed = 0;
    self.register("=", true, Jsonpiler::f_local_set);
    self.register("$", true, Jsonpiler::f_local_get);
    self.register("+", true, Jsonpiler::f_plus);
    self.register("-", true, Jsonpiler::f_minus);
    self.register("message", true, Jsonpiler::f_message);
    self.register("begin", true, Jsonpiler::f_begin);
    let mut start = String::new();
    self.sect = Section::default();
    let result = self.eval(&json, &mut start)?;
    writeln!(
      start,
      "  {}
  call [qword ptr __imp_ExitProcess[rip]]
  .seh_endproc",
      if let JValue::Int(int) = result.value {
        format!("mov rcx, {int}")
      } else if let JValue::IntVar(var) = &result.value {
        format!("mov rcx, qword ptr {var}[rip]")
      } else {
        "xor ecx, ecx".into()
      }
    )?;
    self.write_file(&start, filename, json_file)?;
    Ok(())
  }
  /// Evaluates a JSON object.
  fn eval(&mut self, json: &Json, function: &mut String) -> JResult {
    const ERR: &str = "Unreachable (eval)";
    let JValue::Array(list) = &json.value else {
      let JValue::Object(object) = &json.value else { return Ok(json.clone()) };
      let mut evaluated = JObject::default();
      for kv in object.iter() {
        evaluated.insert(kv.0.clone(), self.eval(&kv.1, function)?);
      }
      return Ok(obj_json(JValue::Object(evaluated), json.info.clone()));
    };
    let Some(first_elem) = list.first() else {
      return Err(self.fmt_err("An function call cannot be an empty list.", &json.info).into());
    };
    let first = &self.eval(first_elem, function)?;
    if let JValue::String(cmd) = &first.value {
      if cmd == "lambda" {
        let mut func_buffer = String::new();
        let result = Ok(self.eval_lambda(json, &mut func_buffer)?);
        self.sect.text.push_str(&func_buffer);
        result
      } else if self.f_table.contains_key(cmd.as_str()) {
        let args = if self.f_table.get_mut(cmd.as_str()).ok_or(ERR)?.evaluated {
          &self.eval_args(list.get(1..).unwrap_or(&[]), function)?
        } else {
          list.get(1..).unwrap_or(&[])
        };
        Ok(obj_json(
          (self.f_table.get_mut(cmd.as_str()).ok_or(ERR)?.func)(self, first, args, function)?,
          first.info.clone(),
        ))
      } else {
        Err(self.fmt_err(&format!("Function '{cmd}' is undefined."), &first.info).into())
      }
    } else if let JValue::Function { name: n, ret: re, .. } = &first.value {
      writeln!(function, "  call {n}")?;
      if let JValue::IntVar(_) | JValue::Int(_) = **re {
        let na = self.get_name()?;
        writeln!(self.sect.bss, "  .lcomm {na}, 8")?;
        writeln!(function, "  mov qword ptr {na}[rip], rax")?;
        Ok(obj_json(JValue::IntVar(na), first.info.clone()))
      } else {
        Ok(Json::default())
      }
    } else {
      Err(
        self
          .fmt_err(
            "The first element of an evaluation list requires a function name or a lambda object.",
            &json.info,
          )
          .into(),
      )
    }
  }
  /// Evaluate arguments.
  fn eval_args(&mut self, args: &[Json], function: &mut String) -> ErrOR<Vec<Json>> {
    let mut result = vec![];
    for arg in args {
      result.push(self.eval(arg, function)?);
    }
    Ok(result)
  }
  /// Evaluates a lambda function definition.
  fn eval_lambda(&mut self, func: &Json, function: &mut String) -> JResult {
    const ERR: &str = "Unreachable (eval_lambda)";
    let tmp = self.vars.clone();
    let JValue::Array(func_list) = &func.value else {
      return Err(self.fmt_err("Invalid function definition.", &func.info).into());
    };
    self.assert(func_list.len() >= 3, "Invalid function definition.", func)?;
    let lambda = func_list.first().ok_or(ERR)?;
    self.assert(
      matches!(&lambda.value, JValue::String(st) if st == "lambda"),
      r#"The first element of a lambda list requires "lambda"."#,
      lambda,
    )?;
    let params_json = func_list.get(1).ok_or(ERR)?;
    let JValue::Array(params) = &params_json.value else {
      return Err(
        self
          .fmt_err(
            "The second element of a lambda list requires an argument list.",
            &params_json.info,
          )
          .into(),
      );
    };
    self.assert(params.is_empty(), "PARAMS ISN'T IMPLEMENTED.", params_json)?;
    let name = self.get_name()?;
    writeln!(
      function,
      ".section .text${},\"x\"
.seh_proc {name}
{name}:
  push rbp
  .seh_pushreg rbp
  mov rbp, rsp
  .seh_setframe rbp, 0
  sub rsp, 32
  .seh_stackalloc 32
  .seh_endprologue
  .seh_handler .L_SEH_HANDLER, @except",
      &name.get(1..).ok_or(ERR)?
    )?;
    let mut ret = JValue::Null;
    for arg in func_list.get(2..).ok_or("Empty lambda body.")? {
      ret = self.eval(arg, function)?.value;
    }
    writeln!(
      function,
      "  {}
  add rsp, 32
  leave
  ret
  .seh_endproc",
      if let JValue::Int(int) = ret {
        format!("mov rax, {int}")
      } else if let JValue::IntVar(var) = &ret {
        format!("mov rax, qword ptr {var}[rip]")
      } else {
        "xor eax, eax".into()
      }
    )?;
    self.vars = tmp;
    Ok(obj_json(
      JValue::Function { name, params: params.clone(), ret: Box::new(ret) },
      lambda.info.clone(),
    ))
  }
  /// Evaluates a 'begin' block.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_begin(&mut self, first: &Json, args: &[Json], _: &mut String) -> JFuncResult {
    args.last().map_or_else(
      || Err(self.fmt_err("'begin' requires at least one argument", &first.info).into()),
      |last| Ok(last.value.clone()),
    )
  }
  /// Utility functions for binary operations
  fn f_binary_op(
    &mut self, first: &Json, args: &[Json], function: &mut String, mn: &str, op: &str,
  ) -> JFuncResult {
    let mut f_binary_mn = |json: &Json, mne: &str| -> ErrOR<()> {
      if let JValue::Int(int) = json.value {
        Ok(writeln!(function, "  {mne} rax, {int}")?)
      } else if let JValue::IntVar(var) = &json.value {
        Ok(writeln!(function, "  {mne} rax, qword ptr {var}[rip]")?)
      } else {
        Err(self.fmt_err(&format!("'{op}' requires integer operands"), &json.info).into())
      }
    };
    self.assert(args.len() >= 2, &format!("'{op}' requires at least two arguments"), first)?;
    let operand_r = args.first().ok_or("Unreachable (binary_op)")?;
    f_binary_mn(operand_r, "mov")?;
    for operand_l in args.get(1..).ok_or("Unreachable (binary_op)")? {
      f_binary_mn(operand_l, mn)?;
    }
    let ret = self.get_name()?;
    writeln!(self.sect.bss, "  .lcomm {ret}, 8")?;
    writeln!(function, "  mov qword ptr {ret}[rip], rax")?;
    Ok(JValue::IntVar(ret))
  }
  /// Gets the value of a local variable.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_local_get(&mut self, first: &Json, args: &[Json], _: &mut String) -> JFuncResult {
    self.assert(args.len() == 1, "'$' requires one argument.", first)?;
    let Some(var) = args.first() else {
      return Err(self.fmt_err("'$' requires one argument.", &first.info).into());
    };
    let JValue::String(var_name) = &var.value else {
      return Err(self.fmt_err("Variable name requires string literal.", &var.info).into());
    };
    match self.vars.get(var_name) {
      Some(value) => Ok(value.clone()),
      None => Err(self.fmt_err(&format!("Undefined variables: '{var_name}'"), &var.info).into()),
    }
  }
  /// Sets a local variable.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_local_set(&mut self, first: &Json, args: &[Json], _: &mut String) -> JFuncResult {
    self.assert(args.len() == 2, "'=' requires two arguments.", first)?;
    let JValue::String(var_name) = &args.first().ok_or("Unreachable (f_set_local)")?.value else {
      return Err(
        self
          .fmt_err(
            "Variable name requires compile-time fixed strings.",
            &args.first().ok_or("Unreachable (f_set_local)")?.info,
          )
          .into(),
      );
    };
    let result = args.get(1).ok_or("Unreachable (f_set_local)")?;
    if let JValue::String(st) = &result.value {
      let name = self.get_name()?;
      writeln!(self.sect.data, "  {name}: .string \"{st}\"")?;
      self.vars.insert(var_name.clone(), JValue::StringVar(name.clone()));
      Ok(JValue::StringVar(name))
    } else if let JValue::StringVar(sv) = &result.value {
      self.vars.insert(var_name.clone(), JValue::StringVar(sv.clone()));
      Ok(JValue::StringVar(sv.clone()))
    } else {
      Err(self.fmt_err("Assignment to an unimplemented type.", &result.info).into())
    }
  }
  /// Displays a message box.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_message(&mut self, first: &Json, args: &[Json], function: &mut String) -> JFuncResult {
    self.assert(args.len() == 2, "'message' requires two arguments.", first)?;
    let title = self.string2var(args.first().ok_or("Unreachable (f_message)")?, "title")?;
    let msg = self.string2var(args.get(1).ok_or("Unreachable (f_message)")?, "text")?;
    let wtitle = self.get_name()?;
    let wmsg = self.get_name()?;
    let ret = self.get_name()?;
    for data in [&wtitle, &wmsg, &ret] {
      writeln!(self.sect.bss, "  .lcomm {data}, 8")?;
    }
    write!(
      function,
      include_str!("asm/message.s"),
      msg, wmsg, title, wtitle, wmsg, wtitle, ret, wmsg, wtitle
    )?;
    Ok(JValue::IntVar(ret))
  }
  /// Performs subtraction.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_minus(&mut self, first: &Json, args: &[Json], function: &mut String) -> JFuncResult {
    self.f_binary_op(first, args, function, "sub", "-")
  }
  /// Performs addition.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_plus(&mut self, first: &Json, args: &[Json], function: &mut String) -> JFuncResult {
    self.f_binary_op(first, args, function, "add", "+")
  }
  /// Generates a unique name for internal use.
  fn get_name(&mut self) -> Result<String, &str> {
    let Some(seed) = self.seed.checked_add(1) else { return Err("SeedOverflowError") };
    self.seed = seed;
    Ok(format!(".LC{seed:x}"))
  }
  /// Registers a function in the function table.
  fn register(&mut self, name: &str, ev: bool, fu: JFunc) {
    self.f_table.insert(name.into(), BuiltinFunc { evaluated: ev, func: fu });
  }
  /// Convert `JValue::StringVar` or `JValue::String` to `JValue::StringVar`, otherwise return `Err`
  fn string2var(&mut self, json: &Json, text: &str) -> ErrOR<String> {
    if let JValue::String(st) = &json.value {
      let name = self.get_name()?;
      writeln!(self.sect.data, "  {name}: .string \"{st}\"")?;
      Ok(name)
    } else if let JValue::StringVar(var) = &json.value {
      Ok(var.clone())
    } else {
      Err(self.fmt_err(&format!("'{text}' must be a string."), &json.info).into())
    }
  }
  /// Writes the compiled assembly code to a file.
  fn write_file(&self, start: &str, filename: &str, json_file: &str) -> io::Result<()> {
    let mut writer = BufWriter::new(File::create(filename)?);
    writer.write_all(format!(".file \"{json_file}\"\n.intel_syntax noprefix\n").as_bytes())?;
    writer.write_all(include_bytes!("asm/data.s"))?;
    writer.write_all(self.sect.data.as_bytes())?;
    writer.write_all(include_bytes!("asm/bss.s"))?;
    writer.write_all(self.sect.bss.as_bytes())?;
    writer.write_all(include_bytes!("asm/start.s"))?;
    writer.write_all(start.as_bytes())?;
    writer.write_all(include_bytes!("asm/text.s"))?;
    writer.write_all(self.sect.text.as_bytes())?;
    writer.flush()?;
    Ok(())
  }
}
