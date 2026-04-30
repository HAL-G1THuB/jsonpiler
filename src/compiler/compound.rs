use crate::prelude::*;
built_in! {self, func, scope, compound;
  assign_add => {"+=", SPECIAL, Exact(2), {
    self.assign_normal(None, AddRR(Rax, Rcx), Add, func, scope)
  }},
  assign_div => {"/=", SPECIAL, Exact(2), {
    self.assign_normal(Some(&Jsonpiler::check_zero_cqo), IDivR(Rcx), Div, func, scope)
  }},
  assign_mul => {"*=", SPECIAL, Exact(2), {
    self.assign_normal(None, IMulRR(Rax, Rcx), Mul, func, scope)
  }},
  assign_sub => {"-=", SPECIAL, Exact(2), {
    self.assign_normal(None, SubRR(Rax, Rcx), Sub, func, scope)
  }}
}
type CheckFn = dyn Fn(&mut Jsonpiler, Position, LabelId) -> ErrOR<Vec<Inst>>;
impl Jsonpiler {
  fn assign_normal(
    &mut self,
    check_opt: Option<&CheckFn>,
    int_inst: Inst,
    float_inst: ArithSdKind,
    func: &mut Pos<BuiltIn>,
    scope: &mut Scope,
  ) -> ErrOR<Json> {
    let var = func.arg()?.into_ident("Variable name")?;
    let variable = &self.get_var(&var, scope)?.val;
    let Some(memory) = variable.val.memory() else {
      return err!(var.pos, UndefinedVar(var.val));
    };
    let value = self.eval(func.arg()?, scope)?;
    match &value {
      Pos { val: Int(int), .. } => {
        if variable.val.as_type() != IntT {
          return Err(type_err(
            format_variable(&var.val, variable.kind),
            vec![IntT],
            var.pos.with(variable.val.as_type()),
          ));
        }
        scope.extend(&mov_memory(Rax, memory));
        scope.extend(&mov_int(Rcx, *int));
        if let Some(check) = check_opt {
          scope.extend(&check(self, var.pos, scope.id)?);
        }
        scope.push(int_inst);
        if !self.release {
          scope.extend(&[
            LogicRR(Test, Rax, Rax),
            JCc(O, self.custom_err(RuntimeOverflow, None, var.pos, scope.id)?),
          ]);
        }
        scope.extend(&ret_memory(memory, Rcx, Rax)?);
      }
      Pos { val: Float(float), .. } => {
        if variable.val.as_type() != FloatT {
          return Err(type_err(
            format_variable(&var.val, variable.kind),
            vec![FloatT],
            var.pos.with(variable.val.as_type()),
          ));
        }
        scope.extend(&self.mov_float_xmm(Rax, Rax, Var(memory))?);
        scope.extend(&self.mov_float_xmm(Rcx, Rax, *float)?);
        scope.push(ArithSd(float_inst, Rax, Rcx));
        scope.extend(&ret_memory_xmm(memory, Rax, Rax)?);
      }
      Pos { val: Str(string), .. } if func.val.name == "+=" => {
        let Str(Var(dst_str @ Memory(_, MemoryType { heap: HeapPtr, .. }))) = variable.val else {
          return Err(type_err(
            format_variable(&var.val, variable.kind),
            vec![StrT],
            var.pos.with(variable.val.as_type()),
          ));
        };
        let heap_alloc = self.import(KERNEL32, "HeapAlloc");
        let str_len = self.str_len(scope.id)?;
        let tmp = scope.tmp(8, 8, func)?;
        let tmp2 = scope.tmp(8, 8, func)?;
        let heap = Global(self.symbols[HEAP]);
        let leak = Global(self.symbols[LEAK_CNT]);
        scope.extend(&[
          self.mov_str(Rcx, Var(dst_str)),
          Call(str_len),
          mov_q(tmp, Rax),
          self.mov_str(Rcx, string.clone()),
          Call(str_len),
          mov_q(tmp2, Rax),
          mov_q(Rcx, heap),
          mov_q(Rdx, 8),
          mov_q(R8, tmp),
          AddRR(R8, Rax),
          IncR(R8),
          CallApi(heap_alloc),
          IncMd(leak),
          mov_q(Rcx, tmp),
          self.mov_str(Rsi, Var(dst_str)),
          mov_q(Rdi, Rax),
          Custom(CLD_REP_MOVSB),
          mov_q(Rcx, tmp2),
          self.mov_str(Rsi, string.clone()),
          Custom(CLD_REP_MOVSB),
          // Dil not supported
          mov_q(Rcx, Rdi),
          mov_b(Rdx, 0),
          mov_b(Ref(Rcx), Rdx),
          mov_q(tmp, Rax),
        ]);
        self.heap_free(dst_str, scope);
        scope.extend(&[mov_q(Rax, tmp), mov_q(dst_str.0, Rax)]);
      }
      other => {
        return Err(func.args_err(
          if func.val.name == "+=" { vec![IntT, FloatT, StrT] } else { vec![IntT, FloatT] },
          other.map_ref(Json::as_type),
        ));
      }
    }
    self.drop_json(value.val, false, scope);
    Ok(Null(Lit(())))
  }
}
