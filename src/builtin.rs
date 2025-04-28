//! Built-in functions.
use super::{Align, Args, AsmFunc, ErrOR, FuncInfo, Json, JsonWithPos, Jsonpiler, err};
use core::mem::take;
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
    let special = (true, false);
    let scope = (false, true);
    let sp_scope = (true, true);
    self.register("lambda", sp_scope, Jsonpiler::lambda);
    self.register("begin", common, Jsonpiler::begin);
    self.register("scope", scope, Jsonpiler::begin);
    self.register("global", common, Jsonpiler::set_global);
    self.register("=", common, Jsonpiler::set_local);
    self.register("message", common, Jsonpiler::message);
    self.register("'", special, Jsonpiler::quote);
    self.register("eval", common, Jsonpiler::f_eval);
    self.register("list", common, Jsonpiler::list);
    self.register("+", common, Jsonpiler::op_plus);
    self.register("-", common, Jsonpiler::op_minus);
    self.register("*", common, Jsonpiler::op_mul);
    self.register("$", common, Jsonpiler::variable);
  }
}
impl Jsonpiler {
  /// Evaluates a `begin` block.
  fn begin(&mut self, first: &JsonWithPos, mut args: Args, _: &mut FuncInfo) -> ErrOR<Json> {
    self.validate_args("begin` and `scope", true, 1, args.len(), &first.pos)?;
    Ok(take(args.last_mut().ok_or("Unreachable (begin)")?).value)
  }
  /// Return the first argument.
  #[expect(clippy::single_call_fn, reason = "")]
  fn f_eval(&mut self, first: &JsonWithPos, mut args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    self.validate_args("eval", false, 1, args.len(), &first.pos)?;
    Ok(self.eval(take(args.first_mut().ok_or("Unreachable (eval)")?), info)?.value)
  }
  /// Evaluates a lambda function definition.
  #[expect(clippy::single_call_fn, reason = "")]
  fn lambda(&mut self, first: &JsonWithPos, mut args: Args, _: &mut FuncInfo) -> ErrOR<Json> {
    const ERR: &str = "Unreachable (lambda)";
    self.validate_args("lambda", true, 2, args.len(), &first.pos)?;
    let mut info = FuncInfo::default();
    let json1 = take(args.first_mut().ok_or(ERR)?);
    let Json::LArray(params) = json1.value else {
      return self.typ_err(1, "lambda", "LArray", &json1);
    };
    if !params.is_empty() {
      return err!(self, &json1.pos, "PARAMETERS IS NOT IMPLEMENTED.");
    }
    let name = self.get_global("FNC", "")?;
    let mut ret = Json::Null;
    for arg in args.get_mut(1..).ok_or(self.fmt_err("Empty lambda body.", &first.pos))? {
      ret = self.eval(take(arg), &mut info)?.value;
    }
    self.sect.text.push(format!(".seh_proc {name}\n{name}:\n"));
    let mut registers: Vec<&String> = info.reg_used.iter().collect();
    registers.sort();
    for &reg in &registers {
      self.sect.text.push(format!("  push {reg}\n  .seh_pushreg {reg}\n"));
    }
    self.sect.text.push("  push rbp\n  .seh_pushreg rbp\n".into());
    let size = info.calc_alloc(if info.reg_used.len() % 2 == 1 { 8 } else { 0 })?;
    self.sect.text.push(format!(include_str!("asm/common/prologue.s"), size = size));
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
    for reg in registers {
      self.sect.text.push(format!("  pop {reg}\n"));
    }
    self.sect.text.push("  ret\n.seh_endproc\n".into());
    Ok(Json::Function(AsmFunc { name, params, ret: Box::new(ret) }))
  }
  /// Return the arguments.
  #[expect(clippy::single_call_fn, reason = "")]
  #[expect(clippy::unnecessary_wraps, reason = "")]
  #[expect(clippy::unused_self, reason = "")]
  fn list(&mut self, _: &JsonWithPos, args: Args, _: &mut FuncInfo) -> ErrOR<Json> {
    Ok(Json::LArray(args))
  }
  /// Displays a message box.
  #[expect(clippy::single_call_fn, reason = "")]
  fn message(&mut self, first: &JsonWithPos, mut args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    const ERR: &str = "Unreachable (message)";
    self.validate_args("message", false, 2, args.len(), &first.pos)?;
    info.reg_used.insert("rdi".into());
    info.reg_used.insert("rsi".into());
    let title = self.string2var(take(args.first_mut().ok_or(ERR)?), 1, "message")?;
    let msg = self.string2var(take(args.get_mut(1).ok_or(ERR)?), 2, "message")?;
    let ret = info.get_local(Align::U64)?;
    include_once!(self, "func/U8TO16");
    info.body.push(format!(
      include_str!("asm/caller/message.s"),
      title = title,
      msg = msg,
      ret = ret
    ));
    Ok(Json::VInt(ret))
  }
  /// Utility functions for binary operations.
  #[expect(clippy::needless_pass_by_value, reason = "")]
  fn op(
    &mut self, args: Args, info: &mut FuncInfo, mn: &str, func_name: &str, ident_elem: i64,
  ) -> ErrOR<Json> {
    let binary_mn =
      |json: &JsonWithPos, mne: &str, ord: usize, body: &mut Vec<String>| -> ErrOR<()> {
        if let Json::LInt(int) = json.value {
          if int > i64::from(i32::MAX) || int < i64::from(i32::MIN) {
            body.push(format!("  mov rcx, {int}\n"));
            body.push(format!("  {mne} rax, rcx\n"));
          } else {
            body.push(format!("  {mne} rax, {int}\n"));
          }
        } else if let Json::VInt(var) = &json.value {
          body.push(format!("  {mne} rax, {var}\n"));
        } else {
          self.typ_err(ord, func_name, "integer", json)?;
        }
        Ok(())
      };
    if let Some(operand_r) = args.first() {
      binary_mn(operand_r, "mov", 1, &mut info.body)?;
    } else {
      info.body.push(format!("  mov rax, {ident_elem}\n"));
    }
    for (ord, operand_l) in args.iter().enumerate().skip(1) {
      binary_mn(operand_l, mn, ord, &mut info.body)?;
    }
    let ret = info.get_local(Align::U64)?;
    info.body.push(format!("  mov {ret}, rax\n"));
    Ok(Json::VInt(ret))
  }
  /// Performs subtraction.
  #[expect(clippy::single_call_fn, reason = "")]
  fn op_minus(&mut self, _: &JsonWithPos, args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    self.op(args, info, "sub", "-", 0)
  }
  /// Performs addition.
  #[expect(clippy::single_call_fn, reason = "")]
  fn op_mul(&mut self, _: &JsonWithPos, args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    self.op(args, info, "imul", "*", 1)
  }
  /// Performs addition.
  #[expect(clippy::single_call_fn, reason = "")]
  fn op_plus(&mut self, _: &JsonWithPos, args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    self.op(args, info, "add", "+", 0)
  }
  /// Return the first argument.
  #[expect(clippy::single_call_fn, reason = "")]
  fn quote(&mut self, first: &JsonWithPos, mut args: Args, _: &mut FuncInfo) -> ErrOR<Json> {
    self.validate_args("'", false, 1, args.len(), &first.pos)?;
    Ok(take(args.first_mut().ok_or("Unreachable (quote)")?).value)
  }
  /// Sets a global variable.
  #[expect(clippy::single_call_fn, reason = "")]
  fn set_global(&mut self, first: &JsonWithPos, mut args: Args, _: &mut FuncInfo) -> ErrOR<Json> {
    const ERR: &str = "Unreachable (set)";
    self.validate_args("global", false, 2, args.len(), &first.pos)?;
    let json1 = take(args.first_mut().ok_or(ERR)?);
    let Json::LString(variable) = json1.value else {
      return self.typ_err(1, "global", "LString", &json1);
    };
    let json2 = take(args.get_mut(1).ok_or(ERR)?);
    let value = match json2.value {
      var if !var.is_literal() => var,
      Json::LString(st) => Json::VString(self.get_global("STR", &st)?),
      Json::Null => Json::Null,
      Json::LInt(int) => Json::VInt(self.get_global("INT", &int.to_string())?),
      _ => return self.typ_err(2, "$", "that supports assignment", &json2),
    };
    self.vars.first_mut().ok_or("InternalError: Invalid scope.")?.insert(variable, value);
    Ok(Json::Null)
  }
  /// Sets a local variable.
  #[expect(clippy::single_call_fn, reason = "")]
  fn set_local(&mut self, first: &JsonWithPos, mut args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    const ERR: &str = "Unreachable (set)";
    self.validate_args("=", false, 2, args.len(), &first.pos)?;
    let json1 = take(args.first_mut().ok_or(ERR)?);
    let Json::LString(variable) = json1.value else {
      return self.typ_err(1, "=", "LString", &json1);
    };
    let json2 = take(args.get_mut(1).ok_or(ERR)?);
    let value = match json2.value {
      var if !var.is_literal() => var,
      Json::LString(st) => Json::VString(self.get_global("STR", &st)?),
      Json::Null => Json::Null,
      Json::LInt(int) => {
        let name = info.get_local(Align::U64)?;
        info.body.push(format!("  mov {name}, {int}\n"));
        Json::VInt(name)
      }
      _ => return self.typ_err(2, "$", "that supports assignment", &json2),
    };
    self.vars.last_mut().ok_or("InternalError: Invalid scope.")?.insert(variable, value);
    Ok(Json::Null)
  }
  /// Gets the value of a local variable.
  #[expect(clippy::single_call_fn, clippy::needless_pass_by_value, reason = "")]
  fn variable(&mut self, first: &JsonWithPos, args: Args, _: &mut FuncInfo) -> ErrOR<Json> {
    self.validate_args("$", false, 1, args.len(), &first.pos)?;
    let json1 = args.first().ok_or("Unreachable (variable)")?;
    let Json::LString(var_name) = &json1.value else {
      return self.typ_err(1, "$", "LString", json1);
    };
    self.get_var(var_name, &json1.pos)
  }
}
