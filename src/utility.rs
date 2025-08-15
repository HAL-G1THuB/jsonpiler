use crate::{
  Bind::{self, Lit, Var},
  CompileContext, ErrOR, FuncInfo,
  Inst::{self, *},
  Json, Jsonpiler, Label,
  OpQ::{Iq, Mq, Rq},
  Parser,
  Reg::{self, *},
  ScopeInfo,
  VarKind::Global,
  take_arg,
};
use std::{
  collections::HashMap,
  time::{SystemTime, UNIX_EPOCH},
};
impl Jsonpiler {
  pub(crate) const COMMON: (bool, bool) = (false, false);
  pub(crate) const KERNEL32: &'static str = "kernel32.dll";
  pub(crate) const REGS: [Reg; 4] = [Rcx, Rdx, R8, R9];
  pub(crate) const SPECIAL: (bool, bool) = (false, true);
  pub(crate) const SP_SCOPE: (bool, bool) = (true, true);
  pub(crate) const USER32: &'static str = "user32.dll";
  pub(crate) fn get_bss_id(&mut self, size: u32) -> usize {
    let id = self.ctx.gen_id();
    self.insts.push(Bss(id, size));
    id
  }
  pub(crate) fn get_var(&self, var_name: &str, scope: &ScopeInfo) -> Option<Json> {
    for table in scope.iter_all_scope(&self.globals) {
      if let Some(val) = table.get(var_name) {
        return Some(val.clone());
      }
    }
    None
  }
  pub(crate) fn global_bool(&mut self, boolean: bool) -> usize {
    let id = self.ctx.gen_id();
    self.insts.push(Byte(id, if boolean { 0xFF } else { 0 }));
    id
  }
  pub(crate) fn global_num(&mut self, value: u64) -> usize {
    let id = self.ctx.gen_id();
    self.insts.push(Quad(id, value));
    id
  }
  pub(crate) fn global_str(&mut self, value: String) -> usize {
    if let Some(&id) = self.ctx.str_cache.get(&value) {
      return id;
    }
    let id = self.ctx.gen_id();
    self.ctx.str_cache.insert(value.clone(), id);
    self.insts.push(StringZ(id, value));
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
  pub(crate) fn mov_bool(
    &self, reg: Reg, func: &mut FuncInfo, nth: usize, scope: &mut ScopeInfo,
  ) -> ErrOR<()> {
    let boolean = take_arg!(self, func, nth, "Bool", Json::Bool(x) => x).0;
    match boolean {
      Bind::Lit(l_bool) => scope.push(MovRbIb(reg, if l_bool { 0xFF } else { 0 })),
      Bind::Var(label) => {
        func.sched_free_tmp(&label);
        scope.push(MovRbMb(reg, label.kind));
      }
    }
    Ok(())
  }
  pub(crate) fn mov_int(
    &self, reg: Reg, func: &mut FuncInfo, nth: usize, scope: &mut ScopeInfo,
  ) -> ErrOR<()> {
    let int = take_arg!(self, func, nth, "Int", Json::Int(x) => x).0;
    match int {
      #[expect(clippy::cast_sign_loss)]
      Bind::Lit(l_int) => scope.push(MovQQ(Rq(reg), Iq(l_int as u64))),
      Bind::Var(label) => {
        func.sched_free_tmp(&label);
        scope.push(MovQQ(Rq(reg), Mq(label.kind)));
      }
    }
    Ok(())
  }
  pub(crate) fn mov_str(&mut self, reg: Reg, func: &mut FuncInfo, nth: usize) -> ErrOR<Inst> {
    let string = take_arg!(self, func, nth, "String", Json::String(x) => x).0;
    Ok(match string {
      Lit(l_str) => LeaRM(reg, Global { id: self.global_str(l_str) }),
      Var(Label { kind: kind @ Global { .. }, .. }) => LeaRM(reg, kind),
      Var(label) => {
        func.sched_free_tmp(&label);
        MovRbMb(reg, label.kind)
      }
    })
  }
  // fn format(&mut self, func: &mut FuncInfo, scope: &mut ScopeInfo) -> ErrOR<Json> {}
  #[inline]
  #[must_use]
  pub fn setup(source: Vec<u8>) -> Self {
    Self {
      builtin: HashMap::new(),
      ctx: CompileContext::default(),
      globals: HashMap::new(),
      parser: Parser::from(source),
      insts: vec![],
      sym_table: HashMap::new(),
      import_table: vec![],
    }
  }
}
pub(crate) fn get_prefix(num: u32) -> Option<&'static str> {
  match num {
    1 => Some("byte"),
    2 => Some("word"),
    4 => Some("dword"),
    8 => Some("qword"),
    _ => None,
  }
}
pub(crate) fn align_up(num: usize, align: usize) -> ErrOR<usize> {
  num.div_ceil(align).checked_mul(align).ok_or("InternalError: Overflow".into())
}
pub(crate) fn align_up_32(value: u32, align: u32) -> ErrOR<u32> {
  value.div_ceil(align).checked_mul(align).ok_or("InternalError: Overflow".into())
}
#[expect(clippy::cast_possible_truncation)]
pub(crate) fn get_time_stamp() -> ErrOR<u32> {
  Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as u32)
}
