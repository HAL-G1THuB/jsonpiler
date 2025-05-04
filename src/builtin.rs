//! Built-in functions.
use crate::{
  Args, AsmFunc, Bind, ErrOR, FuncInfo, GlobalKind, Json, JsonWithPos, Jsonpiler, Name,
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
#[expect(clippy::single_call_fn, reason = "")]
impl Jsonpiler {
  /// Absolute value.
  fn abs(&mut self, first: &JsonWithPos, mut args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    const ERR: &str = "Unreachable (abs)";
    self.validate_args("abs", false, 1, args.len(), &first.pos)?;
    let json1 = take(args.first_mut().ok_or(ERR)?);
    let Json::Int(int) = json1.value else {
      return self.typ_err(1, "abs", "Int", &json1);
    };
    let int_str = match int {
      Bind::Lit(l_int) => l_int.to_string(),
      Bind::Var(name) => {
        if name.var == Tmp {
          info.free(name.seed, 8)?;
        }
        format!("qword{name}")
      }
    };
    info.body.push(mn("mov", &["rax", &int_str]));
    info.body.push(mn("cqo", &[]));
    info.body.push(mn("xor", &["rax", "rdx"]));
    info.body.push(mn("sub", &["rax", "rdx"]));
    let ret = info.get_local(8)?;
    info.body.push(mn("mov", &[&format!("qword{ret}"), "rax"]));
    Ok(Json::Int(Bind::Var(Name { var: Tmp, ..ret })))
  }
  /// Return the first argument.
  fn f_eval(&mut self, first: &JsonWithPos, mut args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    self.validate_args("eval", false, 1, args.len(), &first.pos)?;
    self.eval(take(args.first_mut().ok_or("Unreachable (eval)")?).value, info)
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
    let name = self.get_global(&GlobalKind::Fnc, "")?.seed;
    let mut ret = Json::Null;
    for arg in args.get_mut(1..).ok_or(self.fmt_err("Empty lambda body.", &first.pos))? {
      ret = self.eval(take(arg).value, &mut info)?;
      ret.tmp().and_then(|tuple| info.free(tuple.0, tuple.1).ok());
    }
    self.text.push(mn(".seh_proc", &[&format!(".L{name:x}")]));
    self.text.push(format!(".L{name:x}:\n"));
    let mut registers: Vec<&String> = info.reg_used.iter().collect();
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
        Bind::Lit(l_int) => l_int.to_string(),
        Bind::Var(bind_name) => format!("qword{bind_name}"),
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
    let title_json = take(args.first_mut().ok_or(ERR)?);
    let title = if let Json::String(st) = title_json.value {
      match st {
        Bind::Lit(l_str) => self.get_global(&GlobalKind::Str, &l_str)?,
        Bind::Var(name) => name,
      }
    } else {
      return self.typ_err(1, "message", "String", &title_json);
    };
    let msg_json = take(args.get_mut(1).ok_or(ERR)?);
    let msg = if let Json::String(st) = msg_json.value {
      match st {
        Bind::Lit(l_str) => self.get_global(&GlobalKind::Str, &l_str)?,
        Bind::Var(name) => name,
      }
    } else {
      return self.typ_err(2, "message", "String", &msg_json);
    };
    let ret = info.get_local(8)?;
    include_once!(self, self.text, "func/U8TO16");
    info.body.push(format!(
      include_str!("asm/caller/message.s"),
      title = title,
      msg = msg,
      ret = ret
    ));
    Ok(Json::Int(Bind::Var(Name { var: Tmp, ..ret })))
  }
  /// Utility functions for binary operations.
  #[expect(clippy::needless_pass_by_value, reason = "")]
  fn op(
    &mut self, args: Args, info: &mut FuncInfo, mne: &str, f_name: &str, id_elem: usize,
  ) -> ErrOR<Json> {
    if let Some(op_r) = args.first() {
      if args.len() == 1 && f_name == "-" {
        if let Json::Int(int) = &op_r.value {
          let int_str = get_int_str(int, info)?;
          info.body.push(mn("mov", &["rax", &int_str]));
          info.body.push(mn("neg", &["rax"]));
        } else {
          self.typ_err(1, f_name, "Int", op_r)?;
        }
      } else {
        self.op_mn(op_r, "mov", 1, info, f_name)?;
        for (ord, op_l) in args.iter().enumerate().skip(1) {
          self.op_mn(op_l, mne, add(ord, 1)?, info, f_name)?;
        }
      }
    } else {
      info.body.push(mn("mov", &["rax", &id_elem.to_string()]));
    }
    let ret = info.get_local(8)?;
    info.body.push(mn("mov", &[&format!("qword{ret}"), "rax"]));
    Ok(Json::Int(Bind::Var(Name { var: Tmp, ..ret })))
  }
  /// Performs division.
  fn op_div(&mut self, first: &JsonWithPos, mut args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    const ERR: &str = "Unreachable (div)";
    self.validate_args("/", true, 2, args.len(), &first.pos)?;
    let json1 = take(args.first_mut().ok_or(ERR)?);
    let Json::Int(int1) = json1.value else {
      return self.typ_err(1, "/", "Int", &json1);
    };
    let int_str1 = get_int_str(&int1, info)?;
    info.body.push(mn("mov", &["rax", &int_str1]));
    for (ord, op_l) in args.iter().enumerate().skip(1) {
      let Json::Int(int_l) = &op_l.value else {
        return self.typ_err(add(ord, 1)?, "/", "Int", op_l);
      };
      let int_str2 = match int_l {
        Bind::Lit(l_int) => {
          if *l_int == 0 {
            return err!(self, &op_l.pos, "ZeroDivisionError");
          }
          info.body.push(mn("mov", &["rcx", &l_int.to_string()]));
          "rcx".to_owned()
        }
        Bind::Var(name) => {
          if name.var == Tmp {
            info.free(name.seed, 8)?;
          }
          let name_str = format!("qword{name}");
          info.body.push(mn("cmp", &[&name_str, "0"]));
          include_once!(self, self.data, "err/ZERO_DIVISION_MSG");
          include_once!(self, self.text, "err/ZERO_DIVISION_ERR");
          info.body.push(mn("jz", &[".L__ZERO_DIVISION_ERR"]));
          name_str
        }
      };
      info.body.push(mn("cqo", &[]));
      info.body.push(mn("idiv", &[&int_str2]));
    }
    let ret = info.get_tmp(8)?;
    info.body.push(mn("mov", &[&format!("qword{ret}"), "rax"]));
    Ok(Json::Int(Bind::Var(ret)))
  }
  /// Performs subtraction.
  fn op_minus(&mut self, _: &JsonWithPos, args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    self.op(args, info, "sub", "-", 0)
  }
  /// Write Binary operation mnemonic.
  fn op_mn(
    &self, json: &JsonWithPos, mne: &str, ord: usize, info: &mut FuncInfo, f_name: &str,
  ) -> ErrOR<()> {
    if let Json::Int(int) = &json.value {
      let int_str = match int {
        Bind::Lit(l_int) => {
          if *l_int > i64::from(i32::MAX) || *l_int < i64::from(i32::MIN) {
            info.body.push(mn("mov", &["rcx", &l_int.to_string()]));
            "rcx".to_owned()
          } else {
            l_int.to_string()
          }
        }
        Bind::Var(name) => {
          if name.var == Tmp {
            info.free(name.seed, 8)?;
          }
          format!("qword{name}")
        }
      };
      info.body.push(mn(mne, &["rax", &int_str]));
    } else {
      self.typ_err(ord, f_name, "Int", json)?;
    }
    Ok(())
  }
  /// Performs addition.
  fn op_mul(&mut self, _: &JsonWithPos, args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    self.op(args, info, "imul", "*", 1)
  }
  /// Performs addition.
  fn op_plus(&mut self, _: &JsonWithPos, args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    self.op(args, info, "add", "+", 0)
  }
  /// Performs remain.
  fn op_rem(&mut self, first: &JsonWithPos, mut args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    const ERR: &str = "Unreachable (%)";
    self.validate_args("%", false, 2, args.len(), &first.pos)?;
    let json1 = take(args.first_mut().ok_or(ERR)?);
    let Json::Int(int1) = json1.value else {
      return self.typ_err(1, "%", "Int", &json1);
    };
    let int_str1 = get_int_str(&int1, info)?;
    info.body.push(mn("mov", &["rax", &int_str1]));
    let json2 = take(args.get_mut(1).ok_or(ERR)?);
    let Json::Int(int_l) = json2.value else {
      return self.typ_err(2, "/", "Int", &json2);
    };
    let int_str2 = match int_l {
      Bind::Lit(l_int) => {
        if l_int == 0 {
          return err!(self, &json2.pos, "ZeroDivisionError");
        }
        info.body.push(mn("mov", &["rcx", &l_int.to_string()]));
        "rcx".to_owned()
      }
      Bind::Var(name) => {
        if name.var == Tmp {
          info.free(name.seed, 8)?;
        }
        let name_str = format!("qword{name}");
        info.body.push(mn("cmp", &[&name_str, "0"]));
        include_once!(self, self.data, "err/ZERO_DIVISION_MSG");
        include_once!(self, self.text, "err/ZERO_DIVISION_ERR");
        info.body.push(mn("jz", &[".L__ZERO_DIVISION_ERR"]));
        name_str
      }
    };
    info.body.push(mn("cqo", &[]));
    info.body.push(mn("idiv", &[&int_str2]));
    let ret = info.get_tmp(8)?;
    info.body.push(mn("mov", &[&format!("qword{ret}"), "rdx"]));
    Ok(Json::Int(Bind::Var(ret)))
  }
  /// Return the first argument.
  fn quote(&mut self, first: &JsonWithPos, mut args: Args, _: &mut FuncInfo) -> ErrOR<Json> {
    self.validate_args("'", false, 1, args.len(), &first.pos)?;
    Ok(take(args.first_mut().ok_or("Unreachable (quote)")?).value)
  }
  /// Evaluates a `scope` block.
  fn scope(&mut self, first: &JsonWithPos, mut args: Args, info: &mut FuncInfo) -> ErrOR<Json> {
    self.validate_args("scope", true, 1, args.len(), &first.pos)?;
    let len = args.len();
    if len <= 1 {
      return self.eval(take(args.last_mut().ok_or("Unreachable (scope)")?).value, info);
    }
    for arg in args.get_mut(1..len.saturating_sub(1)).unwrap_or(&mut []) {
      let val = self.eval(take(arg).value, info)?;
      if let Some((addr, size)) = val.tmp() {
        info.free(addr, size)?;
      }
    }
    self.eval(take(args.last_mut().ok_or("Unreachable (scope)")?).value, info)
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
      Json::Function(func) => {
        if self.builtin.contains_key(&variable) {
          return err!(
            self,
            first.pos,
            "The variable name of this function object already exists as a built-in function"
          );
        }
        Json::Function(func)
      }
      var if var.var() == Some(Global) && is_global => var,
      mut var if var.var().is_some() && !is_global => var.tmp_to_local(),
      Json::String(Bind::Var(_)) if is_global => {
        return err!(self, json2.pos, "Local string cannot be assigned to a global variable.");
      }
      Json::Int(Bind::Var(local)) => {
        let name = self.get_global(&GlobalKind::Bss, "8")?;
        info.body.push(mn("mov", &[&format!("qword{name}"), &format!("qword{local}")]));
        Json::Int(Bind::Var(name))
      }
      Json::String(Bind::Lit(st)) => {
        Json::String(Bind::Var(self.get_global(&GlobalKind::Str, &st)?))
      }
      Json::Null => Json::Null,
      Json::Int(Bind::Lit(int)) => {
        if is_global {
          Json::Int(Bind::Var(self.get_global(&GlobalKind::Int, &int.to_string())?))
        } else {
          let name = info.get_local(8)?;
          info.body.push(mn("mov", &[&format!("qword{name}"), &int.to_string()]));
          Json::Int(Bind::Var(name))
        }
      }
      _ => return self.typ_err(2, "=` and `global", "that supports assignment", &json2),
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
    if let Json::String(Bind::Lit(var_name)) = &json1.value {
      self.get_var(var_name, &json1.pos)
    } else {
      self.typ_err(1, "$", "LString", json1)
    }
  }
}
