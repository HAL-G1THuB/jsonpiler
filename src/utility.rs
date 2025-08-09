use crate::{
  Bind,
  Bind::{Lit, Var},
  CompileContext, ErrOR, FuncInfo, Json, Jsonpiler, Label, Parser, ScopeInfo,
  VarKind::Global,
  WithPos, err, mn, mn_write, take_arg,
};
use core::{fmt::UpperHex, iter};
use std::{
  collections::HashMap,
  fs::File,
  io::{BufWriter, Write as _},
};
impl Jsonpiler {
  pub(crate) const COMMON: (bool, bool) = (false, false);
  pub(crate) const SPECIAL: (bool, bool) = (false, true);
  pub(crate) const SP_SCOPE: (bool, bool) = (true, true);
  // label.kind == Local
  pub(crate) fn get_argument(&mut self, jwp: &WithPos<Json>) -> ErrOR<String> {
    match &jwp.value {
      Json::String(string) => Ok(match string {
        Lit(l_str) => format!("{}", self.global_str(l_str)?),
        Var(label) => format!("[{label}]"),
      }),
      Json::Float(Var(label)) | Json::Bool(Bind::Var(label)) | Json::Int(Var(label)) => {
        Ok(format!("{label}"))
      }
      Json::Null => Ok("0".to_owned()),
      Json::Int(Bind::Lit(l_int)) => Ok(l_int.to_string()),
      Json::Bool(Bind::Lit(l_bool)) => Ok(if *l_bool { "0xFF" } else { "0x00" }.to_owned()),
      Json::Float(Lit(l_float)) => Ok(format!("{:#016x}", l_float.to_bits())),
      Json::Array(_) | Json::Object(_) | Json::Function(_) => {
        err!(
          self,
          jwp.pos,
          "This type cannot be accepted as an argument of an user-defined function."
        )
      }
    }
  }
  pub(crate) fn get_bool_str(&self, func: &mut FuncInfo, nth: usize) -> ErrOR<String> {
    let boolean = take_arg!(self, func, nth, "Bool", Json::Bool(x) => x).0;
    Ok(match boolean {
      Bind::Lit(l_bool) => if l_bool { "0xFF" } else { "0x00" }.to_owned(),
      Bind::Var(label) => label.sched_free_2str(func),
    })
  }
  pub(crate) fn get_bss(&mut self, size: usize) -> ErrOR<Label> {
    let label = self.ctx.global(size)?;
    self.bss.push((label.id, size));
    Ok(label)
  }
  pub(crate) fn get_int_str(&self, func: &mut FuncInfo, nth: usize) -> ErrOR<String> {
    let int = take_arg!(self, func, nth, "Int", Json::Int(x) => x).0;
    Ok(match int {
      Bind::Lit(l_int) => l_int.to_string(),
      Bind::Var(label) => label.sched_free_2str(func),
    })
  }
  pub(crate) fn get_str_str(&mut self, func: &mut FuncInfo, nth: usize) -> ErrOR<String> {
    let string = take_arg!(self, func, nth, "String", Json::String(x) => x).0;
    Ok(match string {
      Lit(l_str) => format!("{}", self.global_str(&l_str)?),
      Var(label) if label.kind == Global => format!("{label}"),
      Var(label) => format!("[{}]", label.sched_free_2str(func)),
    })
  }
  pub(crate) fn get_var(&self, var_name: &str, scope: &ScopeInfo) -> Option<Json> {
    for table in scope.locals.iter().rev().chain(iter::once(&self.globals)) {
      if let Some(val) = table.get(var_name) {
        return Some(val.clone());
      }
    }
    None
  }
  pub(crate) fn global_bool(&mut self, boolean: bool) -> ErrOR<Label> {
    let label = self.ctx.global(8)?;
    self.data.write_all(label.to_def().as_bytes())?;
    mn_write!(self.data, ".byte", if boolean { "0xFF" } else { "0x00" });
    Ok(label)
  }
  pub(crate) fn global_num<T: UpperHex>(&mut self, value: T) -> ErrOR<Label> {
    let label = self.ctx.global(8)?;
    mn_write!(self.data, ".align", "8");
    self.data.write_all(label.to_def().as_bytes())?;
    mn_write!(self.data, ".quad", format!("{value:#X}"));
    Ok(label)
  }
  pub(crate) fn global_str(&mut self, value: &str) -> ErrOR<Label> {
    if let Some(cached_label) = self.ctx.get_cache(value) {
      return Ok(Label { kind: Global, id: cached_label, size: 8 });
    }
    let label = self.ctx.global(8)?;
    self.ctx.insert_cache(value, label.id);
    self.data.write_all(label.to_def().as_bytes())?;
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
      parser: Parser::from(source),
      text: vec![],
      ctx: CompileContext::default(),
    })
  }
}
pub(crate) fn get_argument_mem(idx: usize, size: usize) -> ErrOR<String> {
  const REGS: [&str; 4] = ["rcx", "rdx", "r8", "r9"];
  Ok(if let Some(&reg) = REGS.get(idx) {
    reg.to_owned()
  } else {
    format!(
      "{}\tptr\t{}[rsp]",
      get_prefix(size).ok_or("InternalError: Invalid size")?,
      8 * (idx - 4)
    )
  })
}
#[expect(clippy::single_call_fn)]
pub(crate) fn imp_call(func: &str) -> String {
  mn!("call", format!("[qword\tptr\t__imp_{func}[rip]]"))
}
pub(crate) fn get_prefix(num: usize) -> Option<&'static str> {
  match num {
    1 => Some("byte"),
    2 => Some("word"),
    4 => Some("dword"),
    8 => Some("qword"),
    _ => None,
  }
}
