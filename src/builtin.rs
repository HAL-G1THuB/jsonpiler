//! Built-in functions.
use super::{
  Align, Args, AsmFunc, Bind, ErrOR, FuncInfo, Json, JsonWithPos, Jsonpiler, err,
  utility::fmt_local,
};
use core::mem::take;
use std::collections::HashMap;
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
    let sp_scope = (true, true);
    self.register("lambda", special, Jsonpiler::lambda);
    self.register("begin", special, Jsonpiler::begin);
    self.register("scope", sp_scope, Jsonpiler::begin);
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
#[expect(clippy::single_call_fn, reason = "")]
impl Jsonpiler {
  /// Evaluates a `begin` block.
  fn begin(&mut self, first: &JsonWithPos, mut args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    self.validate_args("begin` and `scope", true, 1, args.len(), &first.pos)?;
    let len = args.len();
    if len <= 1 {
      return self
        .eval(take(args.last_mut().ok_or("Unreachable (begin)")?), info)
        .map(|jwp| jwp.value);
    }
    for arg in args.get_mut(1..len.saturating_sub(1)).unwrap_or(&mut []) {
      let val = self.eval(take(arg), info)?.value;
      if let Some((addr, size)) = val.tmp() {
        info.free(addr, size)?;
      }
    }
    self.eval(take(args.last_mut().ok_or("Unreachable (begin)")?), info).map(|jwp| jwp.value)
  }
  /// Return the first argument.
  fn f_eval(&mut self, first: &JsonWithPos, mut args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    self.validate_args("eval", false, 1, args.len(), &first.pos)?;
    Ok(self.eval(take(args.first_mut().ok_or("Unreachable (eval)")?), info)?.value)
  }
  /// Evaluates a lambda function definition.
  fn lambda(&mut self, first: &JsonWithPos, mut args: Args, _: &mut FuncInfo) -> ErrOR<Json> {
    const ERR: &str = "Unreachable (lambda)";
    self.validate_args("lambda", true, 2, args.len(), &first.pos)?;
    self.vars.push(HashMap::new());
    let mut info = FuncInfo::default();
    let json1 = take(args.first_mut().ok_or(ERR)?);
    let Json::Array(Bind::Lit(params)) = json1.value else {
      return self.typ_err(1, "lambda", "LArray", &json1);
    };
    if !params.is_empty() {
      return err!(self, &json1.pos, "PARAMETERS IS NOT IMPLEMENTED.");
    }
    let name = self.get_global("FNC", "")?;
    let mut ret = Json::Null;
    for arg in args.get_mut(1..).ok_or(self.fmt_err("Empty lambda body.", &first.pos))? {
      ret = self.eval(take(arg), &mut info)?.value;
      ret.tmp().and_then(|tuple| info.free(tuple.0, tuple.1).ok());
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
    if let Json::Int(int) = &ret {
      match int {
        Bind::Lit(lint) => self.sect.text.push(format!("  mov rax, {lint}\n")),
        Bind::Var(var) => self.sect.text.push(format!("  mov rax, {var}\n")),
        Bind::Local(local) | Bind::Tmp(local) => {
          self.sect.text.push(format!("  mov rax, {}\n", fmt_local("qword", *local)));
        }
      }
    } else {
      self.sect.text.push("  xor eax, eax\n".into());
    }
    self.sect.text.push("  mov rsp, rbp\n  pop rbp\n".into());
    registers.reverse();
    for reg in registers {
      self.sect.text.push(format!("  pop {reg}\n"));
    }
    self.sect.text.push("  ret\n.seh_endproc\n".into());
    self.vars.pop();
    Ok(Json::Function(AsmFunc { name, params, ret: Box::new(ret) }))
  }
  /// Return the arguments.
  #[expect(clippy::unnecessary_wraps, reason = "")]
  #[expect(clippy::unused_self, reason = "")]
  fn list(&mut self, _: &JsonWithPos, args: Args, _: &mut FuncInfo) -> ErrOR<Json> {
    Ok(Json::Array(Bind::Lit(args)))
  }
  /// Displays a message box.
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
      ret = fmt_local("qword", ret)
    ));
    Ok(Json::Int(Bind::Tmp(ret)))
  }
  /// Utility functions for binary operations.
  #[expect(clippy::needless_pass_by_value, reason = "")]
  fn op(
    &mut self, args: Args, info: &mut FuncInfo, mn: &str, f_name: &str, id_elem: usize,
  ) -> ErrOR<Json> {
    let binary_mn =
      |json: &JsonWithPos, mne: &str, ord: usize, f_info: &mut FuncInfo| -> ErrOR<()> {
        if let Json::Int(int) = &json.value {
          match int {
            Bind::Lit(l_int) => {
              if *l_int > i64::from(i32::MAX) || *l_int < i64::from(i32::MIN) {
                f_info.body.push(format!("  mov rcx, {l_int}\n"));
                f_info.body.push(format!("  {mne} rax, rcx\n"));
              } else {
                f_info.body.push(format!("  {mne} rax, {l_int}\n"));
              }
            }
            Bind::Local(local) | Bind::Tmp(local) => {
              if matches!(int, Bind::Tmp(_)) {
                f_info.free(*local, 8)?;
              }
              f_info.body.push(format!("  {mne} rax, {}\n", fmt_local("qword", *local)));
            }
            Bind::Var(var) => f_info.body.push(format!("  {mne} rax, {var}\n")),
          }
        } else {
          self.typ_err(ord, f_name, "Int", json)?;
        }
        Ok(())
      };
    if let Some(op_r) = args.first() {
      if args.len() == 1 && f_name == "-" {
        if let Json::Int(int) = &op_r.value {
          match int {
            Bind::Lit(l_int) => info.body.push(format!("  mov rax, {l_int}\n  neg rax\n")),
            Bind::Local(local) | Bind::Tmp(local) => {
              if matches!(int, Bind::Tmp(_)) {
                info.free(*local, 8)?;
              }
              info.body.push(format!("  mov rax, {}\n  neg rax\n", fmt_local("qword", *local)));
            }
            Bind::Var(var) => info.body.push(format!("  mov rax, {var}\n  neg rax\n")),
          }
        } else {
          self.typ_err(1, f_name, "Int", op_r)?;
        }
      } else {
        binary_mn(op_r, "mov", 1, info)?;
        for (ord, op_l) in args.iter().enumerate().skip(1) {
          binary_mn(op_l, mn, ord, info)?;
        }
      }
    } else {
      info.body.push(format!("  mov rax, {id_elem}\n"));
    }
    let ret = info.get_local(Align::U64)?;
    info.body.push(format!("  mov {}, rax\n", fmt_local("qword", ret)));
    Ok(Json::Int(Bind::Tmp(ret)))
  }
  /// Performs subtraction.
  fn op_minus(&mut self, _: &JsonWithPos, args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    self.op(args, info, "sub", "-", 0)
  }
  /// Performs addition.
  fn op_mul(&mut self, _: &JsonWithPos, args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    self.op(args, info, "imul", "*", 1)
  }
  /// Performs addition.
  fn op_plus(&mut self, _: &JsonWithPos, args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    self.op(args, info, "add", "+", 0)
  }
  /// Return the first argument.
  fn quote(&mut self, first: &JsonWithPos, mut args: Args, _: &mut FuncInfo) -> ErrOR<Json> {
    self.validate_args("'", false, 1, args.len(), &first.pos)?;
    Ok(take(args.first_mut().ok_or("Unreachable (quote)")?).value)
  }
  /// Sets a variable.
  fn set(
    &mut self, first: &JsonWithPos, mut args: Args, info: &mut FuncInfo, is_global: bool,
    f_name: &str,
  ) -> ErrOR<Json> {
    const ERR: &str = "Unreachable (set)";
    self.validate_args(f_name, false, 2, args.len(), &first.pos)?;
    let json1 = take(args.first_mut().ok_or(ERR)?);
    let Json::String(Bind::Lit(variable)) = json1.value else {
      return self.typ_err(1, f_name, "LString", &json1);
    };
    let json2 = take(args.get_mut(1).ok_or(ERR)?);
    let value = match json2.value {
      mut var if !var.is_literal() => var.tmp_to_local(),
      Json::String(Bind::Lit(st)) => Json::String(Bind::Var(self.get_global("STR", &st)?)),
      Json::Null => Json::Null,
      Json::Int(Bind::Lit(int)) => {
        if is_global {
          Json::Int(Bind::Var(self.get_global("INT", &int.to_string())?))
        } else {
          let offset = info.get_local(Align::U64)?;
          info.body.push(format!("  mov {}, {int}\n", fmt_local("qword", offset)));
          Json::Int(Bind::Local(offset))
        }
      }
      _ => return self.typ_err(2, "$", "that supports assignment", &json2),
    };
    if is_global {
      self.vars.first_mut().ok_or("InternalError: Invalid scope.")?
    } else {
      self.vars.last_mut().ok_or("InternalError: Invalid scope.")?
    }
    .insert(variable, value);
    Ok(Json::Null)
  }
  /// Sets a global variable.
  fn set_global(&mut self, first: &JsonWithPos, args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    self.set(first, args, info, true, "global")
  }
  /// Sets a local variable.
  fn set_local(&mut self, first: &JsonWithPos, args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    self.set(first, args, info, false, "=")
  }
  /// Gets the value of a local variable.
  #[expect(clippy::needless_pass_by_value, reason = "")]
  fn variable(&mut self, first: &JsonWithPos, args: Args, _: &mut FuncInfo) -> ErrOR<Json> {
    self.validate_args("$", false, 1, args.len(), &first.pos)?;
    let json1 = args.first().ok_or("Unreachable (variable)")?;
    let Json::String(Bind::Lit(var_name)) = &json1.value else {
      return self.typ_err(1, "$", "LString", json1);
    };
    self.get_var(var_name, &json1.pos)
  }
}
