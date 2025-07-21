//! Built-in functions.
use crate::{
  AsmFunc,
  Bind::{self, Lit, Var},
  ErrOR, FuncInfo, Json, JsonWithPos, Jsonpiler, Position, ScopeInfo, add, err, include_once, mn,
  utility::get_int_str,
};
use core::mem::{replace, take};
use std::collections::HashMap;
impl Jsonpiler {
  /// Registers all functions.
  pub(crate) fn all_register(&mut self) {
    let common = (false, false);
    let special = (false, true);
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
  fn abs(&mut self, func: FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.validate_args(&func, false, 1)?;
    let json = &func.args[0];
    let Json::Int(int) = &json.value else {
      return self.typ_err(1, &func.name, "Int", json);
    };
    let int_str = get_int_str(int, scope)?;
    scope.body.push(mn!("mov", "rax", &int_str));
    scope.body.push(mn!("cqo"));
    scope.body.push(mn!("xor", "rax", "rdx"));
    scope.body.push(mn!("sub", "rax", "rdx"));
    let ret = scope.get_tmp(8)?;
    scope.body.push(mn!("mov", &format!("qword{ret}"), "rax"));
    Ok(Json::Int(Var(ret)))
  }
  /// Returns the first argument.
  fn f_eval(&mut self, mut func: FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.validate_args(&func, false, 1)?;
    self.eval(take(&mut func.args[0]).value, scope)
  }
  /// Evaluates a lambda function definition.
  fn lambda(&mut self, mut func: FuncInfo, _: &mut ScopeInfo) -> ErrOR<Json> {
    self.validate_args(&func, true, 2)?;
    let mut tmp_local_scope = replace(&mut self.vars_local, vec![HashMap::new()]);
    let mut scope = ScopeInfo::default();
    let json1 = take(&mut func.args[0]);
    let Json::Array(Lit(params)) = json1.value else {
      return self.typ_err(1, &func.name, "Array (Literal)", &json1);
    };
    if !params.is_empty() {
      return err!(self, &json1.pos, "PARAMETERS HAS BEEN NOT IMPLEMENTED.");
    }
    let func_name = self.get_global_label()?;
    let dec_len = func.args.len().saturating_sub(1);
    for arg in &mut func.args[1..dec_len] {
      let evaluated = self.eval(take(arg).value, &mut scope)?;
      if let Some((end, size)) = evaluated.tmp() {
        scope.free(end, size)?;
      }
    }
    let ret = Box::new(self.eval(take(&mut func.args[dec_len]).value, &mut scope)?);
    self.text.push(mn!(".seh_proc", func_name.to_ref()));
    self.text.push(func_name.to_def());
    let mut registers = scope.reg_used.iter().collect::<Vec<&String>>();
    registers.sort();
    for &reg in &registers {
      self.text.push(mn!("push", reg.as_str()));
      self.text.push(mn!(".seh_pushreg", reg.as_str()));
    }
    self.text.push(mn!("push", "rbp"));
    self.text.push(mn!(".seh_pushreg", "rbp"));
    let size = format!("{:#x}", scope.calc_alloc((scope.reg_used.len() % 2).saturating_mul(8))?);
    self.text.push(format!(include_str!("asm/common/prologue.s"), size = size));
    for body in scope.body {
      self.text.push(body);
    }
    if let Json::Int(int) = &*ret {
      let int_str = match int {
        Lit(l_int) => l_int.to_string(),
        Var(bind_name) => format!("qword{bind_name}"),
      };
      self.text.push(mn!("mov", "rax", &int_str));
    } else {
      self.text.push(mn!("xor", "eax", "eax"));
    }
    self.text.push(mn!("mov", "rsp", "rbp"));
    self.text.push(mn!("pop", "rbp"));
    registers.reverse();
    for reg in registers {
      self.text.push(mn!("pop", reg.as_str()));
    }
    self.text.push(mn!("ret"));
    self.text.push(mn!(".seh_endproc"));
    self.vars_local = take(&mut tmp_local_scope);
    Ok(Json::Function(AsmFunc { name: func_name, params, ret }))
  }
  /// Returns the arguments.
  #[expect(clippy::unnecessary_wraps, reason = "")]
  #[expect(clippy::unused_self, reason = "")]
  fn list(&mut self, func: FuncInfo, _: &mut ScopeInfo) -> ErrOR<Json> {
    Ok(Json::Array(Lit(func.args)))
  }
  /// Displays a message box.
  fn message(&mut self, mut func: FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.validate_args(&func, false, 2)?;
    scope.reg_used.insert("rdi".into());
    scope.reg_used.insert("rsi".into());
    let title_json = take(&mut func.args[0]);
    let title = if let Json::String(st) = title_json.value {
      match st {
        Lit(l_str) => self.get_global_str(&l_str)?,
        Var(name) => name,
      }
    } else {
      return self.typ_err(1, &func.name, "String", &title_json);
    };
    let msg_json = take(&mut func.args[1]);
    let msg = if let Json::String(st) = msg_json.value {
      match st {
        Lit(l_str) => self.get_global_str(&l_str)?,
        Var(name) => name,
      }
    } else {
      return self.typ_err(2, &func.name, "String", &msg_json);
    };
    let ret = scope.get_tmp(8)?;
    include_once!(self, self.text, "func/U8TO16");
    scope.body.push(format!(
      include_str!("asm/caller/message.s"),
      title = title,
      msg = msg,
      ret = ret
    ));
    Ok(Json::Int(Var(ret)))
  }
  /// Utility functions for binary operations.
  fn op(
    &mut self, func: FuncInfo, scope: &mut ScopeInfo, mne: &str, identity_element: usize,
  ) -> ErrOR<Json> {
    if let Some(op_r) = func.args.first() {
      self.op_mn(op_r, "mov", 1, scope, &func.name)?;
      for (ord, op_l) in func.args.iter().enumerate().skip(1) {
        self.op_mn(op_l, mne, add(ord, 1)?, scope, &func.name)?;
      }
    } else {
      scope.body.push(mn!("mov", "rax", &identity_element.to_string()));
    }
    let ret = scope.get_tmp(8)?;
    scope.body.push(mn!("mov", &format!("qword{ret}"), "rax"));
    Ok(Json::Int(Var(ret)))
  }
  /// Performs division.
  fn op_div(&mut self, mut func: FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.validate_args(&func, true, 2)?;
    let json1 = take(&mut func.args[0]);
    let Json::Int(int1) = json1.value else {
      return self.typ_err(1, &func.name, "Int", &json1);
    };
    let int_str1 = get_int_str(&int1, scope)?;
    scope.body.push(mn!("mov", "rax", &int_str1));
    for (ord, op_l) in func.args.iter().enumerate().skip(1) {
      let Json::Int(int_l) = &op_l.value else {
        return self.typ_err(add(ord, 1)?, &func.name, "Int", op_l);
      };
      let int_str2 = self.op_nonzero_int_str(int_l, &op_l.pos, scope)?;
      scope.body.push(mn!("cqo"));
      scope.body.push(mn!("idiv", int_str2));
    }
    let ret = scope.get_tmp(8)?;
    scope.body.push(mn!("mov", &format!("qword{ret}"), "rax"));
    Ok(Json::Int(Var(ret)))
  }
  /// Performs subtraction.
  fn op_minus(&mut self, func: FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    if let Some(first) = func.args.first() {
      if func.args.len() == 1 {
        if let Json::Int(int) = &first.value {
          let int_str = get_int_str(int, scope)?;
          scope.body.push(mn!("mov", "rax", &int_str));
          scope.body.push(mn!("neg", "rax"));
          let ret = scope.get_tmp(8)?;
          scope.body.push(mn!("mov", &format!("qword{ret}"), "rax"));
          Ok(Json::Int(Var(ret)))
        } else {
          self.typ_err(1, &func.name, "Int", first)
        }
      } else {
        self.op(func, scope, "sub", 0)
      }
    } else {
      let ret = scope.get_tmp(8)?;
      scope.body.push(mn!("mov", &format!("qword{ret}"), "0"));
      Ok(Json::Int(Var(ret)))
    }
  }
  /// Writes Binary operation mnemonic.
  fn op_mn(
    &self, json: &JsonWithPos, mne: &str, ord: usize, scope: &mut ScopeInfo, f_name: &str,
  ) -> ErrOR<()> {
    if let Json::Int(int) = &json.value {
      let int_str = match int {
        Lit(l_int) => {
          if *l_int > i64::from(i32::MAX) || *l_int < i64::from(i32::MIN) {
            scope.body.push(mn!("mov", "rcx", &l_int.to_string()));
            "rcx".to_owned()
          } else {
            l_int.to_string()
          }
        }
        Var(name) => name.try_free_and_2str(scope)?,
      };
      scope.body.push(mn!(mne, "rax", &int_str));
    } else {
      self.typ_err(ord, f_name, "Int", json)?;
    }
    Ok(())
  }
  /// Performs addition.
  fn op_mul(&mut self, func: FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.op(func, scope, "imul", 1)
  }
  /// Checks zero otherwise gets integer string.
  fn op_nonzero_int_str(
    &mut self, int: &Bind<i64>, pos: &Position, scope: &mut ScopeInfo,
  ) -> ErrOR<String> {
    match int {
      Lit(l_int) => {
        if *l_int == 0 {
          return err!(self, pos, "ZeroDivisionError");
        }
        scope.body.push(mn!("mov", "rcx", &l_int.to_string()));
        Ok("rcx".to_owned())
      }
      Var(name) => {
        let name_str = name.try_free_and_2str(scope)?;
        scope.body.push(mn!("cmp", &name_str, "0"));
        include_once!(self, self.data, "err/ZERO_DIVISION_MSG");
        include_once!(self, self.text, "err/ZERO_DIVISION_ERR");
        scope.body.push(mn!("jz", ".L__ZERO_DIVISION_ERR"));
        Ok(name_str)
      }
    }
  }
  /// Performs addition.
  fn op_plus(&mut self, func: FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.op(func, scope, "add", 0)
  }
  /// Performs remain.
  fn op_rem(&mut self, mut func: FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.validate_args(&func, false, 2)?;
    let json1 = take(&mut func.args[0]);
    let Json::Int(int1) = json1.value else {
      return self.typ_err(1, &func.name, "Int", &json1);
    };
    let int_str1 = get_int_str(&int1, scope)?;
    scope.body.push(mn!("mov", "rax", &int_str1));
    let json2 = take(&mut func.args[1]);
    let Json::Int(int_l) = json2.value else {
      return self.typ_err(2, &func.name, "Int", &json2);
    };
    let int_str2 = self.op_nonzero_int_str(&int_l, &json2.pos, scope)?;
    scope.body.push(mn!("cqo"));
    scope.body.push(mn!("idiv", int_str2));
    let ret = scope.get_tmp(8)?;
    scope.body.push(mn!("mov", &format!("qword{ret}"), "rdx"));
    Ok(Json::Int(Var(ret)))
  }
  /// Returns the first argument without evaluating it.
  fn quote(&mut self, mut func: FuncInfo, _: &mut ScopeInfo) -> ErrOR<Json> {
    self.validate_args(&func, false, 1)?;
    Ok(take(&mut func.args[0]).value)
  }
  /// Evaluates a `scope` block.
  fn scope(&mut self, mut func: FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.validate_args(&func, true, 1)?;
    let dec_len = func.args.len().saturating_sub(1);
    let last = take(&mut func.args[dec_len]).value;
    for arg in &mut func.args[..dec_len] {
      let val = self.eval(take(arg).value, scope)?;
      if let Some((end, size)) = val.tmp() {
        scope.free(end, size)?;
      }
    }
    self.eval(last, scope)
  }
  /// Sets a variable.
  fn set(&mut self, mut func: FuncInfo, scope: &mut ScopeInfo, is_global: bool) -> ErrOR<Json> {
    self.validate_args(&func, false, 2)?;
    let json1 = take(&mut func.args[0]);
    let Json::String(Lit(variable)) = json1.value else {
      return self.typ_err(1, &func.name, "String (Literal)", &json1);
    };
    let json2 = take(&mut func.args[1]);
    let value = match json2.value {
      Json::Function(asm_func) => {
        if self.builtin.contains_key(&variable) {
          return err!(self, json1.pos, "Name conflict with a built-in function.");
        }
        Json::Function(asm_func)
      }
      Json::String(Lit(st)) => Json::String(Var(self.get_global_str(&st)?)),
      var @ Json::String(Var(_)) => {
        if is_global {
          return err!(self, json2.pos, "Local string cannot be assigned to a global variable.");
        }
        var
      }
      Json::Null => Json::Null,
      Json::Int(Lit(int)) if is_global => Json::Int(Var(self.get_global_int(int)?)),
      Json::Int(Lit(int)) => {
        let name = scope.get_local(8)?;
        scope.body.push(mn!("mov", format!("qword{name}"), int.to_string()));
        Json::Int(Var(name))
      }
      Json::Int(int @ Var(_)) => {
        let name = if is_global { self.get_global_bss(8) } else { scope.get_local(8) }?;
        let int_str = get_int_str(&int, scope)?;
        scope.body.push(mn!("mov", format!("qword{name}"), int_str));
        Json::Int(Var(name))
      }
      Json::LBool(l_bool) => {
        let asm_bool = if is_global { self.get_global_bool() } else { scope.get_bool_local() }?;
        let bit_mask = 1u8 << asm_bool.bit;
        if l_bool {
          scope.body.push(mn!(
            "or",
            format!("byte{}", asm_bool.name),
            format!("{:#010b}", bit_mask)
          ));
        } else {
          scope.body.push(mn!(
            "and",
            format!("byte{}", asm_bool.name),
            format!("{:#010b}", !bit_mask)
          ));
        }
        Json::VBool(asm_bool)
      }
      Json::VBool(asm_bool) => {
        let new_asm_bool = if is_global { self.get_global_bool() } else { scope.get_bool_local() }?;
        let src_name_str = format!("byte{}", asm_bool.name);
        scope.body.push(mn!("movzx", "eax", &src_name_str));
        scope.body.push(mn!("and", "al", &format!("{:#010b}", 1u8 << asm_bool.bit)));
        let dest_name = format!("byte{}", asm_bool.name);
        let new_bit_mask = 1u8 << new_asm_bool.bit;
        let false_label = self.get_global_label()?;
        let end_label = self.get_global_label()?;
        scope.body.push(mn!("test", "al", "al"));
        scope.body.push(mn!("jz", false_label.to_ref()));
        scope.body.push(mn!("or", dest_name.as_str(), &format!("{new_bit_mask:#010b}")));
        scope.body.push(mn!("jmp", end_label.to_ref()));
        scope.body.push(false_label.to_def());
        scope.body.push(mn!("and", dest_name.as_str(), &format!("{new_bit_mask:#010b}")));
        scope.body.push(end_label.to_def());
        Json::VBool(new_asm_bool)
      }
      Json::Float(Lit(float)) if is_global => {
        Json::Int(Var(self.get_global_float(float.to_bits())?))
      }
      Json::Float(Lit(float)) => {
        let name = scope.get_local(8)?;
        scope.body.push(mn!("mov", format!("qword{name}"), format!("{:#x}", float.to_bits())));
        Json::Int(Var(name))
      }
      Json::Float(Var(float)) => {
        let name = if is_global { self.get_global_bss(8) } else { scope.get_local(8) }?;
        scope.body.push(mn!("mov", format!("qword{name}"), format!("qword{float}")));
        Json::Int(Var(name))
      }
      Json::Array(_) | Json::Object(_) => {
        return self.typ_err(
          2,
          &func.name,
          "that supports assignment (excluding Array and Object)",
          &json2,
        );
      }
    };
    if if is_global {
      &mut self.vars_global
    } else {
      self.vars_local.last_mut().ok_or("InternalError: Invalid scope.")?
    }
    .insert(variable, value)
    .is_some()
    {
      return err!(self, json1.pos, "Reassignment may not be possible in some scope.");
    }
    Ok(Json::Null)
  }
  /// Sets a global variable.
  fn set_global(&mut self, func: FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.set(func, scope, true)
  }
  /// Sets a local variable.
  fn set_local(&mut self, func: FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.set(func, scope, false)
  }
  /// Gets the value of a local variable.
  fn variable(&mut self, func: FuncInfo, _: &mut ScopeInfo) -> ErrOR<Json> {
    self.validate_args(&func, false, 1)?;
    let json1 = &func.args[0];
    if let Json::String(Lit(var_name)) = &json1.value {
      self.get_var(var_name, &json1.pos)
    } else {
      self.typ_err(1, &func.name, "String (Literal)", json1)
    }
  }
}
