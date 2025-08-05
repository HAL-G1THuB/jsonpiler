use super::super::{
  ArgLen::{Any, AtLeast},
  AsmFunc,
  Bind::{Lit, Var},
  ErrOR, FuncInfo, Json, Jsonpiler, ScopeInfo, err, mn, validate_type,
};
use core::mem::{replace, take};
use std::collections::HashMap;
impl Jsonpiler {
  pub(crate) fn register_construct(&mut self) {
    let common = (false, false);
    let special = (false, true);
    self.register("lambda", special, Jsonpiler::lambda, AtLeast(2));
    self.register("list", common, Jsonpiler::list, Any);
  }
}
#[expect(clippy::single_call_fn, reason = "")]
impl Jsonpiler {
  fn lambda(&mut self, mut func: FuncInfo, _: &mut ScopeInfo) -> ErrOR<Json> {
    let mut tmp_local_scope = replace(&mut self.vars_local, vec![HashMap::new()]);
    let mut scope = ScopeInfo::default();
    let json1 = func.arg()?;
    let params = validate_type!(self, func, 1, json1, Json::Array(Lit(x)) => x, "Array (Literal)");
    if !params.is_empty() {
      return err!(self, &json1.pos, "PARAMETERS HAS BEEN NOT IMPLEMENTED.");
    }
    let label = self.get_label(8)?;
    for _ in 2..func.len {
      let evaluated = self.eval(func.arg()?.value, &mut scope)?;
      scope.drop_json(evaluated)?;
    }
    let ret = Box::new(self.eval(func.arg()?.value, &mut scope)?);
    self.text.push(mn!(".seh_proc", label.to_ref()));
    self.text.push(label.to_def());
    let mut registers = scope.reg_used.iter().collect::<Vec<&String>>();
    registers.sort();
    for &reg in &registers {
      self.text.push(mn!("push", reg));
      self.text.push(mn!(".seh_pushreg", reg));
    }
    self.text.push(mn!("push", "rbp"));
    self.text.push(mn!(".seh_pushreg", "rbp"));
    let size = format!("{:#x}", scope.calc_alloc((scope.reg_used.len() % 2).saturating_mul(8))?);
    self.text.push(format!(include_str!("../asm/common/prologue.s"), size = size));
    for body in scope.body {
      self.text.push(body);
    }
    if let Json::Int(int) = &*ret {
      let int_str = match int {
        Lit(l_int) => l_int.to_string(),
        Var(int_label) => format!("{int_label}"),
      };
      self.text.push(mn!("mov", "rax", &int_str));
    } else {
      self.text.push(mn!("xor", "eax", "eax"));
    }
    self.text.push(mn!("mov", "rsp", "rbp"));
    self.text.push(mn!("pop", "rbp"));
    registers.reverse();
    for reg in registers {
      self.text.push(mn!("pop", reg));
    }
    self.text.push(mn!("ret"));
    self.text.push(mn!(".seh_endproc"));
    self.vars_local = take(&mut tmp_local_scope);
    Ok(Json::Function(AsmFunc { label, params, ret }))
  }
  #[expect(clippy::unnecessary_wraps, reason = "")]
  #[expect(clippy::unused_self, reason = "")]
  fn list(&mut self, func: FuncInfo, _: &mut ScopeInfo) -> ErrOR<Json> {
    Ok(Json::Array(Lit(Vec::from(func.args))))
  }
}
