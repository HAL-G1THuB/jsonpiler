use crate::{
  Bind::{self, Lit, Var},
  ConditionCode::E,
  ErrOR, FuncInfo,
  Inst::{self, *},
  Json, Jsonpiler, Label,
  OpQ::*,
  Parser,
  Reg::{self, *},
  ScopeInfo,
  VarKind::*,
  take_arg,
};
use std::{
  collections::HashMap,
  time::{SystemTime, UNIX_EPOCH},
};
impl Jsonpiler {
  pub(crate) const COMMON: (bool, bool) = (false, false);
  pub(crate) const CQO: [u8; 2] = [0x48, 0x99];
  pub(crate) const KERNEL32: &'static str = "kernel32.dll";
  pub(crate) const REGS: [Reg; 4] = [Rcx, Rdx, R8, R9];
  pub(crate) const SCOPE: (bool, bool) = (true, false);
  pub(crate) const SPECIAL: (bool, bool) = (false, true);
  pub(crate) const SP_SCOPE: (bool, bool) = (true, true);
  pub(crate) const USER32: &'static str = "user32.dll";
  pub(crate) fn call_api_check_null(&self, api: (usize, usize)) -> [Inst; 3] {
    [CallApi(api), TestRR(Rax, Rax), Jcc(E, self.sym_table["WIN_HANDLER"])]
  }
  // Overflow is unlikely.
  pub(crate) fn gen_id(&mut self) -> usize {
    let id = self.label_id;
    self.label_id += 1;
    id
  }
  pub(crate) fn get_bss_id(&mut self, size: u32) -> usize {
    let id = self.gen_id();
    self.insts.push(Bss(id, size));
    id
  }
  pub(crate) fn get_var(&self, var_name: &str, scope: &ScopeInfo) -> Option<Json> {
    scope.get_var_local(var_name).or_else(|| Some(self.globals.get(var_name)?.clone()))
  }
  pub(crate) fn global_bool(&mut self, boolean: bool) -> usize {
    let id = self.gen_id();
    self.insts.push(Byte(id, if boolean { 0xFF } else { 0 }));
    id
  }
  pub(crate) fn global_num(&mut self, value: u64) -> usize {
    let id = self.gen_id();
    self.insts.push(Quad(id, value));
    id
  }
  pub(crate) fn global_str(&mut self, value: String) -> usize {
    if let Some(&id) = self.str_cache.get(&value) {
      return id;
    }
    let len = value.len() as u64;
    let id = self.gen_id();
    let unused = self.gen_id();
    self.str_cache.insert(value.clone(), id);
    self.insts.push(Quad(id, len));
    self.insts.push(StringZ(unused, value));
    id
  }
  pub(crate) fn import(
    &mut self, key1: &'static str, key2: &'static str, id: u16,
  ) -> (usize, usize) {
    let outer_idx =
      if let Some(idx) = self.import_table.iter().position(|(outer_key, _)| *outer_key == key1) {
        idx
      } else {
        self.import_table.push((key1, Vec::new()));
        self.import_table.len() - 1
      };
    let inner_idx =
      if let Some(idx) = self.import_table[outer_idx].1.iter().position(|(id2, _)| *id2 == id) {
        idx
      } else {
        self.import_table[outer_idx].1.push((id, key2));
        self.import_table[outer_idx].1.len() - 1
      };
    (outer_idx, inner_idx)
  }
  pub(crate) fn mov_float_xmm(&self, float: &Bind<f64>, xmm: Reg, reg: Reg, scope: &mut ScopeInfo) {
    let addr = match float {
      Bind::Lit(l_float) => {
        scope.push(MovQQ(Rq(reg), Iq(l_float.to_bits())));
        let tmp = Global { id: self.sym_table["TMP"], disp: 0i32 };
        scope.push(MovQQ(Mq(tmp), Rq(reg)));
        tmp
      }
      Bind::Var(label) => label.kind,
    };
    scope.push(MovSdXM(xmm, addr));
  }
  pub(crate) fn mov_str(&mut self, string: Bind<String>, str_reg: Reg, scope: &mut ScopeInfo) {
    match string {
      Lit(l_str) => {
        let lbl = Global { id: self.global_str(l_str), disp: 8i32 };
        scope.push(LeaRM(str_reg, lbl));
      }
      Var(Label { kind: Global { id, .. }, .. }) => {
        scope.push(MovQQ(Rq(str_reg), Mq(Global { id, disp: 0i32 })));
        scope.push(AddRId(str_reg, 8));
      }
      Var(Label { kind: kind @ (Tmp { .. } | Local { .. }), .. }) => {
        scope.push(MovQQ(Rq(str_reg), Mq(kind)));
        scope.push(AddRId(str_reg, 8));
      }
    }
  }
  pub(crate) fn mov_str_len(
    &mut self, string: Bind<String>, str_reg: Reg, len_reg: Reg, scope: &mut ScopeInfo,
  ) {
    self.mov_str(string.clone(), str_reg, scope);
    mov_len(string, len_reg, scope);
  }
  // fn format(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {}
  #[inline]
  #[must_use]
  pub fn setup(source: Vec<u8>, name: String) -> Self {
    Self {
      builtin: HashMap::new(),
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
  pub(crate) fn take_bool(
    &self, reg: Reg, func: &mut FuncInfo, scope: &mut ScopeInfo,
  ) -> ErrOR<()> {
    let boolean = take_arg!(self, func, "Bool", Json::Bool(x) => x).value;
    mov_bool(&boolean, reg, scope);
    Ok(())
  }
  pub(crate) fn take_float(
    &self, xmm: Reg, reg: Reg, func: &mut FuncInfo, scope: &mut ScopeInfo,
  ) -> ErrOR<()> {
    let float = take_arg!(self, func, "Float", Json::Float(x) => x).value;
    self.mov_float_xmm(&float, xmm, reg, scope);
    Ok(())
  }
  pub(crate) fn take_int(&self, reg: Reg, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<()> {
    let int = take_arg!(self, func, "Int", Json::Int(x) => x).value;
    mov_int(&int, reg, scope);
    Ok(())
  }
  pub(crate) fn take_len(
    &mut self, len_reg: Reg, func: &mut FuncInfo, scope: &mut ScopeInfo,
  ) -> ErrOR<()> {
    let string = take_arg!(self, func, "String", Json::String(x) => x).value;
    mov_len(string, len_reg, scope);
    Ok(())
  }
  pub(crate) fn take_str(
    &mut self, reg: Reg, func: &mut FuncInfo, scope: &mut ScopeInfo,
  ) -> ErrOR<()> {
    let string = take_arg!(self, func, "String", Json::String(x) => x).value;
    self.mov_str(string, reg, scope);
    Ok(())
  }
  pub(crate) fn take_str_len(
    &mut self, str_reg: Reg, len_reg: Reg, func: &mut FuncInfo, scope: &mut ScopeInfo,
  ) -> ErrOR<()> {
    let string = take_arg!(self, func, "String", Json::String(x) => x).value;
    self.mov_str_len(string, str_reg, len_reg, scope);
    Ok(())
  }
}
pub(crate) fn align_up(num: usize, align: usize) -> ErrOR<usize> {
  num.div_ceil(align).checked_mul(align).ok_or("InternalError: Overflow".into())
}
pub(crate) fn align_up_32(value: u32, align: u32) -> ErrOR<u32> {
  value.div_ceil(align).checked_mul(align).ok_or("InternalError: Overflow".into())
}
pub(crate) fn align_up_i32(value: i32, align: i32) -> ErrOR<i32> {
  (value + align - 1)
    .checked_div(align)
    .and_then(|x| x.checked_mul(align))
    .ok_or("InternalError: Overflow".into())
}
#[expect(clippy::cast_possible_truncation)]
pub(crate) fn get_time_stamp() -> ErrOR<u32> {
  Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as u32)
}
pub(crate) fn mov_bool(boolean: &Bind<bool>, reg: Reg, scope: &mut ScopeInfo) {
  match boolean {
    Bind::Lit(l_bool) => scope.push(MovRbIb(reg, if *l_bool { 0xFF } else { 0 })),
    Bind::Var(label) => {
      scope.push(MovRbMb(reg, label.kind));
    }
  }
}
pub(crate) fn mov_float_reg(float: &Bind<f64>, reg: Reg, scope: &mut ScopeInfo) {
  let src = match float {
    Bind::Lit(l_float) => Iq(l_float.to_bits()),
    Bind::Var(label) => Mq(label.kind),
  };
  scope.push(MovQQ(Rq(reg), src));
}
pub(crate) fn mov_int(int: &Bind<i64>, reg: Reg, scope: &mut ScopeInfo) {
  #[expect(clippy::cast_sign_loss)]
  let src = match int {
    Bind::Lit(l_int) => Iq(*l_int as u64),
    Bind::Var(label) => Mq(label.kind),
  };
  scope.push(MovQQ(Rq(reg), src));
}
pub(crate) fn mov_len(string: Bind<String>, len_reg: Reg, scope: &mut ScopeInfo) {
  let src = match string {
    Lit(l_str) => Iq(l_str.len() as u64),
    Var(Label { kind: Global { id, .. }, .. }) => {
      scope.push(MovQQ(Rq(len_reg), Mq(Global { id, disp: 0i32 })));
      Ref(len_reg)
    }
    Var(Label { kind: kind @ (Tmp { .. } | Local { .. }), .. }) => {
      scope.push(MovQQ(Rq(len_reg), Mq(kind)));
      Ref(len_reg)
    }
  };
  scope.push(MovQQ(Rq(len_reg), src));
}
