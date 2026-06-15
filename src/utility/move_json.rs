use crate::prelude::*;
impl Scope {
  pub(crate) fn alloc_n(&mut self, reg_size: RegSize) -> ErrOR<Memory> {
    let addr = Local(Tmp, self.alloc(reg_size as i32, reg_size as i32)?);
    let mem_type = MemType { pass_by: Value, size: Small(reg_size) };
    Ok(Memory(addr, mem_type))
  }
  pub(crate) fn alloc_str(&mut self, pass_by: PassBy) -> ErrOR<Memory> {
    let addr = Local(Tmp, self.alloc(8, 8)?);
    let mem_type = MemType { pass_by, size: Dynamic };
    Ok(Memory(addr, mem_type))
  }
  pub(crate) fn ret_a(&mut self, reg_size: RegSize, tmp: A64Reg, src: A64Reg) -> ErrOR<Memory> {
    let mem = self.alloc_n(reg_size)?;
    self.e_a(store_a(mem, tmp, src)?)?;
    Ok(mem)
  }
  pub(crate) fn ret_dn(&mut self, tmp: A64Reg, dn: A64Reg) -> ErrOR<Json> {
    let mem = self.alloc_n(S8)?;
    self.e_a(load_ref_a(tmp, mem)?)?;
    self.p_a(FStRD(dn, tmp, 0))?;
    Ok(Float(Var(mem)))
  }
  pub(crate) fn ret_json_take_a(
    &mut self,
    dst: &Pos<JsonType>,
    tmp: A64Reg,
    src: A64Reg,
  ) -> ErrOR<Json> {
    Ok(match dst.val {
      NullT => Null(Var(self.ret_a(S8, tmp, src)?)),
      IntT => Int(Var(self.ret_a(S8, tmp, src)?)),
      FloatT => Float(Var(self.ret_a(S8, tmp, src)?)),
      BoolT => Bool(Var(self.ret_a(S1, tmp, src)?)),
      StrT => self.ret_str_a(tmp, src, HeapPtr)?,
      CustomT(_) | FuncT(_) | ArrayT(_) | ObjectT => {
        return err!(dst.pos, UnsupportedType(dst.val.to_string()));
      }
    })
  }
  pub(crate) fn ret_json_take_x(&mut self, dst: &Pos<JsonType>, src: X64Reg) -> ErrOR<Json> {
    Ok(match dst.val {
      NullT => Null(Var(self.ret_x(S8, src)?)),
      IntT => Int(Var(self.ret_x(S8, src)?)),
      FloatT => Float(Var(self.ret_x(S8, src)?)),
      BoolT => Bool(Var(self.ret_x(S1, src)?)),
      StrT => self.ret_str_x(src, HeapPtr)?,
      CustomT(_) | FuncT(_) | ArrayT(_) | ObjectT => {
        return err!(dst.pos, UnsupportedType(dst.val.to_string()));
      }
    })
  }
  pub(crate) fn ret_str_a(&mut self, tmp: A64Reg, src: A64Reg, pass_by: PassBy) -> ErrOR<Json> {
    let mem = self.alloc_str(pass_by)?;
    self.e_a(store_a(mem, tmp, src)?)?;
    Ok(Str(Var(mem)))
  }
  pub(crate) fn ret_str_x(&mut self, src: X64Reg, pass_by: PassBy) -> ErrOR<Json> {
    let mem = self.alloc_str(pass_by)?;
    self.p_x(store(S8, mem.0, src))?;
    Ok(Str(Var(mem)))
  }
  pub(crate) fn ret_x(&mut self, reg_size: RegSize, src: X64Reg) -> ErrOR<Memory> {
    let mem = self.alloc_n(reg_size)?;
    self.p_x(store(reg_size, mem.0, src))?;
    Ok(mem)
  }
  pub(crate) fn ret_xmm(&mut self, xmm: X64Reg) -> ErrOR<Json> {
    let mem = self.alloc_n(S8)?;
    self.p_x(MovOSd(Mem(mem.0), xmm))?;
    Ok(Float(Var(mem)))
  }
}
impl Jsonpiler {
  pub(crate) fn load_args_a(
    &mut self,
    idx: u32,
    float_idx: &mut u32,
    mem: Memory,
    is_float: bool,
  ) -> ErrOR<Vec<A64Inst>> {
    let mut insts = vec![];
    let int_idx = idx - *float_idx;
    let target_idx = if is_float { *float_idx } else { int_idx };
    let is_reg = target_idx < len_u32(&A64_ARG_REGS)?;
    let reg = *A64_ARG_REGS.get(target_idx as usize).unwrap_or(&X9);
    if is_float && is_reg {
      insts.extend_from_slice(&store_dn(mem, X9, reg)?);
    } else {
      if !is_reg {
        let int_stack = int_idx.saturating_sub(len_u32(&A64_ARG_REGS)?);
        let float_stack = float_idx.saturating_sub(len_u32(&A64_ARG_REGS)?);
        insts.push(MovRR(X9, X29));
        insts.extend_from_slice(&load_imm_a(X10, 16 + (int_stack + float_stack) as i64 * 8));
        insts.extend_from_slice(&[AddR3(X9, X9, X10), LdR(S8, X9, X9, 0)]);
      }
      insts.extend_from_slice(&store_a(mem, X10, reg)?);
    }
    if is_float {
      *float_idx += 1;
    }
    Ok(insts)
  }
  pub(crate) fn load_args_x(
    &mut self,
    idx: u32,
    mem: Memory,
    is_float: bool,
  ) -> ErrOR<Vec<X64Inst>> {
    let mut insts = vec![];
    let reg = *X64_ARG_REGS.get(idx as usize).unwrap_or(&Rax);
    if reg != Rax && is_float {
      insts.extend_from_slice(&store_xmm(mem, R10, reg)?);
    } else {
      if reg == Rax {
        insts.push(load(S8, Rax, Local(Tmp, i32::try_from(idx * 8 + 16)?)));
      }
      insts.extend_from_slice(&store_x(mem, R10, reg)?);
    }
    Ok(insts)
  }
  pub(crate) fn load_dn(
    &mut self,
    dn: A64Reg,
    tmp: A64Reg,
    float: Bind<f64>,
  ) -> ErrOR<Vec<A64Inst>> {
    let mem = match float {
      Lit(lit) => self.global_q(lit.to_bits()),
      Var(mem) => mem,
    };
    let mut insts = load_ref_a(tmp, mem)?;
    insts.push(FLdRD(dn, tmp, 0));
    Ok(insts)
  }
  pub(crate) fn load_json_a(
    &mut self,
    dst: A64Reg,
    src: &Pos<Json>,
    copy: Option<LabelId>,
  ) -> ErrOR<Vec<A64Inst>> {
    match &src.val {
      Null(_) => Ok(vec![MovRR(dst, Xzr)]),
      Bool(boolean) => load_bool_a(dst, *boolean),
      Int(int) => load_int_a(dst, *int),
      Float(float) => load_float_reg_a(dst, *float),
      Str(string) => Ok(if let Some(caller) = copy {
        let mut insts = self.load_str_a(X0, string)?;
        insts.extend_from_slice(&[Bl(self.clone_str_a(caller)?), MovRR(dst, X0)]);
        insts
      } else {
        self.load_str_a(dst, string)?
      }),
      Array(_, _) | Object(_) => err!(src.pos, UnsupportedType(src.val.describe())),
    }
  }
  pub(crate) fn load_json_x(
    &mut self,
    dst: X64Reg,
    src: &Pos<Json>,
    copy: Option<LabelId>,
  ) -> ErrOR<Vec<X64Inst>> {
    match &src.val {
      Null(_) => Ok(vec![Clear(dst)]),
      Bool(boolean) => load_bool_x(dst, *boolean),
      Int(int) => load_int_x(dst, *int),
      Float(float) => load_float_reg_x(dst, *float),
      Str(string) => Ok(if let Some(caller) = copy {
        let mut insts = self.load_str_x(Rcx, string)?;
        insts.extend_from_slice(&[Call(self.clone_str_x(caller)?), mov(S8, dst, Rax)]);
        insts
      } else {
        self.load_str_x(dst, string)?
      }),
      Array(_, _) | Object(_) => err!(src.pos, UnsupportedType(src.val.describe())),
    }
  }
  pub(crate) fn load_str_a(&mut self, dst: A64Reg, string: &Bind<String>) -> ErrOR<Vec<A64Inst>> {
    load_a(
      dst,
      match string {
        Lit(lit) => self.global_str(lit),
        Var(mem) => *mem,
      },
    )
  }
  pub(crate) fn load_str_x(&mut self, dst: X64Reg, string: &Bind<String>) -> ErrOR<Vec<X64Inst>> {
    load_x(
      dst,
      match string {
        Lit(lit) => self.global_str(lit),
        Var(mem) => *mem,
      },
    )
  }
  pub(crate) fn load_xmm(
    &mut self,
    xmm: X64Reg,
    tmp: X64Reg,
    float: Bind<f64>,
  ) -> ErrOR<Vec<X64Inst>> {
    match float {
      Lit(lit) => Ok(vec![MovSdO(xmm, Mem(self.global_q(lit.to_bits()).0))]),
      Var(mem) => load_xmm(xmm, tmp, mem),
    }
  }
  fn store_arg_clobbering_a(
    &mut self,
    idx: usize,
    float_idx: &mut usize,
    arg: &Pos<Json>,
    copy: Option<LabelId>,
    scope: &mut Scope,
  ) -> ErrOR<()> {
    let is_float = arg.val.as_type() == FloatT;
    let int_idx = idx - *float_idx;
    let target_idx = if is_float { *float_idx } else { int_idx };
    let is_reg = target_idx < A64_ARG_REGS.len();
    let reg = *A64_ARG_REGS.get(target_idx).unwrap_or(&X9);
    scope.e_a(
      if is_float
        && is_reg
        && let Float(float) = arg.val
      {
        self.load_dn(reg, X9, float)?
      } else {
        self.load_json_a(reg, arg, copy)?
      },
    )?;
    if !is_reg {
      let int_stack = int_idx.saturating_sub(A64_ARG_REGS.len());
      let float_stack = float_idx.saturating_sub(A64_ARG_REGS.len());
      scope.ee_a(vec![
        vec![AddRI12(X10, SP, 0)],
        load_imm_a(X11, ((int_stack + float_stack) * 8) as i64),
        vec![AddR3(X10, X10, X11), StR(S8, X9, X10, 0)],
      ])?;
    }
    if is_float {
      *float_idx += 1;
    }
    Ok(())
  }
  fn store_arg_clobbering_x(
    &mut self,
    idx: usize,
    arg: &Pos<Json>,
    copy: Option<LabelId>,
    scope: &mut Scope,
  ) -> ErrOR<()> {
    let reg = *X64_ARG_REGS.get(idx).unwrap_or(&Rax);
    scope.e_x(
      if reg != Rax
        && let Float(float) = arg.val
      {
        self.load_xmm(reg, Rax, float)?
      } else {
        self.load_json_x(reg, arg, copy)?
      },
    )?;
    if reg == Rax {
      scope.p_x(store(S8, Args(i32::try_from(idx + 1)?), Rax))?;
    }
    Ok(())
  }
  pub(crate) fn store_args_a(
    &mut self,
    args: Vec<Pos<Json>>,
    copy: bool,
    scope: &mut Scope,
  ) -> ErrOR<()> {
    if copy {
      let mut float_counts = vec![0usize; args.len()];
      let mut n = 0;
      for (idx, arg) in args.iter().enumerate() {
        float_counts[idx] = n;
        if arg.val.as_type() == FloatT {
          n += 1;
        }
      }
      let is_str_int_reg = |idx: usize| -> ErrOR<bool> {
        let int_idx = idx.saturating_sub(float_counts[idx]);
        Ok(int_idx < A64_ARG_REGS.len())
      };
      for (idx, arg) in args.iter().enumerate() {
        if arg.val.as_type() == StrT && !is_str_int_reg(idx)? {
          let mut float_idx = float_counts[idx];
          self.store_arg_clobbering_a(idx, &mut float_idx, arg, Some(scope.id), scope)?;
        }
      }
      let mut str_regs = vec![];
      for (idx, arg) in args.iter().enumerate() {
        if arg.val.as_type() == StrT && is_str_int_reg(idx)? {
          let int_idx = idx.saturating_sub(float_counts[idx]);
          let reg = A64_ARG_REGS[int_idx];
          if str_regs.is_empty() {
            let mut float_idx = float_counts[idx];
            self.store_arg_clobbering_a(idx, &mut float_idx, arg, Some(scope.id), scope)?;
            str_regs.push(reg);
            continue;
          }
          let tmp_len = i32::try_from(str_regs.len() * 8)?;
          let tmp = scope.alloc(tmp_len, 8)?;
          for (tmp_idx, tmp_reg) in str_regs.iter().enumerate() {
            let tmp_mem = Local(Tmp, tmp + i32::try_from(tmp_idx * 8)?).v_rq();
            scope.e_a(store_a(tmp_mem, X10, *tmp_reg)?)?;
          }
          let mut float_idx = float_counts[idx];
          self.store_arg_clobbering_a(idx, &mut float_idx, arg, Some(scope.id), scope)?;
          for (tmp_idx, tmp_reg) in str_regs.iter().enumerate() {
            let tmp_mem = Local(Tmp, tmp + i32::try_from(tmp_idx * 8)?).v_rq();
            scope.e_a(load_a(*tmp_reg, tmp_mem)?)?;
          }
          scope.free(tmp, tmp_len)?;
          str_regs.push(reg);
        }
      }
      for (idx, arg) in args.iter().enumerate() {
        if arg.val.as_type() != StrT {
          let mut float_idx = float_counts[idx];
          self.store_arg_clobbering_a(idx, &mut float_idx, arg, Some(scope.id), scope)?;
        }
      }
    } else {
      let mut float_idx = 0;
      for (idx, arg) in args.iter().enumerate() {
        self.store_arg_clobbering_a(idx, &mut float_idx, arg, None, scope)?;
      }
    }
    Ok(())
  }
  pub(crate) fn store_args_x(
    &mut self,
    args: Vec<Pos<Json>>,
    copy: bool,
    scope: &mut Scope,
  ) -> ErrOR<()> {
    if copy {
      for (idx, arg) in args.iter().enumerate().skip(X64_ARG_REGS.len()) {
        if arg.val.as_type() == StrT {
          self.store_arg_clobbering_x(idx, arg, Some(scope.id), scope)?;
        }
      }
      let mut str_regs = vec![];
      for (idx, arg) in args.iter().enumerate().take(X64_ARG_REGS.len()) {
        if arg.val.as_type() == StrT {
          let reg = X64_ARG_REGS[idx];
          if str_regs.is_empty() {
            self.store_arg_clobbering_x(idx, arg, Some(scope.id), scope)?;
            str_regs.push(reg);
            continue;
          }
          let tmp_len = i32::try_from(str_regs.len() * 8)?;
          let tmp = scope.alloc(tmp_len, 8)?;
          for (tmp_idx, tmp_reg) in str_regs.iter().enumerate() {
            scope.p_x(store(S8, Local(Tmp, tmp + i32::try_from(tmp_idx * 8)?), *tmp_reg))?;
          }
          self.store_arg_clobbering_x(idx, arg, Some(scope.id), scope)?;
          for (tmp_idx, tmp_reg) in str_regs.iter().enumerate() {
            scope.p_x(load(S8, *tmp_reg, Local(Tmp, tmp + i32::try_from(tmp_idx * 8)?)))?;
          }
          scope.free(tmp, tmp_len)?;
          str_regs.push(reg);
        }
      }
      for (idx, arg) in args.iter().enumerate() {
        if arg.val.as_type() != StrT {
          self.store_arg_clobbering_x(idx, arg, Some(scope.id), scope)?;
        }
      }
    } else {
      for (idx, arg) in args.iter().enumerate() {
        self.store_arg_clobbering_x(idx, arg, None, scope)?;
      }
    }
    Ok(())
  }
}
pub(crate) fn load_bool_x(dst: X64Reg, boolean: Bind<bool>) -> ErrOR<Vec<X64Inst>> {
  match boolean {
    Lit(lit) => Ok(vec![MovMIb(Reg(dst), bool2byte(lit))]),
    Var(mem) => load_x(dst, mem),
  }
}
pub(crate) fn load_bool_a(dst: A64Reg, boolean: Bind<bool>) -> ErrOR<Vec<A64Inst>> {
  let int = match boolean {
    Lit(lit) => Lit(i64::from(bool2byte(lit))),
    Var(mem) => Var(mem),
  };
  load_int_a(dst, int)
}
pub(crate) fn load_float_reg_x(dst: X64Reg, float: Bind<f64>) -> ErrOR<Vec<X64Inst>> {
  let int = match float {
    Lit(lit) => Lit(lit.to_bits() as i64),
    Var(mem) => Var(mem),
  };
  load_int_x(dst, int)
}
pub(crate) fn load_float_reg_a(dst: A64Reg, float: Bind<f64>) -> ErrOR<Vec<A64Inst>> {
  let int = match float {
    Lit(lit) => Lit(lit.to_bits() as i64),
    Var(mem) => Var(mem),
  };
  load_int_a(dst, int)
}
pub(crate) fn load_int_x(dst: X64Reg, int: Bind<i64>) -> ErrOR<Vec<X64Inst>> {
  match int {
    Lit(lit) => Ok(vec![load_imm_x(dst, lit)]),
    Var(mem) => load_x(dst, mem),
  }
}
pub(crate) fn load_int_a(dst: A64Reg, int: Bind<i64>) -> ErrOR<Vec<A64Inst>> {
  match int {
    Lit(lit) => Ok(load_imm_a(dst, lit)),
    Var(mem) => load_a(dst, mem),
  }
}
pub(crate) fn load_imm_x(dst: X64Reg, qword: i64) -> X64Inst {
  match qword {
    0 => Clear(dst),
    _ => {
      if let Ok(dword) = i32::try_from(qword)
        && qword.is_positive()
      {
        MovMId(Reg(dst), dword.cast_unsigned())
      } else {
        MovRI(dst, qword.cast_unsigned())
      }
    }
  }
}
pub(crate) fn load_imm_a(dst: A64Reg, qword: i64) -> Vec<A64Inst> {
  match qword {
    0 => vec![MovRR(dst, Xzr)],
    _ => {
      let u_qw = qword.cast_unsigned();
      let parts: [u16; 4] =
        [u_qw as u16, (u_qw >> 16) as u16, (u_qw >> 32) as u16, (u_qw >> 48) as u16];
      let zero_cnt = parts.iter().filter(|&&part| part == 0).count();
      let max_cnt = parts.iter().filter(|&&part| part == u16::MAX).count();
      let is_mov_n = max_cnt > zero_cnt;
      let base = if is_mov_n { u16::MAX } else { 0 };
      let mut vec =
        vec![if is_mov_n { MovN(dst, !parts[0], Lsl0) } else { MovZ(dst, parts[0], Lsl0) }];
      for (part, lsl) in parts[1..].iter().zip([Lsl16, Lsl32, Lsl48]) {
        if *part != base {
          vec.push(MovK(dst, *part, lsl));
        }
      }
      vec
    }
  }
}
pub(crate) fn store_x(
  Memory(addr, mem_type): Memory,
  tmp: X64Reg,
  src: X64Reg,
) -> ErrOR<Vec<X64Inst>> {
  let reg_size = mem_type.reg_size();
  Ok(
    if mem_type.pass_by.is_ptr()
      && let Small(size) = mem_type.size
    {
      vec![
        load(S8, tmp, addr),
        match size {
          S8 => store(S8, Ref(tmp), src),
          S4 => store(S4, Ref(tmp), src),
          S1 => store(S1, Ref(tmp), src),
        },
      ]
    } else {
      vec![match reg_size {
        S8 => store(S8, addr, src),
        S4 => store(S4, addr, src),
        S1 => store(S1, addr, src),
      }]
    },
  )
}
pub(crate) fn store_a(
  Memory(addr, mem_type): Memory,
  tmp: A64Reg,
  src: A64Reg,
) -> ErrOR<Vec<A64Inst>> {
  let reg_size = mem_type.reg_size();
  let mut insts = vec![];
  match addr {
    Global(lbl) => insts.extend_from_slice(&[Adrp(tmp, lbl), AddLbl(tmp, lbl)]),
    Local(_, offset) => {
      insts.extend_from_slice(&load_imm_a(tmp, offset as i64));
      insts.push(AddR3(tmp, X29, tmp));
    }
  };
  if mem_type.pass_by.is_ptr() && matches!(mem_type.size, Small(_)) {
    insts.push(LdR(S8, tmp, tmp, 0));
  }
  insts.push(StR(reg_size, src, tmp, 0));
  Ok(insts)
}
pub(crate) fn load_x(dst: X64Reg, Memory(addr, mem_type): Memory) -> ErrOR<Vec<X64Inst>> {
  let reg_size = mem_type.reg_size();
  if mem_type.pass_by == Value && matches!(mem_type.size, Known(_) | Dynamic) {
    return Ok(vec![LeaRM(dst, addr)]);
  }
  let mut insts = vec![match reg_size {
    S8 => load(S8, dst, addr),
    S4 => load(S4, dst, addr),
    S1 => load(S1, dst, addr),
  }];
  if mem_type.pass_by.is_ptr()
    && let Small(size) = mem_type.size
  {
    insts.push(match size {
      S8 => load(S8, dst, Ref(dst)),
      S4 => load(S4, dst, Ref(dst)),
      S1 => load(S1, dst, Ref(dst)),
    });
  }
  Ok(insts)
}
pub(crate) fn load_a(dst: A64Reg, Memory(addr, mem_type): Memory) -> ErrOR<Vec<A64Inst>> {
  let reg_size = mem_type.reg_size();
  let mut insts = load_ptr_a(dst, addr)?;
  if mem_type.pass_by == Value && matches!(mem_type.size, Known(_) | Dynamic) {
    return Ok(insts);
  }
  insts.push(LdR(reg_size, dst, dst, 0));
  if mem_type.pass_by.is_ptr()
    && let Small(size) = mem_type.size
  {
    insts.push(LdR(size, dst, dst, 0));
  }
  Ok(insts)
}
pub(crate) fn load_ref_a(dst: A64Reg, Memory(addr, mem_type): Memory) -> ErrOR<Vec<A64Inst>> {
  let mut insts = load_ptr_a(dst, addr)?;
  if mem_type.pass_by.is_ptr() && matches!(mem_type.size, Small(_)) {
    insts.push(LdR(S8, dst, dst, 0));
  }
  Ok(insts)
}
pub(crate) fn load_ptr_a(dst: A64Reg, addr: Address) -> ErrOR<Vec<A64Inst>> {
  Ok(match addr {
    Global(lbl) => vec![Adrp(dst, lbl), AddLbl(dst, lbl)],
    Local(_, offset) => [load_imm_a(dst, offset as i64), vec![AddR3(dst, X29, dst)]].concat(),
  })
}
pub(crate) fn load_xmm(xmm: X64Reg, tmp: X64Reg, mem: Memory) -> ErrOR<Vec<X64Inst>> {
  Ok(if mem.1.pass_by.is_ptr() {
    vec![MovRO(S8, tmp, Mem(mem.0)), MovSdO(xmm, Ref(tmp))]
  } else {
    vec![MovSdO(xmm, Mem(mem.0))]
  })
}
pub(crate) fn store_xmm(mem: Memory, tmp: X64Reg, xmm: X64Reg) -> ErrOR<Vec<X64Inst>> {
  Ok(if mem.1.pass_by.is_ptr() {
    vec![MovRO(S8, tmp, Mem(mem.0)), MovOSd(Ref(tmp), xmm)]
  } else {
    vec![MovOSd(Mem(mem.0), xmm)]
  })
}
pub(crate) fn store_dn(mem: Memory, tmp: A64Reg, dn: A64Reg) -> ErrOR<Vec<A64Inst>> {
  let mut insts = load_ref_a(tmp, mem)?;
  insts.push(FStRD(dn, tmp, 0));
  Ok(insts)
}
pub(crate) fn inc_a(mem: Memory, tmp: A64Reg, tmp2: A64Reg) -> ErrOR<Vec<A64Inst>> {
  Ok([load_a(tmp, mem)?, vec![AddRI12(tmp, tmp, 1)], store_a(mem, tmp2, tmp)?].concat())
}
pub(crate) fn mov(reg_size: RegSize, dst: X64Reg, src: X64Reg) -> X64Inst {
  MovRO(reg_size, dst, Reg(src))
}
pub(crate) fn m8i<T>(kind: Group1, dst: T, imm: i32) -> X64Inst
where
  T: Into<X64Operand>,
{
  MId(S8, kind, dst.into(), imm)
}
pub(crate) fn m4i<T>(kind: Group1, dst: T, imm: i32) -> X64Inst
where
  T: Into<X64Operand>,
{
  MId(S4, kind, dst.into(), imm)
}
pub(crate) fn m_r<T>(reg_size: RegSize, kind: Group1, dst: T, src: X64Reg) -> X64Inst
where
  T: Into<X64Operand>,
{
  MR(reg_size, kind, dst.into(), src)
}
pub(crate) fn r_m<T>(reg_size: RegSize, kind: Group1, dst: X64Reg, src: T) -> X64Inst
where
  T: Into<X64Operand>,
{
  RM(reg_size, kind, dst, src.into())
}
pub(crate) fn store<T>(reg_size: RegSize, dst: T, src: X64Reg) -> X64Inst
where
  T: Into<X64Operand>,
{
  MovOR(reg_size, dst.into(), src)
}
pub(crate) fn load<T>(reg_size: RegSize, dst: X64Reg, src: T) -> X64Inst
where
  T: Into<X64Operand>,
{
  MovRO(reg_size, dst, src.into())
}
