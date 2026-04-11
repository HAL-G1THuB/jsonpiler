use crate::prelude::*;
built_in! {self, func, scope, compare;
  eq => {"==", COMMON, AtLeast(2), { self.compare_op(E, E, func, scope) }},
  grater => {">", COMMON, AtLeast(2), { self.compare_op(G, A, func, scope) }},
  grater_eq => {">=", COMMON, AtLeast(2), { self.compare_op(Ge, Ae, func, scope) }},
  less => {"<", COMMON, AtLeast(2), { self.compare_op(L, B, func, scope) }},
  less_eq => {"<=", COMMON, AtLeast(2), { self.compare_op(Le, Be, func, scope) }},
  not_eq => {"!=", COMMON, AtLeast(2), { self.compare_op(Ne, Ne, func, scope) }},
}
impl Jsonpiler {
  fn compare_op(
    &mut self,
    cc: ConditionCode,
    f_cc: ConditionCode,
    func: &mut BuiltIn,
    scope: &mut Scope,
  ) -> ErrOR<Json> {
    scope.push(mov_b(Rdx, 1));
    match func.arg()? {
      WithPos { val: Int(int), .. } => {
        scope.extend(&mov_int(Rax, int));
        for nth in 1..func.len {
          let (old, new) = if nth & 1 == 1 { (Rax, Rcx) } else { (Rcx, Rax) };
          scope.extend(&mov_int(new, arg!(self, func, (Int(x)) => x).val));
          scope.extend(&[LogicRR(Cmp, old, new), SetCc(old, cc), LogicRbRb(And, Rdx, old)]);
        }
        scope.push(UnaryRb(Neg, Rdx));
        scope.ret_bool(Rdx)
      }
      WithPos { val: Float(float), .. } => {
        scope.extend(&self.mov_float_xmm(Rax, Rax, float));
        for nth in 1..func.len {
          let (old, new) = if nth & 1 == 1 { (Rax, Rcx) } else { (Rcx, Rax) };
          scope.extend(&self.mov_float_xmm(new, Rax, arg!(self, func, (Float(x)) => x).val));
          scope.extend(&[UComISd(old, new), SetCc(Rax, f_cc), LogicRbRb(And, Rdx, Rax)]);
        }
        scope.push(UnaryRb(Neg, Rdx));
        scope.ret_bool(Rdx)
      }
      WithPos { val: Str(string), .. } if matches!(func.name.as_ref(), "==" | "!=") => {
        func.validate_args(Exact(2))?;
        let str_eq = self.str_eq(scope.id)?;
        scope.extend(&[
          self.mov_str(Rcx, string),
          self.mov_str(Rdx, arg!(self, func, (Str(x)) => x).val),
          Call(str_eq),
        ]);
        if func.name == "!=" {
          scope.push(UnaryRb(Not, Rax));
        }
        scope.ret_bool(Rax)
      }
      other => Err(args_type_err(1, &func.name, vec![IntT, BoolT], other.map_ref(Json::as_type))),
    }
  }
}
