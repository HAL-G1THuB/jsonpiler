//! Built-in functions.
use {
  super::{AsmFunc, ErrOR, FResult, FuncInfo, JValue, Json, Jsonpiler},
  core::fmt::Write as _,
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
  /// Registers all functions.
  pub(crate) fn all_register(&mut self) {
    let common = (false, false);
    #[expect(clippy::no_effect_underscore_binding, reason = "todo")]
    let _special = (true, false);
    let scope = (false, true);
    let sp_scope = (true, true);
    self.register("lambda", sp_scope, Jsonpiler::f_lambda);
    self.register("begin", scope, Jsonpiler::f_begin);
    self.register("global", common, Jsonpiler::f_set_global);
    self.register("=", common, Jsonpiler::f_set_local);
    self.register("message", common, Jsonpiler::f_message);
    self.register("+", common, Jsonpiler::f_plus);
    self.register("-", common, Jsonpiler::f_minus);
    self.register("$", common, Jsonpiler::f_variable);
  }
  /// Evaluates a 'begin' block.
  #[expect(clippy::single_call_fn, reason = "")]
  pub(crate) fn f_begin(&mut self, first: &Json, args: &[Json], _: &mut FuncInfo) -> FResult {
    args.last().map_or_else(
      || Err(self.fmt_err("'begin' requires at least one arguments.", &first.info).into()),
      |last| Ok(last.value.clone()),
    )
  }
  /// Utility functions for binary operations.
  fn f_binary_op(
    &mut self, first: &Json, args: &[Json], func: &mut FuncInfo, mn: &str, op: &str,
  ) -> FResult {
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
  /// Evaluates a lambda function definition.
  #[expect(clippy::single_call_fn, reason = "")]
  pub(crate) fn f_lambda(&mut self, first: &Json, args: &[Json], _: &mut FuncInfo) -> FResult {
    const ERR: &str = "Unreachable (f_lambda)";
    let mut func = FuncInfo::default();
    self.assert(args.len() >= 3, "Invalid function definition.", &first.info)?;
    let params_json = args.first().ok_or(ERR)?;
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
    for arg in args.get(1..).ok_or(self.fmt_err("Empty lambda body.", &first.info))? {
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
    Ok(JValue::Function(AsmFunc { name, params: params.clone(), ret: Box::new(ret) }))
  }
  /// Displays a message box.
  #[expect(clippy::single_call_fn, reason = "")]
  pub(crate) fn f_message(&mut self, first: &Json, args: &[Json], func: &mut FuncInfo) -> FResult {
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
  pub(crate) fn f_minus(&mut self, first: &Json, args: &[Json], func: &mut FuncInfo) -> FResult {
    self.f_binary_op(first, args, func, "sub", "-")
  }
  /// Performs addition.
  #[expect(clippy::single_call_fn, reason = "")]
  pub(crate) fn f_plus(&mut self, first: &Json, args: &[Json], func: &mut FuncInfo) -> FResult {
    self.f_binary_op(first, args, func, "add", "+")
  }
  /// Sets a variable.
  fn f_set(&mut self, first: &Json, args: &[Json], is_global: bool, func_name: &str) -> FResult {
    self.assert(args.len() == 2, &format!("'{func_name}' requires two arguments."), &first.info)?;
    let json1 = args.first().ok_or("Unreachable (f_set)")?;
    let JValue::LString(variable) = &json1.value else {
      return Err(self.fmt_err("Variable name must be a string literal.", &json1.info).into());
    };
    let json2 = args.get(1).ok_or("Unreachable (f_set)")?;
    let value = match &json2.value {
      JValue::LString(st) => {
        let name = self.get_name("STR")?;
        writeln!(self.sect.data, "  {name}: .string \"{st}\"")?;
        JValue::VString(name.clone())
      }
      JValue::Null => JValue::Null,
      JValue::LInt(int) => {
        let name = self.get_name("INT")?;
        writeln!(self.sect.data, "  {name}: .quad 0x{int:x}")?;
        JValue::VInt(name.clone())
      }
      JValue::VString(_)
      | JValue::VInt(_)
      | JValue::Function { .. }
      | JValue::VArray(_)
      | JValue::VBool(..)
      | JValue::VFloat(_)
      | JValue::VObject(_) => json2.value.clone(),
      JValue::LArray(_) | JValue::LBool(_) | JValue::LFloat(_) | JValue::LObject(_) => {
        return Err(self.fmt_err("Assignment to an unimplemented type.", &json2.info).into());
      }
    };
    if is_global {
      self.vars.first_mut().ok_or("InternalError: Invalid scope.")?
    } else {
      self.vars.last_mut().ok_or("InternalError: Invalid scope.")?
    }
    .insert(variable.clone(), value);
    Ok(JValue::Null)
  }
  /// Sets a global variable.
  #[expect(clippy::single_call_fn, reason = "")]
  pub(crate) fn f_set_global(&mut self, first: &Json, args: &[Json], _: &mut FuncInfo) -> FResult {
    self.f_set(first, args, true, "global")
  }
  /// Sets a local variable.
  #[expect(clippy::single_call_fn, reason = "")]
  pub(crate) fn f_set_local(&mut self, first: &Json, args: &[Json], _: &mut FuncInfo) -> FResult {
    self.f_set(first, args, false, "=")
  }
  /// Gets the value of a local variable.
  #[expect(clippy::single_call_fn, reason = "")]
  pub(crate) fn f_variable(&mut self, first: &Json, args: &[Json], _: &mut FuncInfo) -> FResult {
    self.assert(args.len() == 1, "'$' requires one argument.", &first.info)?;
    let json1 = args.first().ok_or("Unreachable (f_set_local)")?;
    let JValue::LString(var_name) = &json1.value else {
      return Err(self.fmt_err("Variable name must be a string literal.", &json1.info).into());
    };
    for scope in self.vars.iter().rev() {
      if let Some(val) = scope.get(var_name) {
        return Ok(val.clone());
      }
    }
    Err(self.fmt_err(&format!("Undefined variables: '{var_name}'"), &json1.info).into())
  }
}
