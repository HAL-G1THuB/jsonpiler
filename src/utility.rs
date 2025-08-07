use crate::{
  Bind, CompileContext, ErrOR, FuncInfo, Json, Jsonpiler, Label, Parser, Position, ScopeInfo,
  VarKind::Global, mn, mn_write,
};
use core::{fmt::LowerHex, iter};
use std::{
  collections::HashMap,
  fs::File,
  io::{BufWriter, Write as _},
};
impl Jsonpiler {
  pub(crate) fn get_bss(&mut self, size: usize) -> ErrOR<Label> {
    let label = self.ctx.label(size)?;
    self.bss.push((label.id, size));
    Ok(label)
  }
  pub(crate) fn get_var(&self, var_name: &str, scope: &ScopeInfo) -> Option<Json> {
    for table in scope.locals.iter().rev().chain(iter::once(&self.globals)) {
      if let Some(val) = table.get(var_name) {
        return Some(val.clone());
      }
    }
    None
  }
  pub(crate) fn global_num<T: LowerHex>(&mut self, value: T) -> ErrOR<Label> {
    let label = self.ctx.label(8)?;
    mn_write!(self.data, ".align", "8");
    mn_write!(self.data, label.to_def());
    mn_write!(self.data, ".quad", format!("{value:#x}"));
    Ok(label)
  }
  pub(crate) fn global_str(&mut self, value: &str) -> ErrOR<Label> {
    if let Some(cached_label) = self.ctx.get_cache(value) {
      return Ok(Label { kind: Global, id: cached_label, size: 8 });
    }
    let label = self.ctx.label(8)?;
    self.ctx.insert_cache(value, label.id);
    mn_write!(self.data, label.to_def());
    mn_write!(self.data, ".string", format!("\"{value}\""));
    Ok(label)
  }
  #[inline]
  pub fn setup(source: Vec<u8>, out_file: &str) -> ErrOR<Self> {
    Ok(Self {
      bss: vec![],
      builtin: HashMap::new(),
      globals: HashMap::new(),
      data: BufWriter::new(File::create(out_file)?),
      parser: Parser { pos: Position { offset: 0, line: 1, size: 1 }, source },
      text: vec![],
      ctx: CompileContext::default(),
    })
  }
}
pub(crate) fn get_int_str(int: &Bind<i64>, func: &mut FuncInfo) -> String {
  match int {
    Bind::Lit(l_int) => l_int.to_string(),
    Bind::Var(label) => label.sched_free_2str(func),
  }
}
pub(crate) fn get_int_str_without_free(int: &Bind<i64>) -> String {
  match int {
    Bind::Lit(l_int) => l_int.to_string(),
    Bind::Var(label) => format!("{label}"),
  }
}
pub(crate) fn get_bool_str(boolean: &Bind<bool>, func: &mut FuncInfo) -> String {
  match boolean {
    Bind::Lit(l_bool) => if *l_bool { -1i8 } else { 0i8 }.to_string(),
    Bind::Var(label) => label.sched_free_2str(func),
  }
}
#[expect(clippy::single_call_fn, reason = "")]
pub(crate) fn imp_call(func: &str) -> String {
  mn!("call", format!("[qword ptr __imp_{func}[rip]]"))
}
