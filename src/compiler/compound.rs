use super::arithmetic::{Arith, CHECK_A, CHECK_X};
use crate::prelude::*;
built_in! {self, func, scope, compound;
  {"+=", SPECIAL, Exact(2),
    assign_add_x => { self.assign_add(func, scope) },
    assign_add_a => { self.assign_add(func, scope) }
  },
  {"/=", SPECIAL, Exact(2),
    assign_div_x => { self.assign_div(func, scope) },
    assign_div_a => { self.assign_div(func, scope) }
  },
  {"*=", SPECIAL, Exact(2),
    assign_mul_x => { self.assign_mul(func, scope) },
    assign_mul_a => { self.assign_mul(func, scope) }
  },
  {"-=", SPECIAL, Exact(2),
    assign_sub_x => { self.assign_sub(func, scope) },
    assign_sub_a => { self.assign_sub(func, scope) }
  }
}
impl Jsonpiler {
  fn assign_add(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    self.assign_normal((Ok(Add), AddSd, &AddR3, None), func, scope)
  }
  fn assign_div(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    self.assign_normal((Err((IDivR(Rcx), CHECK_X)), Div, &SDivR3, CHECK_A), func, scope)
  }
  fn assign_mul(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    self.assign_normal((Err((IMulR2(Rax, Rcx), None)), Mul, &MulR3, None), func, scope)
  }
  fn assign_sub(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    self.assign_normal((Ok(Sub), SubSd, &SubR3, None), func, scope)
  }
}
impl Jsonpiler {
  fn assign_normal(
    &mut self,
    (int_x, float_kind, int_a, check_a): Arith,
    func: &mut Pos<BuiltIn>,
    scope: &mut Scope,
  ) -> ErrOR<Json> {
    let name = func.arg()?.into_ident("Variable name")?;
    let Pos { val, pos } = self.eval(func.arg()?, scope)?;
    let var = self.get_var(&name, scope)?.val;
    let Some(mem) = var.val.mem() else {
      return err!(name.pos, UndefinedVar(name.val));
    };
    match &val {
      Int(int) => {
        if var.val.as_type() != IntT {
          let actual = name.pos.with(var.val.as_type());
          return Err(type_err(fmt_var(&name.val, var.kind), vec![IntT], actual));
        }
        //        r_m(S8, Add, Rax, *int);
        self.mov_first_operand(Var(mem), scope)?;
        self.rest_op(*int, (int_x, float_kind, int_a, check_a), func, scope)?;
        if self.flags.a64 {
          scope.e_a(store_a(mem, X1, X0)?)?;
        } else {
          scope.e_x(store_x(mem, Rcx, Rax)?)?;
        }
      }
      Float(float) => {
        if var.val.as_type() != FloatT {
          let actual = name.pos.with(var.val.as_type());
          return Err(type_err(fmt_var(&name.val, var.kind), vec![FloatT], actual));
        }
        self.mov_first_float(Var(mem), scope)?;
        self.rest_float(*float, float_kind, scope)?;
        if self.flags.a64 {
          scope.e_a(store_dn(mem, X0, X0)?)?;
        } else {
          scope.e_x(store_xmm(mem, Rax, Rax)?)?;
        }
      }
      Str(string) if func.val.name == "+=" => {
        let Str(Var(dst_str @ Memory(_, MemType { pass_by: HeapPtr, size: Dynamic }))) = var.val
        else {
          let actual = name.pos.with(var.val.as_type());
          return Err(type_err(fmt_var(&name.val, var.kind), vec![StrT], actual));
        };
        let dst_len = scope.tmp(8, 8, func)?.v_rq();
        let src_len = scope.tmp(8, 8, func)?.v_rq();
        if self.flags.a64 {
          scope.ee_a(vec![
            self.call_str_len_a(scope.id, &Var(dst_str), dst_len)?,
            self.call_str_len_a(scope.id, string, src_len)?,
            load_a(X1, dst_len)?,
            vec![AddR3(X0, X0, X1), AddRI12(X0, X0, 1)],
            self.call_alloc_x0_a()?,
            vec![MovRR(X3, X0)],
            self.copy_loop_a(&Var(dst_str), dst_len)?,
            self.copy_loop_a(string, src_len)?,
            vec![MovRR(X1, X0), StR(S1, Xzr, X3, 0)],
            store_a(dst_len, X1, X0)?,
            self.heap_free_a(mem)?,
            load_a(X0, dst_len)?,
            store_a(dst_str, X1, X0)?,
          ])?;
        } else {
          scope.ee_x(vec![
            self.call_str_len_x(scope.id, &Var(dst_str), dst_len)?,
            self.call_str_len_x(scope.id, string, src_len)?,
            vec![mov(S8, R8, Rax), r_m(S8, Add, R8, dst_len.0), IncR(R8)],
            self.call_alloc_r8_x()?,
            vec![mov(S8, Rdi, Rax)],
            self.copy_loop_x(&Var(dst_str), dst_len)?,
            self.copy_loop_x(string, src_len)?,
            vec![MovMIb(Ref(Rdi), 0)],
            load_x(Rax, dst_len)?,
            self.heap_free_x(dst_str)?,
            load_x(Rax, dst_len)?,
            vec![store(S8, dst_str.0, Rax)],
          ])?;
        }
      }
      Array(..) | Bool(_) | Null(_) | Object(_) | Str(_) => {
        let mut types = vec![IntT, FloatT];
        if func.val.name == "+=" {
          types.push(StrT)
        };
        return Err(func.args_err(types, &pos.with(val)));
      }
    }
    self.drop_json(&val, false, scope)?;
    Ok(Null(Lit(())))
  }
  fn call_str_len_a(
    &mut self,
    caller: LabelId,
    string: &Bind<String>,
    len: Memory,
  ) -> ErrOR<Vec<A64Inst>> {
    let str_len = self.str_len_a(caller)?;
    Ok([self.load_str_a(X0, string)?, vec![Bl(str_len)], store_a(len, X1, X0)?].concat())
  }
  fn call_str_len_x(
    &mut self,
    caller: LabelId,
    string: &Bind<String>,
    len: Memory,
  ) -> ErrOR<Vec<X64Inst>> {
    let str_len = self.str_len_x(caller)?;
    Ok([self.load_str_x(Rcx, string)?, vec![Call(str_len)], store_x(len, Rcx, Rax)?].concat())
  }
  fn copy_loop_a(&mut self, string: &Bind<String>, len: Memory) -> ErrOR<Vec<A64Inst>> {
    let start = self.id();
    let end = self.id();
    Ok(
      [
        load_a(X1, len)?,
        self.load_str_a(X2, string)?,
        vec![
          LblA(start),
          CmpRR(X1, Xzr),
          BCc(E.into(), end),
          LdR(S1, X4, X2, 0),
          StR(S1, X4, X3, 0),
          SubRI12(X1, X1, 1),
          AddRI12(X2, X2, 1),
          AddRI12(X3, X3, 1),
          B_(start),
          LblA(end),
        ],
      ]
      .concat(),
    )
  }
  fn copy_loop_x(&mut self, string: &Bind<String>, len: Memory) -> ErrOR<Vec<X64Inst>> {
    Ok(
      [
        vec![load(S8, Rcx, len.0)],
        self.load_str_x(Rsi, string)?,
        vec![DF(false), Repeat(S1, RepE, MovS)],
      ]
      .concat(),
    )
  }
}
