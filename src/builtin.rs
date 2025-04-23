//! Built-in functions.
use {
  super::{AsmFunc, ErrOR, FResult, FuncInfo, JValue, Json, Jsonpiler},
  core::fmt::Write as _,
};
/// Macro to include assembly files only once.
macro_rules! include_once {
  ($self:ident, $name:literal) => {
    if !$self.include_flag.contains($name) {
      $self.include_flag.insert($name.into());
      write!($self.sect.text, include_str!(concat!("asm/", $name, ".s")))?;
    }
  };
}
impl Jsonpiler {
  /// Registers all functions.
  pub(crate) fn all_register(&mut self) {
    let common = (false, false);
    #[expect(clippy::no_effect_underscore_binding, reason = "todo")]
    let _special = (true, false);
    let scope = (false, true);
    let sp_scope = (true, true);
    self.register("lambda", sp_scope, Jsonpiler::lambda);
    self.register("begin", scope, Jsonpiler::begin);
    self.register("global", common, Jsonpiler::set_global);
    self.register("=", common, Jsonpiler::set_local);
    self.register("message", common, Jsonpiler::message);
    self.register("+", common, Jsonpiler::plus);
    self.register("-", common, Jsonpiler::minus);
    self.register("$", common, Jsonpiler::variable);
  }
}
impl Jsonpiler {
  /// Evaluates a `begin` block.
  #[expect(clippy::single_call_fn, reason = "")]
  fn begin(&mut self, first: &Json, args: &[Json], _: &mut FuncInfo) -> FResult {
    self.validate("begin", true, 1, args.len(), &first.info)?;
    Ok(args.last().ok_or("Unreachable (begin)")?.value.clone())
  }
  /// Utility functions for binary operations.
  fn binary_op(
    &mut self, first: &Json, args: &[Json], func: &mut FuncInfo, mn: &str, op: &str,
  ) -> FResult {
    let mut f_binary_mn = |json: &Json, mne: &str| -> ErrOR<()> {
      if let JValue::LInt(int) = json.value {
        writeln!(func.body, "  {mne} rax, {int}")?;
      } else if let JValue::VInt(var) = &json.value {
        writeln!(func.body, "  {mne} rax, qword ptr {var}[rip]")?;
      } else {
        self.require(op, "integer", json)?;
      }
      Ok(())
    };
    self.validate(op, true, 1, args.len(), &first.info)?;
    let operand_r = args.first().ok_or("Unreachable (binary_op)")?;
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
  fn lambda(&mut self, first: &Json, args: &[Json], _: &mut FuncInfo) -> FResult {
    const ERR: &str = "Unreachable (lambda)";
    self.validate("lambda", true, 3, args.len(), &first.info)?;
    let mut func = FuncInfo::default();
    let json1 = args.first().ok_or(ERR)?;
    let JValue::LArray(params) = &json1.value else {
      return self.require("1st argument of `lambda`", "an argument list", json1);
    };
    self.assert(params.is_empty(), "PARAMS IS NOT IMPLEMENTED.", &json1.info)?;
    let name = self.get_name("FNC")?;
    let mut ret = JValue::Null;
    for arg in args.get(1..).ok_or(self.fmt_err("Empty lambda body.", &first.info))? {
      ret = self.eval(arg, &mut func)?.value;
    }
    writeln!(self.sect.text, ".seh_proc {name}\n{name}:")?;
    let alloc_size = func.calc_alloc(if func.reg_used.len() & 1 == 1 { 8 } else { 0 })?;
    let mut registers: Vec<&String> = func.reg_used.iter().collect();
    registers.sort();
    for &reg in &registers {
      writeln!(self.sect.text, "  push {reg}\n  .seh_pushreg {reg}")?;
    }
    writeln!(
      self.sect.text,
      "  push rbp
  .seh_pushreg rbp
  mov rbp, rsp
  .seh_setframe rbp, 0
  sub rsp, {alloc_size}
  .seh_stackalloc {alloc_size}
  .seh_endprologue
  .seh_handler .L__SEH_HANDLER, @except",
    )?;
    self.sect.text.push_str(&func.body);
    if let JValue::LInt(int) = ret {
      writeln!(self.sect.text, "  mov rax, {int}")?;
    } else if let JValue::VInt(var) = &ret {
      writeln!(self.sect.text, "  mov rax, qword ptr {var}[rip]")?;
    } else {
      self.sect.text.push_str("  xor eax, eax\n");
    }
    self.sect.text.push_str("  mov rsp, rbp\n  pop rbp\n");
    registers.reverse();
    for reg in &registers {
      writeln!(self.sect.text, "  pop {reg}")?;
    }
    self.sect.text.push_str("  ret\n.seh_endproc\n");
    Ok(JValue::Function(AsmFunc { name, params: params.clone(), ret: Box::new(ret) }))
  }
  /// Displays a message box.
  #[expect(clippy::single_call_fn, reason = "")]
  fn message(&mut self, first: &Json, args: &[Json], func: &mut FuncInfo) -> FResult {
    const ERR: &str = "Unreachable (message)";
    self.validate("message", false, 2, args.len(), &first.info)?;
    func.reg_used.insert("rdi".into());
    func.reg_used.insert("rsi".into());
    let title = self.string2var(args.first().ok_or(ERR)?, "title")?;
    let msg = self.string2var(args.get(1).ok_or(ERR)?, "text")?;
    let ret = self.get_name("INT")?;
    writeln!(self.sect.bss, "  .lcomm {ret}, 8")?;
    include_once!(self, "func/U8TO16");
    write!(func.body, include_str!("asm/caller/message.s"), msg = msg, title = title, ret = ret)?;
    Ok(JValue::VInt(ret))
  }
  /// Performs subtraction.
  #[expect(clippy::single_call_fn, reason = "")]
  fn minus(&mut self, first: &Json, args: &[Json], func: &mut FuncInfo) -> FResult {
    self.binary_op(first, args, func, "sub", "-")
  }
  /// Performs addition.
  #[expect(clippy::single_call_fn, reason = "")]
  fn plus(&mut self, first: &Json, args: &[Json], func: &mut FuncInfo) -> FResult {
    self.binary_op(first, args, func, "add", "+")
  }
  /// Sets a variable.
  fn set(&mut self, first: &Json, args: &[Json], is_global: bool, func_name: &str) -> FResult {
    const ERR: &str = "Unreachable (set)";
    self.validate(func_name, false, 2, args.len(), &first.info)?;
    let json1 = args.first().ok_or(ERR)?;
    let JValue::LString(variable) = &json1.value else {
      return self.require(&format!("1st argument of `{func_name}`"), "a string literal", json1);
    };
    let json2 = args.get(1).ok_or(ERR)?;
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
        return self.require("2nd argument of `$`", "a type that supports assignment", json2);
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
  fn set_global(&mut self, first: &Json, args: &[Json], _: &mut FuncInfo) -> FResult {
    self.set(first, args, true, "global")
  }
  /// Sets a local variable.
  #[expect(clippy::single_call_fn, reason = "")]
  fn set_local(&mut self, first: &Json, args: &[Json], _: &mut FuncInfo) -> FResult {
    self.set(first, args, false, "=")
  }
  /// Gets the value of a local variable.
  #[expect(clippy::single_call_fn, reason = "")]
  fn variable(&mut self, first: &Json, args: &[Json], _: &mut FuncInfo) -> FResult {
    self.validate("$", false, 1, args.len(), &first.info)?;
    let json1 = args.first().ok_or("Unreachable (variable)")?;
    let JValue::LString(var_name) = &json1.value else {
      return self.require("1st argument of `$`", "a string literal", json1);
    };
    for scope in self.vars.iter().rev() {
      if let Some(val) = scope.get(var_name) {
        return Ok(val.clone());
      }
    }
    Err(self.fmt_err(&format!("Undefined variables: `{var_name}`"), &json1.info).into())
  }
}
