use crate::{
  Arity::{self, *},
  Bind::{self, Lit, Var},
  CompilationErrKind::*,
  ConditionCode::E,
  DataInst::*,
  ErrOR, FuncInfo,
  Inst::{self, *},
  InternalErrKind::*,
  Json, Jsonpiler,
  JsonpilerErr::{self, *},
  LogicByteOpcode::*,
  Memory::*,
  Operand, Parser,
  Register::{self, *},
  ScopeInfo, WithPos, take_arg,
};
use std::{
  collections::HashMap,
  time::{SystemTime, UNIX_EPOCH},
};
impl Jsonpiler {
  pub(crate) const COMMON: (bool, bool) = (false, false);
  pub(crate) const CQO: &'static [u8] = &[0x48, 0x99];
  pub(crate) const GUI_H: u32 = 0x200;
  pub(crate) const GUI_W: u32 = 0x200;
  pub(crate) const REGS: [Register; 4] = [Rcx, Rdx, R8, R9];
  pub(crate) const RET: &'static [u8] = &[0xC3];
  pub(crate) const SCOPE: (bool, bool) = (true, false);
  pub(crate) const SPECIAL: (bool, bool) = (false, true);
  pub(crate) const SP_SCOPE: (bool, bool) = (true, true);
  pub(crate) fn call_api_check_null(&self, api: (u32, u32)) -> [Inst; 3] {
    [CallApi(api), LogicRR(Test, Rax, Rax), JCc(E, self.sym_table["WIN_HANDLER"])]
  }
  // Overflow is unlikely.
  pub(crate) fn gen_id(&mut self) -> u32 {
    let id = self.label_id;
    self.label_id += 1;
    id
  }
  pub(crate) fn get_bss_id(&mut self, size: u32, align: u32) -> u32 {
    let id = self.gen_id();
    self.data_insts.push(Bss(id, size, align));
    id
  }
  pub(crate) fn get_var(&self, var_name: &str, scope: &ScopeInfo) -> Option<Json> {
    scope.get_var_local(var_name).or_else(|| Some(self.globals.get(var_name)?.clone()))
  }
  pub(crate) fn global_bool(&mut self, boolean: bool) -> u32 {
    let id = self.gen_id();
    self.data_insts.push(Byte(id, if boolean { 0xFF } else { 0 }));
    id
  }
  pub(crate) fn global_num(&mut self, value: u64) -> u32 {
    let id = self.gen_id();
    self.data_insts.push(Quad(id, value));
    id
  }
  pub(crate) fn global_str(&mut self, value: String) -> (u32, usize) {
    if let Some(&id) = self.str_cache.get(&value) {
      return id;
    }
    let len = value.len();
    let id = self.gen_id();
    self.str_cache.insert(value.clone(), (id, len));
    self.data_insts.push(Bytes(id, value.into()));
    (id, len)
  }
  pub(crate) fn import(&mut self, key1: &'static str, key2: &'static str) -> ErrOR<(u32, u32)> {
    let outer_idx =
      if let Some(idx) = self.import_table.iter().position(|(outer_key, _)| *outer_key == key1) {
        idx
      } else {
        self.import_table.push((key1, vec![]));
        self.import_table.len() - 1
      };
    let inner_idx =
      if let Some(idx) = self.import_table[outer_idx].1.iter().position(|func| *func == key2) {
        idx
      } else {
        self.import_table[outer_idx].1.push(key2);
        self.import_table[outer_idx].1.len() - 1
      };
    Ok((
      u32::try_from(outer_idx).or(Err(InternalError(Overflow)))?,
      u32::try_from(inner_idx).or(Err(InternalError(Overflow)))?,
    ))
  }
  pub(crate) fn mov_str(&mut self, string: Bind<String>, dst: Register, scope: &mut ScopeInfo) {
    match string {
      Lit(l_str) => {
        let lbl = Global { id: self.global_str(l_str).0, disp: 0i32 };
        scope.push(LeaRM(dst, lbl));
      }
      Var(label) => scope.push(mov_q(dst, label.mem)),
    }
  }
  pub(crate) fn mov_str_len_c_a_d(
    &mut self, string: Bind<String>, str_reg: Register, len_reg: Register, scope: &mut ScopeInfo,
  ) -> ErrOR<()> {
    mov_len_c_a_d(&string, len_reg, scope)?;
    self.mov_str(string, str_reg, scope);
    Ok(())
  }
  // fn format(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {}
  #[inline]
  #[must_use]
  pub fn setup(source: Vec<u8>, name: String) -> Self {
    Self {
      builtin: HashMap::new(),
      data_insts: vec![],
      startup: vec![],
      str_cache: HashMap::new(),
      label_id: 0,
      globals: HashMap::new(),
      parser: vec![Parser::from(source, 0, name)],
      files: vec![HashMap::new()],
      insts: vec![],
      sym_table: HashMap::new(),
      import_table: vec![],
      user_defined: HashMap::new(),
    }
  }
  pub(crate) fn take_str(
    &mut self, reg: Register, func: &mut FuncInfo, scope: &mut ScopeInfo,
  ) -> ErrOR<()> {
    let string = take_arg!(self, func, "String", Json::String(x) => x).value;
    self.mov_str(string, reg, scope);
    Ok(())
  }
  pub(crate) fn take_str_len_c_a_d(
    &mut self, str_reg: Register, len_reg: Register, func: &mut FuncInfo, scope: &mut ScopeInfo,
  ) -> ErrOR<()> {
    let string = take_arg!(self, func, "String", Json::String(x) => x).value;
    self.mov_str_len_c_a_d(string, str_reg, len_reg, scope)
  }
}
pub(crate) fn align_up(num: usize, align: usize) -> ErrOR<usize> {
  num.div_ceil(align).checked_mul(align).ok_or(InternalError(Overflow))
}
pub(crate) fn align_up_32(value: u32, align: u32) -> ErrOR<u32> {
  value.div_ceil(align).checked_mul(align).ok_or(InternalError(Overflow))
}
pub(crate) fn align_up_i32(value: i32, align: i32) -> ErrOR<i32> {
  (value + align - 1)
    .checked_div(align)
    .and_then(|x| x.checked_mul(align))
    .ok_or(InternalError(Overflow))
}
#[expect(clippy::cast_possible_truncation)]
pub(crate) fn get_time_stamp() -> u32 {
  SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as u32
}
pub(crate) fn mov_bool(boolean: &Bind<bool>, dst: Register, scope: &mut ScopeInfo) {
  scope.push(match boolean {
    Bind::Lit(l_bool) => mov_b(dst, if *l_bool { 0xFF } else { 0 }),
    Bind::Var(label) => mov_b(dst, label.mem),
  });
}
pub(crate) fn mov_float_reg(float: &Bind<f64>, dst: Register, scope: &mut ScopeInfo) {
  scope.push(match float {
    Bind::Lit(l_float) => mov_q(dst, l_float.to_bits()),
    Bind::Var(label) => mov_q(dst, label.mem),
  });
}
pub(crate) fn mov_int(int: &Bind<i64>, dst: Register, scope: &mut ScopeInfo) {
  #[expect(clippy::cast_sign_loss)]
  scope.push(match int {
    Bind::Lit(l_int) => {
      if *l_int == 0 {
        Clear(dst)
      } else if let Ok(l_i32) = i32::try_from(*l_int)
        && l_int.is_positive()
      {
        mov_d(dst, l_i32 as u32)
      } else {
        mov_q(dst, *l_int as u64)
      }
    }
    Bind::Var(label) => mov_q(dst, label.mem),
  });
}
pub(crate) fn mov_q<T: Into<Operand<u64>>, U: Into<Operand<u64>>>(dst: T, src: U) -> Inst {
  MovQQ((dst.into(), src.into()).into())
}
pub(crate) fn mov_d<T: Into<Operand<u32>>, U: Into<Operand<u32>>>(dst: T, src: U) -> Inst {
  MovDD((dst.into(), src.into()).into())
}
pub(crate) fn mov_b<T: Into<Operand<u8>>, U: Into<Operand<u8>>>(dst: T, src: U) -> Inst {
  MovBB((dst.into(), src.into()).into())
}
pub(crate) fn mov_float_xmm(
  float: &Bind<f64>, xmm: Register, reg: Register, scope: &mut ScopeInfo,
) -> ErrOR<()> {
  let addr = match float {
    Bind::Lit(l_float) => {
      scope.push(mov_q(reg, l_float.to_bits()));
      let tmp = scope.alloc(8, 8)?;
      scope.push(mov_q(Tmp { offset: tmp }, reg));
      scope.free(tmp, 8)?;
      Tmp { offset: tmp }
    }
    Bind::Var(label) => label.mem,
  };
  scope.push(MovSdXM(xmm, addr));
  Ok(())
}
#[expect(clippy::cast_possible_wrap)]
pub(crate) fn mov_len_c_a_d(
  string: &Bind<String>, dst: Register, scope: &mut ScopeInfo,
) -> ErrOR<()> {
  match string {
    Lit(l_str) => mov_int(&Lit(l_str.len() as i64), dst, scope),
    Var(label) => {
      const CLD_REPNE_SCASB: &[u8] = &[0xFC, 0xF2, 0xAE];
      let tmp = scope.alloc(8, 8)?;
      scope.push(mov_q(dst, label.mem));
      scope.extend(&[
        mov_q(Tmp { offset: tmp }, Rdi),
        mov_q(Rdx, label.mem),
        mov_q(Rdi, Rdx),
        Clear(Rcx),
        DecR(Rcx),
        Clear(Rax),
        Custom(&CLD_REPNE_SCASB),
        SubRR(Rdi, Rdx),
        DecR(Rdi),
        mov_q(dst, Rdi),
        mov_q(Rdi, Tmp { offset: tmp }),
      ]);
    }
  }
  Ok(())
}
pub(crate) fn args_type_error(
  nth: usize, name: &str, expected: &str, json: &WithPos<Json>,
) -> JsonpilerErr {
  let suffix = match nth % 100 {
    11..=13 => "th",
    _ => match nth % 10 {
      1 => "st",
      2 => "nd",
      3 => "rd",
      _ => "th",
    },
  };
  let typ = json.value.type_name();
  CompilationError {
    kind: TypeError {
      name: format!("{nth}{suffix} argument of `{name}`"),
      expected: expected.to_owned(),
      typ,
    },
    pos: json.pos,
  }
}
pub(crate) fn take_bool(reg: Register, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<()> {
  let boolean = take_arg!(self, func, "Bool", Json::Bool(x) => x).value;
  mov_bool(&boolean, reg, scope);
  Ok(())
}
pub(crate) fn take_float(
  xmm: Register, reg: Register, func: &mut FuncInfo, scope: &mut ScopeInfo,
) -> ErrOR<()> {
  let float = take_arg!(self, func, "Float", Json::Float(x) => x).value;
  mov_float_xmm(&float, xmm, reg, scope)?;
  Ok(())
}
pub(crate) fn take_int(dst: Register, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<()> {
  let int = take_arg!(self, func, "Int", Json::Int(x) => x).value;
  mov_int(&int, dst, scope);
  Ok(())
}
pub(crate) fn take_len_c_a_d(
  dst: Register, func: &mut FuncInfo, scope: &mut ScopeInfo,
) -> ErrOR<()> {
  let string = take_arg!(self, func, "String", Json::String(x) => x).value;
  mov_len_c_a_d(&string, dst, scope)?;
  Ok(())
}
pub(crate) fn type_error(name: &str, expected: &str, json: &WithPos<Json>) -> JsonpilerErr {
  let typ = json.value.type_name();
  CompilationError {
    pos: json.pos,
    kind: TypeError { name: name.to_owned(), expected: expected.to_owned(), typ },
  }
}
pub(crate) fn validate_args(func: &FuncInfo, expected: Arity) -> ErrOR<()> {
  let supplied = func.len;
  match expected {
    Exactly(n) => {
      if supplied == n {
        return Ok(());
      }
    }
    AtLeast(min) => {
      if supplied >= min {
        return Ok(());
      }
    }
    AtMost(max) => {
      if supplied <= max {
        return Ok(());
      }
    }
    Range(min, max) => {
      if supplied >= min && supplied <= max {
        return Ok(());
      }
    }
    NoArgs => {
      if supplied == 0 {
        return Ok(());
      }
    }
    Any => (),
  }
  Err(CompilationError {
    kind: ArityError { name: func.name.clone(), expected, supplied },
    pos: func.pos,
  })
}
