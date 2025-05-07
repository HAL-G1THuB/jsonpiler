//! Built-in functions.
use crate::{
  AsmBool, AsmFunc,
  Bind::{self, Lit, Var},
  ErrOR, FuncInfo, GlobalKind, Json, JsonWithPos, Jsonpiler, Name, Position, ScopeInfo,
  VarKind::{Global, Tmp},
  add, err, include_once,
  utility::{get_int_str, mn},
};
use core::mem::take;
use std::collections::HashMap;
impl Jsonpiler {
  /// Registers all functions.
  pub(crate) fn all_register(&mut self) {
    let common = (false, false);
    let special = (true, false);
    let sp_scope = (true, true);
    self.register("lambda", special, Jsonpiler::lambda);
    self.register("scope", sp_scope, Jsonpiler::scope);
    self.register("global", common, Jsonpiler::set_global);
    self.register("=", common, Jsonpiler::set_local);
    self.register("message", common, Jsonpiler::message);
    self.register("'", special, Jsonpiler::quote);
    self.register("abs", common, Jsonpiler::abs);
    self.register("eval", common, Jsonpiler::f_eval);
    self.register("list", common, Jsonpiler::list);
    self.register("+", common, Jsonpiler::op_plus);
    self.register("%", common, Jsonpiler::op_rem);
    self.register("-", common, Jsonpiler::op_minus);
    self.register("*", common, Jsonpiler::op_mul);
    self.register("/", common, Jsonpiler::op_div);
    self.register("$", common, Jsonpiler::variable);
  }
}
#[expect(clippy::single_call_fn, clippy::needless_pass_by_value, reason = "")]
impl Jsonpiler {
  /// Absolute value.
  fn abs(&mut self, args: FuncInfo, info: &mut ScopeInfo) -> ErrOR<Json> {
    const ERR: &str = "Unreachable (abs)";
    self.validate_args(&args, false, 1)?;
    let json = args.args.first().ok_or(ERR)?;
    let Json::Int(int) = &json.value else {
      return self.typ_err(1, "abs", "Int", json);
    };
    let int_str = get_int_str(int, info)?;
    info.body.push(mn("mov", &["rax", &int_str]));
    info.body.push(mn("cqo", &[]));
    info.body.push(mn("xor", &["rax", "rdx"]));
    info.body.push(mn("sub", &["rax", "rdx"]));
    let ret = info.get_tmp(8)?;
    info.body.push(mn("mov", &[&format!("qword{ret}"), "rax"]));
    Ok(Json::Int(Var(ret)))
  }
  /// Return the first argument.
  fn f_eval(&mut self, mut args: FuncInfo, info: &mut ScopeInfo) -> ErrOR<Json> {
    self.validate_args(&args, false, 1)?;
    self.eval(take(args.args.first_mut().ok_or("Unreachable (eval)")?).value, info)
  }
  /// Evaluates a lambda function definition.
  fn lambda(&mut self, mut args: FuncInfo, _: &mut ScopeInfo) -> ErrOR<Json> {
    const ERR: &str = "Unreachable (lambda)";
    self.validate_args(&args, true, 2)?;
    let tmp_local_scope = self.vars.drain(1..).collect::<Vec<_>>();
    self.vars.push(HashMap::new());
    let mut info = ScopeInfo::default();
    let json1 = take(args.args.first_mut().ok_or(ERR)?);
    let Json::Array(Lit(params)) = json1.value else {
      return self.typ_err(1, "lambda", "LArray", &json1);
    };
    if !params.is_empty() {
      return err!(self, &json1.pos, "PARAMETERS IS NOT IMPLEMENTED.");
    }
    let name = self.get_global(&GlobalKind::Func, "")?.seed;
    let mut ret = Json::Null;
    for arg in args.args.get_mut(1..).ok_or(self.fmt_err("Empty lambda body.", &args.pos))? {
      ret = self.eval(take(arg).value, &mut info)?;
      ret.tmp().and_then(|tuple| info.free(tuple.0, tuple.1).ok());
    }
    self.text.push(mn(".seh_proc", &[&format!(".L{name:x}")]));
    self.text.push(format!(".L{name:x}:\n"));
    let mut registers = info.reg_used.iter().collect::<Vec<&String>>();
    registers.sort();
    for &reg in &registers {
      self.text.push(mn("push", &[reg]));
      self.text.push(mn(".seh_pushreg", &[reg]));
    }
    self.text.push(mn("push", &["rbp"]));
    self.text.push(mn(".seh_pushreg", &["rbp"]));
    let size = format!("0x{:x}", info.calc_alloc((info.reg_used.len() % 2).saturating_mul(8))?);
    self.text.push(format!(include_str!("asm/common/prologue.s"), size = size));
    for body in info.body {
      self.text.push(body);
    }
    if let Json::Int(int) = &ret {
      let int_str = match int {
        Lit(l_int) => l_int.to_string(),
        Var(bind_name) => format!("qword{bind_name}"),
      };
      self.text.push(mn("mov", &["rax", &int_str]));
    } else {
      self.text.push(mn("xor", &["eax", "eax"]));
    }
    self.text.push(mn("mov", &["rsp", "rbp"]));
    self.text.push(mn("pop", &["rbp"]));
    registers.reverse();
    for reg in registers {
      self.text.push(mn("pop", &[reg]));
    }
    self.text.push(mn("ret", &[]));
    self.text.push(mn(".seh_endproc", &[]));
    self.vars.extend(tmp_local_scope);
    Ok(Json::Function(AsmFunc { name, params, ret: Box::new(ret) }))
  }
  /// Return the arguments.
  #[expect(clippy::unnecessary_wraps, reason = "")]
  #[expect(clippy::unused_self, reason = "")]
  fn list(&mut self, args: FuncInfo, _: &mut ScopeInfo) -> ErrOR<Json> {
    Ok(Json::Array(Lit(args.args)))
  }
  /// Displays a message box.
  fn message(&mut self, mut args: FuncInfo, info: &mut ScopeInfo) -> ErrOR<Json> {
    const ERR: &str = "Unreachable (message)";
    self.validate_args(&args, false, 2)?;
    info.reg_used.insert("rdi".into());
    info.reg_used.insert("rsi".into());
    let title_json = take(args.args.first_mut().ok_or(ERR)?);
    let title = if let Json::String(st) = title_json.value {
      match st {
        Lit(l_str) => self.get_global(&GlobalKind::Str, &l_str)?,
        Var(name) => name,
      }
    } else {
      return self.typ_err(1, "message", "String", &title_json);
    };
    let msg_json = take(args.args.get_mut(1).ok_or(ERR)?);
    let msg = if let Json::String(st) = msg_json.value {
      match st {
        Lit(l_str) => self.get_global(&GlobalKind::Str, &l_str)?,
        Var(name) => name,
      }
    } else {
      return self.typ_err(2, "message", "String", &msg_json);
    };
    let ret = info.get_tmp(8)?;
    include_once!(self, self.text, "func/U8TO16");
    info.body.push(format!(
      include_str!("asm/caller/message.s"),
      title = title,
      msg = msg,
      ret = ret
    ));
    Ok(Json::Int(Var(ret)))
  }
  /// Utility functions for binary operations.
  fn op(&mut self, args: FuncInfo, info: &mut ScopeInfo, mne: &str, id_elem: usize) -> ErrOR<Json> {
    if let Some(op_r) = args.args.first() {
      if args.args.len() == 1 && args.name == "-" {
        if let Json::Int(int) = &op_r.value {
          let int_str = get_int_str(int, info)?;
          info.body.push(mn("mov", &["rax", &int_str]));
          info.body.push(mn("neg", &["rax"]));
        } else {
          self.typ_err(1, &args.name, "Int", op_r)?;
        }
      } else {
        self.op_mn(op_r, "mov", 1, info, &args.name)?;
        for (ord, op_l) in args.args.iter().enumerate().skip(1) {
          self.op_mn(op_l, mne, add(ord, 1)?, info, &args.name)?;
        }
      }
    } else {
      info.body.push(mn("mov", &["rax", &id_elem.to_string()]));
    }
    let ret = info.get_tmp(8)?;
    info.body.push(mn("mov", &[&format!("qword{ret}"), "rax"]));
    Ok(Json::Int(Var(ret)))
  }
  /// Performs division.
  fn op_div(&mut self, mut args: FuncInfo, info: &mut ScopeInfo) -> ErrOR<Json> {
    const ERR: &str = "Unreachable (div)";
    self.validate_args(&args, true, 2)?;
    let json1 = take(args.args.first_mut().ok_or(ERR)?);
    let Json::Int(int1) = json1.value else {
      return self.typ_err(1, "/", "Int", &json1);
    };
    let int_str1 = get_int_str(&int1, info)?;
    info.body.push(mn("mov", &["rax", &int_str1]));
    for (ord, op_l) in args.args.iter().enumerate().skip(1) {
      let Json::Int(int_l) = &op_l.value else {
        return self.typ_err(add(ord, 1)?, "/", "Int", op_l);
      };
      let int_str2 = self.op_nonzero_int_str(int_l, &op_l.pos, info)?;
      info.body.push(mn("cqo", &[]));
      info.body.push(mn("idiv", &[&int_str2]));
    }
    let ret = info.get_tmp(8)?;
    info.body.push(mn("mov", &[&format!("qword{ret}"), "rax"]));
    Ok(Json::Int(Var(ret)))
  }
  /// Performs subtraction.
  fn op_minus(&mut self, args: FuncInfo, info: &mut ScopeInfo) -> ErrOR<Json> {
    self.op(args, info, "sub", 0)
  }
  /// Write Binary operation mnemonic.
  fn op_mn(
    &self, json: &JsonWithPos, mne: &str, ord: usize, info: &mut ScopeInfo, f_name: &str,
  ) -> ErrOR<()> {
    if let Json::Int(int) = &json.value {
      let int_str = match int {
        Lit(l_int) => {
          if *l_int > i64::from(i32::MAX) || *l_int < i64::from(i32::MIN) {
            info.body.push(mn("mov", &["rcx", &l_int.to_string()]));
            "rcx".to_owned()
          } else {
            l_int.to_string()
          }
        }
        Var(name) => name.try_free_and_2str(info)?,
      };
      info.body.push(mn(mne, &["rax", &int_str]));
    } else {
      self.typ_err(ord, f_name, "Int", json)?;
    }
    Ok(())
  }
  /// Performs addition.
  fn op_mul(&mut self, args: FuncInfo, info: &mut ScopeInfo) -> ErrOR<Json> {
    self.op(args, info, "imul", 1)
  }
  /// Check zero or get int string.
  fn op_nonzero_int_str(
    &mut self, int: &Bind<i64>, pos: &Position, info: &mut ScopeInfo,
  ) -> ErrOR<String> {
    match int {
      Lit(l_int) => {
        if *l_int == 0 {
          return err!(self, pos, "ZeroDivisionError");
        }
        info.body.push(mn("mov", &["rcx", &l_int.to_string()]));
        Ok("rcx".to_owned())
      }
      Var(name) => {
        if name.var == Tmp {
          info.free(name.seed, 8)?;
        }
        let name_str = format!("qword{name}");
        info.body.push(mn("cmp", &[&name_str, "0"]));
        include_once!(self, self.data, "err/ZERO_DIVISION_MSG");
        include_once!(self, self.text, "err/ZERO_DIVISION_ERR");
        info.body.push(mn("jz", &[".L__ZERO_DIVISION_ERR"]));
        Ok(name_str)
      }
    }
  }
  /// Performs addition.
  fn op_plus(&mut self, args: FuncInfo, info: &mut ScopeInfo) -> ErrOR<Json> {
    self.op(args, info, "add", 0)
  }
  /// Performs remain.
  fn op_rem(&mut self, mut args: FuncInfo, info: &mut ScopeInfo) -> ErrOR<Json> {
    const ERR: &str = "Unreachable (%)";
    self.validate_args(&args, false, 2)?;
    let json1 = take(args.args.first_mut().ok_or(ERR)?);
    let Json::Int(int1) = json1.value else {
      return self.typ_err(1, "%", "Int", &json1);
    };
    let int_str1 = get_int_str(&int1, info)?;
    info.body.push(mn("mov", &["rax", &int_str1]));
    let json2 = take(args.args.get_mut(1).ok_or(ERR)?);
    let Json::Int(int_l) = json2.value else {
      return self.typ_err(2, "/", "Int", &json2);
    };
    let int_str2 = self.op_nonzero_int_str(&int_l, &json2.pos, info)?;
    info.body.push(mn("cqo", &[]));
    info.body.push(mn("idiv", &[&int_str2]));
    let ret = info.get_tmp(8)?;
    info.body.push(mn("mov", &[&format!("qword{ret}"), "rdx"]));
    Ok(Json::Int(Var(ret)))
  }
  /// Return the first argument.
  fn quote(&mut self, mut args: FuncInfo, _: &mut ScopeInfo) -> ErrOR<Json> {
    self.validate_args(&args, false, 1)?;
    Ok(take(args.args.first_mut().ok_or("Unreachable (quote)")?).value)
  }
  /// Evaluates a `scope` block.
  fn scope(&mut self, mut args: FuncInfo, info: &mut ScopeInfo) -> ErrOR<Json> {
    self.validate_args(&args, true, 1)?;
    let len = args.args.len();
    if len <= 1 {
      return self.eval(take(args.args.last_mut().ok_or("Unreachable (scope)")?).value, info);
    }
    for arg in args.args.get_mut(1..len.saturating_sub(1)).unwrap_or(&mut []) {
      let val = self.eval(take(arg).value, info)?;
      if let Some((addr, size)) = val.tmp() {
        info.free(addr, size)?;
      }
    }
    self.eval(take(args.args.last_mut().ok_or("Unreachable (scope)")?).value, info)
  }
  /// Sets a variable.
  fn set(&mut self, mut args: FuncInfo, info: &mut ScopeInfo, is_global: bool) -> ErrOR<Json> {
    const ERR: &str = "Unreachable (set)";
    self.validate_args(&args, false, 2)?;
    let json1 = take(args.args.first_mut().ok_or(ERR)?);
    let Json::String(Lit(variable)) = json1.value else {
      return self.typ_err(1, &args.name, "LString", &json1);
    };
    let json2 = take(args.args.get_mut(1).ok_or(ERR)?);
    let value = match json2.value {
      Json::Function(func) => {
        if self.builtin.contains_key(&variable) {
          return err!(
            self,
            args.pos,
            "The variable name of this function object already exists as a built-in function"
          );
        }
        Json::Function(func)
      }
      var @ (Json::VBool(AsmBool { seed: Name { var: Global, .. }, .. })
      | Json::Object(Var(Name { var: Global, .. }))
      | Json::Float(Var(Name { var: Global, .. }))
      | Json::Int(Var(Name { var: Global, .. }))
      | Json::String(Var(Name { var: Global, .. }))
      | Json::Array(Var(Name { var: Global, .. }))) => var,
      mut var @ (Json::VBool(_)
      | Json::Object(Var(_))
      | Json::Float(Var(_))
      | Json::Int(Var(_))
      | Json::String(Var(_))
      | Json::Array(Var(_)))
        if !is_global =>
      {
        var.tmp_to_local()
      }
      Json::String(Var(_)) if is_global => {
        return err!(self, json2.pos, "Local string cannot be assigned to a global variable.");
      }
      Json::String(Lit(st)) => Json::String(Var(self.get_global(&GlobalKind::Str, &st)?)),
      Json::Null => Json::Null,
      Json::Int(Lit(int)) => {
        if is_global {
          Json::Int(Var(self.get_global(&GlobalKind::Int, &int.to_string())?))
        } else {
          let name = info.get_local(8)?;
          info.body.push(mn("mov", &[&format!("qword{name}"), &int.to_string()]));
          Json::Int(Var(name))
        }
      }
      Json::Int(int @ Var(_)) => {
        let name = self.get_global(&GlobalKind::Bss, "8")?;
        let int_str = get_int_str(&int, info)?;
        info.body.push(mn("mov", &[&format!("qword{name}"), &int_str]));
        Json::Int(Var(name))
      }
      Json::Float(Lit(float)) => {
        if is_global {
          Json::Int(Var(self.get_global(&GlobalKind::Float, &format!("0x{:x}", float.to_bits()))?))
        } else {
          let name = info.get_local(8)?;
          info
            .body
            .push(mn("mov", &[&format!("qword{name}"), &format!("0x{:x}", float.to_bits())]));
          Json::Int(Var(name))
        }
      }
      Json::Float(Var(local)) => {
        let name = self.get_global(&GlobalKind::Bss, "8")?;
        info.body.push(mn("mov", &[&format!("qword{name}"), &format!("qword{local}")]));
        Json::Int(Var(name))
      }
      Json::Array(_) | Json::LBool(_) | Json::Object(_) | Json::String(_) | Json::VBool(_) => {
        return self.typ_err(2, "=` and `global", "that supports assignment", &json2);
      }
    };
    if if is_global {
      self.vars.first_mut().ok_or("InternalError: Invalid scope.")?
    } else {
      self.vars.last_mut().ok_or("InternalError: Invalid scope.")?
    }
    .insert(variable, value)
    .is_some()
    {
      return err!(self, "Reassignment may not be possible in some scope.");
    }
    Ok(Json::Null)
  }
  /// Sets a global variable.
  fn set_global(&mut self, args: FuncInfo, info: &mut ScopeInfo) -> ErrOR<Json> {
    self.set(args, info, true)
  }
  /// Sets a local variable.
  fn set_local(&mut self, args: FuncInfo, info: &mut ScopeInfo) -> ErrOR<Json> {
    self.set(args, info, false)
  }
  /// Gets the value of a local variable.
  #[expect(clippy::needless_pass_by_value, reason = "")]
  fn variable(&mut self, args: FuncInfo, _: &mut ScopeInfo) -> ErrOR<Json> {
    self.validate_args(&args, false, 1)?;
    let json1 = args.args.first().ok_or("Unreachable (variable)")?;
    if let Json::String(Lit(var_name)) = &json1.value {
      self.get_var(var_name, &json1.pos)
    } else {
      self.typ_err(1, "$", "LString", json1)
    }
  }
}
