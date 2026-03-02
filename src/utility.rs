use crate::prelude::*;
use std::{
  env, io,
  path::Path,
  time::{SystemTime, UNIX_EPOCH},
};
impl Jsonpiler {
  pub(crate) fn bss(&mut self, size: u32, align: u32) -> u32 {
    let id = self.id();
    self.data_insts.push(BssAlloc(id, size, align));
    id
  }
  pub(crate) fn global_b(&mut self, boolean: bool) -> Label {
    let id = self.id();
    if boolean {
      self.data_insts.push(Byte(id, bool2byte(boolean)));
    } else {
      self.data_insts.push(BssAlloc(id, 1, 1));
    }
    Label(Global(id), Size(1))
  }
  pub(crate) fn global_q(&mut self, value: u64) -> Label {
    let id = self.id();
    if value != 0 {
      self.data_insts.push(Quad(id, value));
    } else {
      self.data_insts.push(BssAlloc(id, 8, 8));
    }
    Label(Global(id), Size(8))
  }
  pub(crate) fn global_str<T: Into<String>>(&mut self, value: T) -> u32 {
    let string = value.into();
    if let Some(&id) = self.str_cache.get(&string) {
      return id;
    }
    let id = self.id();
    self.str_cache.insert(string.clone(), id);
    self.data_insts.push(Bytes(id, string.into()));
    id
  }
  pub(crate) fn global_w_chars<T: Into<String>>(&mut self, value: T) -> u32 {
    let string = value.into();
    if let Some(&id) = self.str_cache.get(&string) {
      return id;
    }
    let id = self.id();
    self.str_cache.insert(string.clone(), id);
    self.data_insts.push(WChars(id, string.into()));
    id
  }
  // Overflow is unlikely
  pub(crate) fn id(&mut self) -> u32 {
    self.id_seed += 1;
    self.id_seed - 1
  }
}
impl Jsonpiler {
  pub(crate) fn err_info(&self, pos: Position) -> (String, String, String, String) {
    self.parser[pos.file].err_info(pos, &self.parser[0].file)
  }
  pub(crate) fn get_var(
    &mut self,
    var_name: &str,
    pos: Position,
    scope: &mut Scope,
  ) -> ErrOR<Json> {
    or_err!(
      (scope.get_var_local(var_name).or_else(|| self.globals.get(var_name).cloned())),
      pos,
      UndefinedVar(var_name.into())
    )
  }
  pub(crate) fn import(&mut self, dll: &'static str, func: &'static str) -> ErrOR<(u32, u32)> {
    let idx = self.dlls.iter().position(|(dll2, _)| *dll2 == dll).unwrap_or_else(|| {
      self.dlls.push((dll, vec![]));
      self.dlls.len() - 1
    });
    let idx2 = self.dlls[idx].1.iter().position(|func2| *func2 == func).unwrap_or_else(|| {
      self.dlls[idx].1.push(func);
      self.dlls[idx].1.len() - 1
    });
    Ok((u32::try_from(idx)?, u32::try_from(idx2)?))
  }
}
impl Jsonpiler {
  pub(crate) fn mov_deep_json(&mut self, dst: Register, jwp: WithPos<Json>) -> ErrOR<Vec<Inst>> {
    Ok(match jwp.val {
      Null => vec![Clear(dst)],
      Bool(boolean) => mov_bool(dst, boolean),
      Int(int) => mov_int(dst, int),
      Float(float) => mov_float_reg(dst, float),
      Str(string) => vec![self.mov_str(Rcx, string), Call(self.copy_str()?), mov_q(dst, Rax)],
      Array(_) | Object(_) => return err!(jwp.pos, UnsupportedType(jwp.val.describe())),
    })
  }
  pub(crate) fn mov_float_xmm(
    &mut self,
    xmm: Register,
    tmp: Register,
    float: Bind<f64>,
  ) -> Vec<Inst> {
    match float {
      Lit(lit) => vec![MovSdXM(xmm, self.global_q(lit.to_bits()).0)],
      Var(label) => mov_label_xmm(xmm, tmp, label),
    }
  }
  #[expect(clippy::cast_possible_wrap)]
  pub(crate) fn mov_len(
    &mut self,
    dst: Register,
    string: &Bind<String>,
    scope: &mut Scope,
  ) -> ErrOR<()> {
    match string {
      Lit(lit) => scope.extend(&mov_int(dst, Lit(lit.len() as i64))),
      Var(label) => {
        let tmp_d = scope.alloc(0x18, 8)?;
        for (idx, reg) in [Rdi, Rax, Rcx].iter().enumerate() {
          if *reg != dst {
            scope.push(mov_q(Local(Tmp, tmp_d + i32::try_from(idx * 8)?), *reg));
          }
        }
        scope.extend(&[
          self.mov_str(Rdi, Var(*label)),
          Clear(Rcx),
          DecR(Rcx),
          Clear(Rax),
          Custom(CLD_REPNE_SCASB),
          self.mov_str(Rax, Var(*label)),
          SubRR(Rdi, Rax),
          DecR(Rdi),
          mov_q(dst, Rdi),
        ]);
        for (idx, reg) in [Rdi, Rax, Rcx].iter().enumerate() {
          if *reg != dst {
            scope.push(mov_q(*reg, Local(Tmp, tmp_d + i32::try_from(idx * 8)?)));
          }
        }
        scope.free(tmp_d, Size(0x18));
      }
    }
    Ok(())
  }
  pub(crate) fn mov_str(&mut self, dst: Register, string: Bind<String>) -> Inst {
    match string {
      Lit(lit) => LeaRM(dst, Global(self.global_str(lit))),
      Var(Label(addr, _)) => mov_q(dst, addr),
    }
  }
}
pub(crate) fn align_up(num: usize, align: usize) -> ErrOR<usize> {
  num.div_ceil(align).checked_mul(align).ok_or(Internal(OverFlow))
}
pub(crate) fn align_up_32(num: u32, align: u32) -> ErrOR<u32> {
  num.div_ceil(align).checked_mul(align).ok_or(Internal(OverFlow))
}
pub(crate) fn align_down_i32(num: i32, align: i32) -> ErrOR<i32> {
  num.div_euclid(align).checked_mul(align).ok_or(Internal(OverFlow))
}
#[expect(clippy::cast_possible_truncation)]
pub(crate) fn time_stamp() -> u32 {
  SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as u32
}
pub(crate) fn mov_bool(dst: Register, boolean: Bind<bool>) -> Vec<Inst> {
  match boolean {
    Lit(lit) => vec![mov_b(dst, bool2byte(lit))],
    Var(label) => mov_label(dst, label, 1, false),
  }
}
pub(crate) fn mov_float_reg(dst: Register, float: Bind<f64>) -> Vec<Inst> {
  match float {
    Lit(lit) => vec![mov_q(dst, lit.to_bits())],
    Var(label) => mov_label(dst, label, 8, false),
  }
}
pub(crate) fn mov_int(dst: Register, int: Bind<i64>) -> Vec<Inst> {
  match int {
    Lit(lit) => vec![mov_imm(dst, lit)],
    Var(label) => mov_label(dst, label, 8, false),
  }
}
pub(crate) fn mov_imm(dst: Register, lit: i64) -> Inst {
  if lit == 0 {
    Clear(dst)
  } else if let Ok(l_i32) = i32::try_from(lit)
    && lit.is_positive()
  {
    mov_d(dst, l_i32 as u32)
  } else {
    mov_q(dst, lit as u64)
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
pub(crate) fn ret_label(
  Label(addr, size): Label,
  tmp: Register,
  src: Register,
  size_i32: i32,
  is_str: bool,
) -> Vec<Inst> {
  let mov2addr = if size_i32 == 1 { mov_b(addr, src) } else { mov_q(addr, src) };
  match size {
    _ if is_str => vec![mov2addr],
    Size(_) => vec![mov2addr],
    Heap => {
      vec![
        mov_q(tmp, addr),
        if size_i32 == 1 { mov_b(Ref(tmp), src) } else { mov_q(Ref(tmp), src) },
      ]
    }
  }
}
pub(crate) fn mov_label(
  dst: Register,
  Label(addr, size): Label,
  size_i32: i32,
  is_str: bool,
) -> Vec<Inst> {
  let mov2dst = if size_i32 == 1 { mov_b(dst, addr) } else { mov_q(dst, addr) };
  match size {
    _ if is_str => vec![mov2dst],
    Size(_) => vec![mov2dst],
    Heap => vec![
      mov_q(dst, addr),
      if size_i32 == 1 { mov_b(dst, Ref(dst)) } else { mov_q(dst, Ref(dst)) },
    ],
  }
}
pub(crate) fn mov_label_xmm(xmm: Register, tmp: Register, Label(addr, size): Label) -> Vec<Inst> {
  if size == Heap { vec![mov_q(tmp, addr), MovSdXRef(xmm, tmp)] } else { vec![MovSdXM(xmm, addr)] }
}
pub(crate) fn ret_label_xmm(Label(addr, size): Label, tmp: Register, xmm: Register) -> Vec<Inst> {
  if size == Heap { vec![mov_q(tmp, addr), MovSdRefX(tmp, xmm)] } else { vec![MovSdMX(addr, xmm)] }
}
pub(crate) fn v_size<T>(data: &[T]) -> ErrOR<u32> {
  u32::try_from(data.len()).map_err(Into::into)
}
pub(crate) fn r_size(data: u32) -> ErrOR<u32> {
  align_up_32(data, FILE_ALIGNMENT)
}
#[rustfmt::skip]
pub(crate) fn validate_args(func: &Function, expected: Arity) -> ErrOR<()> {
  let actual = func.len;
  match expected {
    Exactly(n) => if actual == n { return Ok(()) }
    AtLeast(min) => if min <= actual { return Ok(()) }
    AtMost(max) => if actual <= max { return Ok(()) }
    Range(min, max) => if min <= actual && actual <= max { return Ok(()) }
    Zero => if actual == 0 { return Ok(()) }
    Any => (),
  }
  err!(func.pos, ArityError { name: func.name.clone(), expected, actual })
}
pub(crate) fn bool2byte(boolean: bool) -> u8 {
  if boolean { 0xFF } else { 0 }
}
pub(crate) fn full_path(file: &str) -> Result<String, io::Error> {
  Ok(
    env::current_dir()
      .and_then(|dir| dir.join(Path::new(file)).canonicalize())?
      .to_string_lossy()
      .to_string(),
  )
}
