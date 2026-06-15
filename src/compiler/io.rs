use crate::prelude::*;
built_in! {self, _func, scope, io;
  {"confirm", COMMON, Exact(2),
    confirm => {
      scope.ee_x(vec![
        self.load_str_x(Rcx, &arg!(_func, (Str(x)) => x).val)?,
        self.load_str_x(Rdx, &arg!(_func, (Str(x)) => x).val)?,
        vec![
          MovMId(Reg(R8), 4),
          Call(self.get_msg_box_x(scope.id)?),
          Clear(Rcx),
          MovMIb(Reg(Rdx), bool2byte(true)),
          m4i(Cmp, Rax, 6),
          CMovCc(E, Rcx, Rdx),
        ]
      ])?;
      Ok(Bool(Var(scope.ret_x(S1, Rcx)?)))
    },
    confirm_a => {
      scope.ee_a(vec![
        self.load_str_a(X0, &arg!(_func, (Str(x)) => x).val)?,
        self.load_str_a(X1, &arg!(_func, (Str(x)) => x).val)?,
        load_imm_a(X2, i64::from(bool2byte(true))),
        vec![Bl(self.get_msg_box_a(scope.id)?)],
        // x0 == 1000
        load_imm_a(X1, 1000),
        vec![CmpRR(X0, X1), CSet(X0, E.into()), SubR3(X0, Xzr, X0)],
      ])?;
      Ok(Bool(Var(scope.ret_a(S1, X1, X0)?)))
    }
  },
  {"input", COMMON, Exact(0),
    input_x => {
      scope.p_x(Call(self.get_input_x(scope.id)?))?;
      scope.ret_str_x(Rax, HeapPtr)
    },
    input_a => {
      scope.p_a(Bl(self.get_input_a(scope.id)?))?;
      scope.ret_str_a(X1, X0, HeapPtr)
    }
  },
  {"message", COMMON, Exact(2),
    message => {
      scope.ee_x(vec![
        self.load_str_x(Rcx, &arg!(_func, (Str(x)) => x).val)?,
        self.load_str_x(Rdx, &arg!(_func, (Str(x)) => x).val)?,
        vec![
          Clear(R8),
          Call(self.get_msg_box_x(scope.id)?)
        ]
      ])?;
      Ok(Null(Lit(())))
    },
    message_a => {
      scope.ee_a(vec![
        self.load_str_a(X0, &arg!(_func, (Str(x)) => x).val)?,
        self.load_str_a(X1, &arg!(_func, (Str(x)) => x).val)?,
        load_imm_a(X2, i64::from(bool2byte(false))),
        vec![Bl(self.get_msg_box_a(scope.id)?)]
      ])?;
      Ok(Null(Lit(())))
    }
  },
  {"print", SPECIAL, AtLeast(1),
    print => { self.print_args(_func, scope) },
    print_a => { self.print_args(_func, scope) }
  }
}
impl Jsonpiler {
  pub(crate) fn print_args(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    for _ in 1..=func.val.len {
      let raw = func.arg()?;
      let printable = self.eval(raw, scope)?;
      let Str(arg) = printable.val else {
        return Err(func.args_err(vec![StrT], &printable));
      };
      if self.flags.a64 {
        scope.ee_a(vec![self.load_str_a(X0, &arg)?, vec![Bl(self.get_print_a(scope.id)?)]])?;
      } else {
        scope.ee_x(vec![self.load_str_x(Rcx, &arg)?, vec![Call(self.get_print_x(scope.id)?)]])?;
      }
      self.drop_json(&Str(arg), false, scope)?;
    }
    Ok(Null(Lit(())))
  }
}
