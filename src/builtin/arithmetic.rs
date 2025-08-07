use super::super::{
  ArgLen::{Any, AtLeast, Exactly},
  Bind::{self, Lit, Var},
  ErrOR, FuncInfo, Json, Jsonpiler, Position, ScopeInfo, err, include_once, mn,write_once,
  utility::get_int_str,
  validate_type,
};
use std::io::Write as _;
impl Jsonpiler {
  pub(crate) fn arithmetic(&mut self) {
    let common = (false, false);
    self.register("abs", common, Jsonpiler::abs, Exactly(1));
    self.register("+", common, Jsonpiler::plus, Any);
    self.register("%", common, Jsonpiler::rem, Exactly(2));
    self.register("-", common, Jsonpiler::minus, Any);
    self.register("*", common, Jsonpiler::mul, Any);
    self.register("/", common, Jsonpiler::div, AtLeast(2));
  }
}
#[expect(clippy::single_call_fn, reason = "")]
impl Jsonpiler {
  fn abs(&mut self,  func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    let json = func.arg()?;
    let int = &validate_type!(self, func, 1, json, Json::Int(x) => x, "Int");
    let int_str = get_int_str(int, func);
    scope.body.push(mn!("mov", "rax", int_str));
    scope.body.push(mn!("cqo"));
    scope.body.push(mn!("xor", "rax", "rdx"));
    scope.body.push(mn!("sub", "rax", "rdx"));
    Ok(Json::Int(Var(scope.mov_tmp("rax")?)))
  }
  fn arithmetic_template(
    &mut self,  func: &mut FuncInfo, scope: &mut ScopeInfo, mn: &str, identity_element: usize,
  ) -> ErrOR<Json> {
    if let Some(mut arg) = func.args.pop_front() {
      let mut int = validate_type!(self, func, 1, arg, Json::Int(x) => x, "Int");
      let mut int_str = arithmetic_int_str(&int, scope, func);
      scope.body.push(mn!("mov", "rax", &int_str));
      for ord in 2..=func.len {
        arg = func.arg()?;
        int = validate_type!(self, func, ord, arg, Json::Int(x) => x, "Int");
        int_str = arithmetic_int_str(&int, scope, func);
        scope.body.push(mn!(mn, "rax", &int_str));
      }
      Ok(Json::Int(Var(scope.mov_tmp("rax")?)))
    } else {
      Ok(Json::Int(Var(scope.mov_tmp(&identity_element.to_string())?)))
    }
  }
  fn div(&mut self,  func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    let mut arg = func.arg()?;
    let mut int = validate_type!(self, func, 1, arg, Json::Int(x) => x, "Int");
    let mut int_str = get_int_str(&int, func);
    scope.body.push(mn!("mov", "rax", int_str));
    for ord in 2..=func.len {
      arg = func.arg()?;
      int = validate_type!(self, func, ord, arg, Json::Int(x) => x, "Int");
      int_str = self.get_nonzero_int_str(&int, &arg.pos, scope, func)?;
      scope.body.push(mn!("cqo"));
      scope.body.push(mn!("idiv", int_str));
    }
    Ok(Json::Int(Var(scope.mov_tmp("rax")?)))
  }
  fn get_nonzero_int_str(
    &mut self, bind: &Bind<i64>, pos: &Position, scope: &mut ScopeInfo,func: &mut FuncInfo
  ) -> ErrOR<String> {
    match bind {
      Lit(int) => {
        if *int == 0 {
          return err!(self, pos, "ZeroDivisionError");
        }
        scope.body.push(mn!("mov", "rcx", int));
        Ok("rcx".to_owned())
      }
      Var(label) => {
        let label_str = label.sched_free_2str(func);
        scope.body.push(mn!("cmp", label_str, "0"));
        write_once!(self, "err/ZERO_DIVISION_MSG");
        include_once!(self, self.text, "err/ZERO_DIVISION_ERR");
        scope.body.push(mn!("jz", ".L__ZERO_DIVISION_ERR"));
        Ok(label_str)
      }
    }
  }
  fn minus(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    if func.len != 1 {
      return self.arithmetic_template(func, scope, "sub", 0);
    }
    let arg = func.arg()?;
    let int = validate_type!(self, func, 1, arg, Json::Int(x) => x, "Int");
    let int_str = get_int_str(&int, func);
    scope.body.push(mn!("mov", "rax", int_str));
    scope.body.push(mn!("neg", "rax"));
    Ok(Json::Int(Var(scope.mov_tmp("rax")?)))
  }
  fn mul(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.arithmetic_template(func, scope, "imul", 1)
  }
  fn plus(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    self.arithmetic_template(func, scope, "add", 0)
  }
  fn rem(&mut self,  func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {
    let json1 = func.arg()?;
    let int1 = validate_type!(self, func, 1, json1, Json::Int(x) => x, "Int");
    let int_str1 = get_int_str(&int1, func);
    scope.body.push(mn!("mov", "rax", &int_str1));
    let json2 = func.arg()?;
    let int2 = validate_type!(self, func, 2, json2, Json::Int(x) => x, "Int");
    let int_str2 = self.get_nonzero_int_str(&int2, &json2.pos, scope, func)?;
    scope.body.push(mn!("cqo"));
    scope.body.push(mn!("idiv", int_str2));
    Ok(Json::Int(Var(scope.mov_tmp("rdx")?)))
  }
}
fn arithmetic_int_str(int: &Bind<i64>, scope: &mut ScopeInfo, func: &mut FuncInfo) -> String {
  match int {
    Lit(l_int) => {
      if i64::from(i32::MIN) < *l_int || *l_int < i64::from(i32::MAX) {
        scope.body.push(mn!("mov", "rcx", l_int));
        "rcx".to_owned()
      } else {
        l_int.to_string()
      }
    }
    Var(label) => label.sched_free_2str(func),
  }
}
