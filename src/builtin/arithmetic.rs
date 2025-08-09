use crate::{
  Arity::{AtLeast, Exactly},
  Bind::{Lit, Var},
  ErrOR, FuncInfo, Json, Jsonpiler, ScopeInfo, built_in, err, include_once, mn, take_arg,
  write_once,
};
use std::io::Write as _;
built_in! {self, func, scope, arithmetic;
  abs => {"abs", COMMON, Exactly(1), {
  scope.body.push(mn!("mov", "rax", self.get_int_str(func, 1)?));
  scope.body.push(mn!("cqo"));
  scope.body.push(mn!("xor", "rax", "rdx"));
  scope.body.push(mn!("sub", "rax", "rdx"));
  Ok(Json::Int(Var(scope.mov_tmp("rax")?)))}},
  div => {"/", COMMON, AtLeast(2), {
    scope.body.push(mn!("mov", "rax", self.get_int_str(func, 1)?));
    for nth in 2..=func.len {
      let int_str = self.get_nonzero_int_str(scope, func, nth)?;
      scope.body.push(mn!("cqo"));
      scope.body.push(mn!("idiv", int_str));
    }
    Ok(Json::Int(Var(scope.mov_tmp("rax")?)))
  }},
  minus => {"-", COMMON, AtLeast(1), {
    if func.len != 1 {
      return self.arithmetic_template(func, scope, "sub");
    }
    scope.body.push(mn!("mov", "rax", self.get_int_str(func, 1)?));
    scope.body.push(mn!("neg", "rax"));
    Ok(Json::Int(Var(scope.mov_tmp("rax")?)))
  }},
  mul => {"*", COMMON, AtLeast(2), {
    self.arithmetic_template(func, scope, "imul")
  }},
  plus => {"+", COMMON, AtLeast(2), {
    self.arithmetic_template(func, scope, "add")
  }},
  rem => {"%", COMMON, Exactly(2), {
    scope.body.push(mn!("mov", "rax", self.get_int_str(func, 1)?));
    let int_str2 = self.get_nonzero_int_str(scope, func, 2)?;
    scope.body.push(mn!("cqo"));
    scope.body.push(mn!("idiv", int_str2));
    Ok(Json::Int(Var(scope.mov_tmp("rdx")?)))
  }}
}
impl Jsonpiler {
  fn arithmetic_int_str(
    &self, scope: &mut ScopeInfo, func: &mut FuncInfo, nth: usize,
  ) -> ErrOR<String> {
    let int = take_arg!(self, func, nth, "Int", Json::Int(x) => x).0;
    Ok(match int {
      Lit(l_int) => {
        if i64::from(i32::MIN) < l_int || l_int < i64::from(i32::MAX) {
          scope.body.push(mn!("mov", "rcx", l_int));
          "rcx".to_owned()
        } else {
          l_int.to_string()
        }
      }
      Var(label) => label.sched_free_2str(func),
    })
  }
  fn arithmetic_template(
    &mut self, func: &mut FuncInfo, scope: &mut ScopeInfo, mn: &str,
  ) -> ErrOR<Json> {
    let mut int_str = self.arithmetic_int_str(scope, func, 1)?;
    scope.body.push(mn!("mov", "rax", int_str));
    for nth in 2..=func.len {
      int_str = self.arithmetic_int_str(scope, func, nth)?;
      scope.body.push(mn!(mn, "rax", int_str));
    }
    Ok(Json::Int(Var(scope.mov_tmp("rax")?)))
  }
  fn get_nonzero_int_str(
    &mut self, scope: &mut ScopeInfo, func: &mut FuncInfo, nth: usize,
  ) -> ErrOR<String> {
    let (int, pos) = take_arg!(self, func, nth, "Int", Json::Int(x) => x);
    match int {
      Lit(l_int) => {
        if l_int == 0 {
          return err!(self, pos, "ZeroDivisionError");
        }
        scope.body.push(mn!("mov", "rcx", l_int));
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
}
