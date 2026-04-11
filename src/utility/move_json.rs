use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn mov_args_json(
    &mut self,
    idx: u32,
    scope: &mut Scope,
    arg: WithPos<Json>,
    copy: bool,
  ) -> ErrOR<()> {
    let reg = *ARG_REGS.get(idx as usize).unwrap_or(&Rax);
    if !copy {
      scope.extend(&self.mov_json(reg, arg, None)?);
      if reg == Rax {
        scope.push(mov_q(Args(i32::try_from(idx + 1)?), Rax));
      }
      return Ok(());
    }
    if arg.val.as_type() != StrT {
      scope.extend(&self.mov_json(reg, arg, Some(scope.id))?);
      if reg == Rax {
        scope.push(mov_q(Args(i32::try_from(idx + 1)?), Rax));
      }
      return Ok(());
    }
    let tmp = scope.alloc(0x20, 8)?;
    for (tmp_idx, tmp_reg) in ARG_REGS.iter().enumerate() {
      if *tmp_reg != reg {
        scope.push(mov_q(Local(Tmp, tmp + i32::try_from(tmp_idx * 8)?), *tmp_reg));
      }
    }
    scope.extend(&self.mov_json(reg, arg, Some(scope.id))?);
    if reg == Rax {
      scope.push(mov_q(Args(i32::try_from(idx + 1)?), Rax));
    }
    for (tmp_idx, tmp_reg) in ARG_REGS.iter().enumerate() {
      if *tmp_reg != reg {
        scope.push(mov_q(*tmp_reg, Local(Tmp, tmp + i32::try_from(tmp_idx * 8)?)));
      }
    }
    scope.free(tmp, Size(0x20));
    Ok(())
  }
  pub(crate) fn mov_float_xmm(
    &mut self,
    xmm: Register,
    tmp: Register,
    float: Bind<f64>,
  ) -> Vec<Inst> {
    match float {
      Lit(lit) => vec![MovSdM(xmm, self.global_q(lit.to_bits()).0)],
      Var(memory) => mov_memory_xmm(xmm, tmp, memory),
    }
  }
  pub(crate) fn mov_json(
    &mut self,
    dst: Register,
    src: WithPos<Json>,
    copy: Option<LabelId>,
  ) -> ErrOR<Vec<Inst>> {
    match src.val {
      Null(_) => Ok(vec![Clear(dst)]),
      Bool(boolean) => Ok(mov_bool(dst, boolean)),
      Int(int) => Ok(mov_int(dst, int)),
      Float(float) => Ok(mov_float_reg(dst, float)),
      Str(string) => Ok(if let Some(caller) = copy {
        vec![self.mov_str(Rcx, string), Call(self.copy_str(caller)?), mov_q(dst, Rax)]
      } else {
        vec![self.mov_str(dst, string)]
      }),
      Array(_) | Object(_) => err!(src.pos, UnsupportedType(src.val.describe())),
    }
  }
  pub(crate) fn mov_str(&mut self, dst: Register, string: Bind<String>) -> Inst {
    match string {
      Lit(lit) => LeaRM(dst, Global(self.global_str(lit))),
      Var(Memory(addr, _)) => mov_q(dst, addr),
    }
  }
}
pub(crate) fn mov_bool(dst: Register, boolean: Bind<bool>) -> Vec<Inst> {
  match boolean {
    Lit(lit) => vec![mov_b(dst, bool2byte(lit))],
    Var(memory) => mov_memory(dst, memory),
  }
}
pub(crate) fn mov_float_reg(dst: Register, float: Bind<f64>) -> Vec<Inst> {
  match float {
    Lit(lit) => vec![mov_q(dst, lit.to_bits())],
    Var(memory) => mov_memory(dst, memory),
  }
}
pub(crate) fn mov_int(dst: Register, int: Bind<i64>) -> Vec<Inst> {
  match int {
    Lit(lit) => vec![mov_imm(dst, lit)],
    Var(memory) => mov_memory(dst, memory),
  }
}
pub(crate) fn mov_imm(dst: Register, qword: i64) -> Inst {
  match qword {
    0 => Clear(dst),
    _ => {
      if let Ok(dword) = i32::try_from(qword)
        && qword.is_positive()
      {
        mov_d(dst, dword.cast_unsigned())
      } else {
        mov_q(dst, qword.cast_unsigned())
      }
    }
  }
}
pub(crate) fn mov_q<T: Into<Operand<u64>>, U: Into<Operand<u64>>>(dst: T, src: U) -> Inst {
  MovQQ((dst.into(), src.into()))
}
pub(crate) fn mov_d<T: Into<Operand<u32>>, U: Into<Operand<u32>>>(dst: T, src: U) -> Inst {
  MovDD((dst.into(), src.into()))
}
pub(crate) fn mov_b<T: Into<Operand<u8>>, U: Into<Operand<u8>>>(dst: T, src: U) -> Inst {
  MovBB((dst.into(), src.into()))
}
pub(crate) fn ret_memory(memory: Memory, tmp: Register, src: Register) -> Vec<Inst> {
  match memory {
    Memory(addr, Size(size)) => vec![if size == 1 { mov_b(addr, src) } else { mov_q(addr, src) }],
    Memory(addr, Heap(None)) => vec![mov_q(addr, src)],
    Memory(addr, Heap(Some(size))) => {
      vec![mov_q(tmp, addr), if size == 1 { mov_b(Ref(tmp), src) } else { mov_q(Ref(tmp), src) }]
    }
  }
}
pub(crate) fn mov_memory(dst: Register, memory: Memory) -> Vec<Inst> {
  match memory {
    Memory(addr, Size(size)) => vec![if size == 1 { mov_b(dst, addr) } else { mov_q(dst, addr) }],
    Memory(addr, Heap(None)) => vec![mov_q(dst, addr)],
    Memory(addr, Heap(Some(size))) => {
      vec![mov_q(dst, addr), if size == 1 { mov_b(dst, Ref(dst)) } else { mov_q(dst, Ref(dst)) }]
    }
  }
}
pub(crate) fn mov_memory_xmm(xmm: Register, tmp: Register, memory: Memory) -> Vec<Inst> {
  match memory {
    Memory(addr, Size(_) | Heap(None)) => vec![MovSdM(xmm, addr)],
    Memory(addr, Heap(Some(_))) => vec![mov_q(tmp, addr), MovSdRef(xmm, tmp)],
  }
}
pub(crate) fn ret_memory_xmm(memory: Memory, tmp: Register, xmm: Register) -> Vec<Inst> {
  match memory {
    Memory(addr, Size(_) | Heap(None)) => vec![MovMSd(addr, xmm)],
    Memory(addr, Heap(Some(_))) => vec![mov_q(tmp, addr), MovRefSd(tmp, xmm)],
  }
}
