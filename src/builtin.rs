mod arithmetic;
mod construct;
mod control;
mod evaluate;
mod logical;
mod output;
mod variable;
use super::{
  ArgLen,
  Bind::{Lit, Var},
  Builtin, ErrOR, FuncInfo, JFunc, Json, Jsonpiler, Position, ScopeInfo, WithPos, err, mn,
  mn_write, sub,
  utility::{get_int_str, imp_call},
};
use core::mem::take;
use std::{
  collections::{HashMap, HashSet, VecDeque},
  fs::File,
  io::{self, Write as _},
};
impl Jsonpiler {
  pub(crate) fn all_register(&mut self) {
    self.register_output();
    self.register_logical();
    self.register_control();
    self.register_arithmetic();
    self.register_construct();
    self.register_variable();
    self.register_evaluate();
  }
  #[inline]
  pub fn build(&mut self, source: String, out_file: &str) -> ErrOR<()> {
    let json = self.parse(source)?;
    self.include_flag = HashSet::new();
    self.text = vec![];
    self.bss = vec![];
    self.data = vec![];
    self.label_id = 0;
    self.builtin = HashMap::new();
    self.vars_global = HashMap::new();
    self.vars_local = vec![HashMap::new()];
    self.all_register();
    let mut scope = ScopeInfo::default();
    let result = self.eval(json.value, &mut scope)?;
    let mut writer = io::BufWriter::new(File::create(out_file)?);
    mn_write!(&mut writer, ".intel_syntax", "noprefix")?;
    writer.write_all(
      format!(include_str!("asm/once/data.s"), msg = include_str!("txt/SEH_HANDLER_MSG.txt"))
        .as_bytes(),
    )?;
    for data in &mut self.data {
      writer.write_all(data.as_bytes())?;
    }
    writer.write_all(include_bytes!("asm/once/bss.s"))?;
    for bss in &mut self.bss {
      writer.write_all(bss.as_bytes())?;
    }
    writer.write_all(include_bytes!("asm/once/main.s"))?;
    write!(
      writer,
      include_str!("asm/common/prologue.s"),
      size = format!("{:#x}", scope.calc_alloc(8)?)
    )?;
    writer.write_all(include_bytes!("asm/once/startup.s"))?;
    for body in &mut scope.body {
      writer.write_all(body.as_bytes())?;
    }
    if let Json::Int(int) = result {
      mn_write!(&mut writer, "mov", "rcx", &get_int_str(&int, &mut scope)?)
    } else {
      mn_write!(&mut writer, "xor", "ecx", "ecx")
    }?;
    writer.write_all(imp_call("ExitProcess").as_bytes())?;
    mn_write!(&mut writer, ".seh_endproc")?;
    for text in &mut self.text {
      writer.write_all(text.as_bytes())?;
    }
    writer.write_all(include_bytes!("asm/once/handler.s"))?;
    writer.flush()?;
    Ok(())
  }
  pub(crate) fn eval(&mut self, json: Json, scope: &mut ScopeInfo) -> ErrOR<Json> {
    if let Json::Array(Lit(list)) = json {
      Ok(Json::Array(Lit(self.eval_args(list, scope)?)))
    } else if let Json::Object(Lit(mut object)) = json {
      let dec_len = sub!(object.len(), 1)?;
      for (key, val) in object.drain(..dec_len) {
        let tmp_json = self.eval_func(scope, key, val)?;
        scope.drop_json(tmp_json)?;
      }
      let (key, val) = take(&mut object[0]);
      self.eval_func(scope, key, val)
    } else {
      Ok(json)
    }
  }
  fn eval_args(
    &mut self, mut args: Vec<WithPos<Json>>, scope: &mut ScopeInfo,
  ) -> ErrOR<Vec<WithPos<Json>>> {
    for arg in &mut args {
      arg.value = self.eval(take(&mut arg.value), scope)?;
    }
    Ok(args)
  }
  fn eval_func(
    &mut self, scope: &mut ScopeInfo, key: WithPos<String>, val: WithPos<Json>,
  ) -> ErrOR<Json> {
    let WithPos { value: name, pos } = key;
    if let Some(builtin) = self.builtin.get_mut(&name) {
      let scoped = builtin.scoped;
      let skip_eval = builtin.skip_eval;
      let func = builtin.func;
      let arg_len = builtin.arg_len;
      let mut tmp = ScopeInfo::default();
      if scoped {
        self.vars_local.push(HashMap::new());
        tmp = scope.begin()?;
      }
      let args: VecDeque<WithPos<Json>> = if let Json::Array(Lit(arr)) = val.value {
        if skip_eval { arr } else { self.eval_args(arr, scope)? }
      } else {
        self.eval_args(vec![val], scope)?
      }
      .into();
      let len = args.len();
      let func_info = FuncInfo { args, len, name, pos };
      self.validate_args(&func_info, arg_len)?;
      let result = func(self, func_info, scope)?;
      if scoped {
        scope.end(tmp)?;
        self.vars_local.pop();
      }
      Ok(result)
    } else if let Ok(Json::Function(asm_func)) = self.get_var(&name, &pos) {
      scope.body.push(mn!("call", asm_func.label.to_ref()));
      if let Json::Int(_) = *asm_func.ret {
        return Ok(Json::Int(Var(scope.mov_tmp("rax")?)));
      }
      Ok(Json::Null)
    } else {
      return err!(self, val.pos, "Undefined function: `{name}`");
    }
  }
  pub(crate) fn get_var(&self, var_name: &str, pos: &Position) -> ErrOR<Json> {
    for scope in self.vars_local.iter().rev() {
      if let Some(val) = scope.get(var_name) {
        return Ok(val.clone());
      }
    }
    if let Some(val) = self.vars_global.get(var_name) {
      return Ok(val.clone());
    }
    err!(self, pos, "Undefined variables: `{var_name}`")
  }
  pub(crate) fn register(
    &mut self, name: &str, (scoped, skip_eval): (bool, bool), func: JFunc, arg_len: ArgLen,
  ) {
    self.builtin.insert(name.to_owned(), Builtin { arg_len, func, scoped, skip_eval });
  }
}
