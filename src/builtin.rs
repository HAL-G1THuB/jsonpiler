//! Built-in functions.
use super::{Args, AsmFunc, ErrOR, FResult, FuncInfo, Json, JsonWithPos, Jsonpiler, err};
/// Macro to include assembly files only once.
macro_rules! include_once {
  ($self:ident, $name:literal) => {
    if !$self.include_flag.contains($name) {
      $self.include_flag.insert($name.into());
      $self.sect.text.push(include_str!(concat!("asm/", $name, ".s")).into());
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
  fn begin(&mut self, first: &JsonWithPos, args: &Args, _: &mut FuncInfo) -> FResult {
    self.validate_args("begin", true, 1, args.len(), &first.pos)?;
    Ok(args.last().ok_or("Unreachable (begin)")?.value.clone())
  }
  /// Utility functions for binary operations.
  fn binary_op(
    &mut self, first: &JsonWithPos, args: &Args, info: &mut FuncInfo, mn: &str, func_name: &str,
  ) -> FResult {
    let mut binary_mn = |json: &JsonWithPos, mne: &str, ord: usize| -> ErrOR<()> {
      if let Json::LInt(int) = json.value {
        info.body.push(format!("  {mne} rax, {int}\n"));
      } else if let Json::VInt(var) = &json.value {
        info.body.push(format!("  {mne} rax, {var}\n"));
      } else {
        self.typ_err(ord, func_name, "integer", json)?;
      }
      Ok(())
    };
    self.validate_args(func_name, true, 1, args.len(), &first.pos)?;
    let operand_r = args.first().ok_or("Unreachable (binary_op)")?;
    binary_mn(operand_r, "mov", 1)?;
    for (ord, operand_l) in args.get(1..).unwrap_or(&[]).iter().enumerate() {
      binary_mn(operand_l, mn, ord)?;
    }
    let ret = self.get_name("BSS", "8")?;
    info.body.push(format!("  mov {ret}, rax\n"));
    Ok(Json::VInt(ret))
  }
  /// Evaluates a lambda function definition.
  #[expect(clippy::single_call_fn, reason = "")]
  fn lambda(&mut self, first: &JsonWithPos, args: &Args, _: &mut FuncInfo) -> FResult {
    const ERR: &str = "Unreachable (lambda)";
    self.validate_args("lambda", true, 2, args.len(), &first.pos)?;
    let mut info = FuncInfo::default();
    let json1 = args.first().ok_or(ERR)?;
    let Json::LArray(params) = json1.value.clone() else {
      return self.typ_err(1, "lambda", "an argument list", json1);
    };
    if !params.is_empty() {
      return err!(self, &json1.pos, "PARAMETERS IS NOT IMPLEMENTED.");
    }
    let name = self.get_name("FNC", "")?;
    let mut ret = Json::Null;
    for arg in args.get(1..).ok_or(self.fmt_err("Empty lambda body.", &first.pos))? {
      ret = self.eval(arg, &mut info)?.value;
    }
    self.sect.text.push(format!(".seh_proc {name}\n{name}:\n"));
    let mut registers: Vec<&String> = info.reg_used.iter().collect();
    registers.sort();
    for &reg in &registers {
      self.sect.text.push(format!("  push {reg}\n  .seh_pushreg {reg}\n"));
    }
    self.sect.text.push("  push rbp\n  .seh_pushreg rbp\n".into());
    self.sect.text.push(format!(
      include_str!("asm/common/prologue.s"),
      size = info.calc_alloc((info.reg_used.len() % 2).saturating_mul(8))?
    ));
    for body in info.body {
      self.sect.text.push(body);
    }
    if let Json::LInt(int) = ret {
      self.sect.text.push(format!("  mov rax, {int}\n"));
    } else if let Json::VInt(var) = &ret {
      self.sect.text.push(format!("  mov rax, {var}\n"));
    } else {
      self.sect.text.push("  xor eax, eax\n".into());
    }
    self.sect.text.push("  mov rsp, rbp\n  pop rbp\n".into());
    registers.reverse();
    for reg in &registers {
      self.sect.text.push(format!("  pop {reg}\n"));
    }
    self.sect.text.push("  ret\n.seh_endproc\n".into());
    Ok(Json::Function(AsmFunc { name, params, ret: Box::new(ret) }))
  }
  /// Displays a message box.
  #[expect(clippy::single_call_fn, reason = "")]
  fn message(&mut self, first: &JsonWithPos, args: &Args, info: &mut FuncInfo) -> FResult {
    const ERR: &str = "Unreachable (message)";
    self.validate_args("message", false, 2, args.len(), &first.pos)?;
    info.reg_used.insert("rdi".into());
    info.reg_used.insert("rsi".into());
    let title = self.string2var(args.first().ok_or(ERR)?, 1, "message")?;
    let msg = self.string2var(args.get(1).ok_or(ERR)?, 2, "message")?;
    let ret = self.get_name("BSS", "8")?;
    include_once!(self, "func/U8TO16");
    info.body.push(format!(
      include_str!("asm/caller/message.s"),
      title = title,
      msg = msg,
      ret = ret
    ));
    Ok(Json::VInt(ret))
  }
  /// Performs subtraction.
  #[expect(clippy::single_call_fn, reason = "")]
  fn minus(&mut self, first: &JsonWithPos, args: &Args, info: &mut FuncInfo) -> FResult {
    self.binary_op(first, args, info, "sub", "-")
  }
  /// Performs addition.
  #[expect(clippy::single_call_fn, reason = "")]
  fn plus(&mut self, first: &JsonWithPos, args: &Args, info: &mut FuncInfo) -> FResult {
    self.binary_op(first, args, info, "add", "+")
  }
  /// Sets a variable.
  fn set(&mut self, first: &JsonWithPos, args: &Args, is_global: bool, f_name: &str) -> FResult {
    const ERR: &str = "Unreachable (set)";
    self.validate_args(f_name, false, 2, args.len(), &first.pos)?;
    let json1 = args.first().ok_or(ERR)?;
    let Json::LString(variable) = &json1.value else {
      return self.typ_err(1, f_name, "LString", json1);
    };
    let json2 = args.get(1).ok_or(ERR)?;
    let value = match &json2.value {
      Json::LString(st) => Json::VString(self.get_name("STR", st)?),
      Json::Null => Json::Null,
      Json::LInt(int) => Json::VInt(self.get_name("INT", &int.to_string())?),
      Json::VString(_)
      | Json::VInt(_)
      | Json::Function { .. }
      | Json::VArray(_)
      | Json::VBool(..)
      | Json::VFloat(_)
      | Json::VObject(_) => json2.value.clone(),
      Json::LArray(_) | Json::LBool(_) | Json::LFloat(_) | Json::LObject(_) => {
        return self.typ_err(2, "$", "that supports assignment", json2);
      }
    };
    if is_global {
      self.vars.first_mut().ok_or("InternalError: Invalid scope.")?
    } else {
      self.vars.last_mut().ok_or("InternalError: Invalid scope.")?
    }
    .insert(variable.clone(), value);
    Ok(Json::Null)
  }
  /// Sets a global variable.
  #[expect(clippy::single_call_fn, reason = "")]
  fn set_global(&mut self, first: &JsonWithPos, args: &Args, _: &mut FuncInfo) -> FResult {
    self.set(first, args, true, "global")
  }
  /// Sets a local variable.
  #[expect(clippy::single_call_fn, reason = "")]
  fn set_local(&mut self, first: &JsonWithPos, args: &Args, _: &mut FuncInfo) -> FResult {
    self.set(first, args, false, "=")
  }
  /// Gets the value of a local variable.
  #[expect(clippy::single_call_fn, reason = "")]
  fn variable(&mut self, first: &JsonWithPos, args: &Args, _: &mut FuncInfo) -> FResult {
    self.validate_args("$", false, 1, args.len(), &first.pos)?;
    let json1 = args.first().ok_or("Unreachable (variable)")?;
    let Json::LString(var_name) = &json1.value else {
      return self.typ_err(1, "$", "LString", json1);
    };
    self.get_var(var_name, &json1.pos)
  }
}
