use crate::prelude::*;
built_in! {self, func, scope, string;
  {"Str", COMMON, Exact(1),
    int_to_str_x => {
      scope.e_x(load_int_x(Rcx, arg!(func, (Int(x)) => x).val)?)?;
      scope.p_x(Call(self.get_int_to_str_x(scope.id)?))?;
      scope.ret_str_x(Rax, HeapPtr)
    },
    int_to_str_a => {
      scope.e_a(load_int_a(X0, arg!(func, (Int(x)) => x).val)?)?;
      scope.p_a(Bl(self.get_int_to_str_a(scope.id)?))?;
      scope.ret_str_a(X1, X0, HeapPtr)
    }
  },
  {"len", COMMON, Exact(1),
    len_x => {
      scope.ee_x(vec![
        self.load_str_x(Rcx, &arg!(func, (Str(x)) => x).val)?,
        vec![Call(self.str_chars_len_x(scope.id)?)]
      ])?;
      Ok(Int(Var(scope.ret_x(S8, Rax)?)))
    },
    len_a => {
      scope.ee_a(vec![
        self.load_str_a(X0, &arg!(func, (Str(x)) => x).val)?,
        vec![Bl(self.str_chars_len_a(scope.id)?)]
      ])?;
      Ok(Int(Var(scope.ret_a(S8, X1, X0)?)))
    }
  },
  {"slice", COMMON, Range(2, 3),
  slice => {
    let str_chars_len = self.str_chars_len_x(scope.id)?;
    let utf8_slice = self.get_utf8_slice_x(scope.id)?;
    let string = arg!(func, (Str(x)) => x).val;
    let start_idx = arg!(func, (Int(x)) => x).val;
    if func.val.len == 3 {
      scope.e_x(load_int_x(R8, arg!(func, (Int(x)) => x).val)?)?;
    } else {
      scope.ee_x(vec![self.load_str_x(Rcx, &string)?, vec![Call(str_chars_len), mov(S8, R8, Rax)]])?;
    }
    scope.ee_x(vec![load_int_x(Rdx, start_idx)?, self.load_str_x(Rcx, &string)?, vec![Call(utf8_slice)]])?;
    scope.ret_str_x(Rax, HeapPtr)
  },
  slice_a => {
    let str_chars_len = self.str_chars_len_a(scope.id)?;
    let utf8_slice = self.get_utf8_slice_a(scope.id)?;
    let string = arg!(func, (Str(x)) => x).val;
    let start_idx = arg!(func, (Int(x)) => x).val;
    if func.val.len == 3 {
      scope.e_a(load_int_a(X2, arg!(func, (Int(x)) => x).val)?)?;
    } else {
      scope.ee_a(vec![self.load_str_a(X0, &string)?, vec![Bl(str_chars_len), MovRR(X2, X0)]])?;
    }
    scope.ee_a(vec![load_int_a(X1, start_idx)?, self.load_str_a(X0, &string)?, vec![Bl(utf8_slice)]])?;
    scope.ret_str_a(X1, X0, HeapPtr)
  }},
}
impl Jsonpiler {
  pub(crate) fn cat_str(
    &mut self,
    first: Bind<String>,
    func: &mut Pos<BuiltIn>,
    scope: &mut Scope,
  ) -> ErrOR<Json> {
    let mut strings = vec![first];
    for _ in 1..func.val.len {
      let arg = arg!(func, (Str(x)) => x);
      match &arg.val {
        Lit(string) => {
          if let Some(Lit(str)) = strings.last_mut() {
            str.push_str(string);
          } else {
            strings.push(arg.val.clone());
          }
        }
        Var(_) => strings.push(arg.val),
      }
    }
    if strings.len() == 1 && matches!(&strings[0], Lit(_)) {
      return Ok(Str(take(&mut strings[0])));
    }
    let buffer = scope.alloc_str(HeapPtr)?;
    let acc_len = scope.tmp(8, 8, func)?.v_rq();
    if self.flags.a64 {
      let mut string_vec = vec![];
      let str_len = self.str_len_a(scope.id)?;
      scope.ee_a(vec![vec![MovRR(X0, Xzr)], store_a(acc_len, X1, X0)?])?;
      for string in strings {
        let len = scope.tmp(8, 8, func)?.v_rq();
        string_vec.push(string.clone());
        scope.ee_a(vec![
          self.load_str_a(X0, &string)?,
          vec![Bl(str_len)],
          store_a(len, X1, X0)?,
          load_a(X1, acc_len)?,
          vec![AddR3(X0, X0, X1)],
          store_a(acc_len, X1, X0)?,
        ])?;
      }
      scope.ee_a(vec![
        load_a(X0, acc_len)?,
        vec![AddRI12(X0, X0, 1)],
        self.call_alloc_x0_a()?,
        store_a(buffer, X1, X0)?,
        vec![MovRR(X3, X0)],
      ])?;
      for st in string_vec {
        let start = self.id();
        let end = self.id();
        scope.ee_a(vec![
          self.load_str_a(X2, &st)?,
          vec![
            MovZ(X5, 0, Lsl0),
            LblA(start),
            LdR(S1, X4, X2, 0),
            CmpRR(X4, X5),
            BCc(E.into(), end),
            StR(S1, X4, X3, 0),
            AddRI12(X2, X2, 1),
            AddRI12(X3, X3, 1),
            B_(start),
            LblA(end),
          ],
        ])?;
      }
    } else {
      let mut string_vec = vec![];
      let str_len = self.str_len_x(scope.id)?;
      let tmp_s = scope.tmp(8, 8, func)?;
      let tmp_d = scope.tmp(8, 8, func)?;
      scope.e_x(vec![
        store(S8, tmp_s, Rsi),
        store(S8, tmp_d, Rdi),
        Clear(Rax),
        store(S8, acc_len.0, Rax),
      ])?;
      for string in strings {
        let len = scope.tmp(8, 8, func)?;
        string_vec.push((string.clone(), len.v_rq()));
        scope.ee_x(vec![
          self.load_str_x(Rcx, &string)?,
          vec![
            Call(str_len),
            store(S8, len, Rax),
            load(S8, Rcx, acc_len.0),
            RR(S8, Add, Rax, Rcx),
            store(S8, acc_len.0, Rax),
          ],
        ])?;
      }
      scope.ee_x(vec![
        vec![load(S8, R8, acc_len.0), IncR(R8)],
        self.call_alloc_r8_x()?,
        vec![store(S8, buffer.0, Rax), store(S8, Rdi, Rax)],
      ])?;
      for (st, len) in string_vec {
        scope.ee_x(vec![
          vec![load(S8, Rcx, len.0)],
          self.load_str_x(Rsi, &st)?,
          vec![DF(false), Repeat(S1, RepE, MovS)],
        ])?;
      }
      scope.e_x(vec![MovMIb(Ref(Rdi), 0), load(S8, Rsi, tmp_s), load(S8, Rdi, tmp_d)])?;
    }
    Ok(Str(Var(buffer)))
  }
}
