mod arithmetic;
mod control;
mod evaluate;
mod logical;
mod output;
mod string;
mod variable;
use super::{
  Arity,
  Arity::Exactly,
  AsmFunc,
  Bind::{Lit, Var},
  Builtin, ErrOR, FuncInfo, JFunc, Json, Jsonpiler, Position, ScopeInfo, WithPos, err, mn,
  mn_write,
  utility::{get_argument_mem, imp_call},
};
use core::mem::{discriminant, take};
use std::io::Write as _;
impl Jsonpiler {
  pub(crate) fn register_all(&mut self) {
    self.arithmetic();
    self.control();
    self.evaluate();
    self.logical();
    self.output();
    self.string();
    self.variable();
  }
}
impl Jsonpiler {
  pub(crate) fn eval(&mut self, json: WithPos<Json>, scope: &mut ScopeInfo) -> ErrOR<Json> {
    if let Json::Array(Lit(list)) = json.value {
      Ok(Json::Array(Lit(self.eval_args(list, scope)?)))
    } else if let Json::Object(Lit(object)) = json.value {
      Ok(self.eval_object(object, json.pos, scope)?)
    } else {
      Ok(json.value)
    }
  }
  fn eval_args(
    &mut self, mut args: Vec<WithPos<Json>>, scope: &mut ScopeInfo,
  ) -> ErrOR<Vec<WithPos<Json>>> {
    for arg in &mut args {
      let pos = arg.pos;
      arg.value = self.eval(take(arg), scope)?;
      arg.pos = pos;
    }
    Ok(args)
  }
  fn eval_func(
    &mut self, scope: &mut ScopeInfo, key: WithPos<String>, val: WithPos<Json>,
  ) -> ErrOR<Json> {
    let WithPos { value: name, pos } = key;
    if let Some(builtin) = self.builtin.get(&name) {
      let Builtin { scoped, skip_eval, func, arg_len } = *builtin;
      let mut maybe_tmp = None;
      if scoped {
        maybe_tmp = Some(scope.begin()?);
      }
      let args_vec = if let Json::Array(Lit(arr)) = val.value {
        if skip_eval { arr } else { self.eval_args(arr, scope)? }
      } else if skip_eval {
        vec![val]
      } else {
        self.eval_args(vec![val], scope)?
      };
      let len = args_vec.len();
      let args = args_vec.into_iter();
      let free_list = vec![];
      let mut func_info = FuncInfo { args, free_list, len, name, pos };
      self.parser.validate_args(&func_info, arg_len)?;
      let result = func(self, &mut func_info, scope)?;
      for label in func_info.free_list {
        scope.free(label)?;
      }
      if let Some(tmp) = maybe_tmp {
        scope.end(tmp)?;
      }
      Ok(result)
    } else if let Some(Json::Function(AsmFunc { label, params, ret })) = self.get_var(&name, scope)
    {
      let args_vec = self
        .eval_args(if let Json::Array(Lit(arr)) = val.value { arr } else { vec![val] }, scope)?;
      let len = args_vec.len();
      let func_info = &FuncInfo { len, name, pos, args: vec![].into_iter(), free_list: vec![] };
      self.parser.validate_args(func_info, Exactly(params.len()))?;
      for (nth, jwp) in args_vec.into_iter().enumerate() {
        if discriminant(&jwp.value) != discriminant(&params[nth]) {
          return Err(
            self.parser.type_err(nth, &func_info.name, &params[nth].type_name(), &jwp).into(),
          );
        }
        scope.body.push(mn!("mov", get_argument_mem(nth, 8)?, self.get_argument(&jwp)?));
      }
      scope.body.push(mn!("call", label.to_ref()));
      match *ret {
        Json::Int(_) => Ok(Json::Int(Var(scope.mov_tmp("rax")?))),
        Json::Bool(_) => scope.mov_tmp_bool("al"),
        Json::Float(_) => Ok(Json::Float(Var(scope.mov_tmp("rax")?))),
        Json::String(_) => Ok(Json::String(Var(scope.mov_tmp("rax")?))),
        Json::Null => Ok(Json::Null),
        Json::Array(_) | Json::Function(_) | Json::Object(_) => {
          err!(self, key.pos, "Unsupported return type: `{}`", ret.type_name())
        }
      }
    } else {
      err!(self, key.pos, "Undefined function")
    }
  }
  fn eval_object(
    &mut self, mut object: Vec<(WithPos<String>, WithPos<Json>)>, pos: Position,
    scope: &mut ScopeInfo,
  ) -> ErrOR<Json> {
    for (key, val) in object.drain(..object.len().saturating_sub(1)) {
      let tmp_json = self.eval_func(scope, key, val)?;
      scope.drop_json(tmp_json)?;
    }
    let (key, val) =
      object.pop().ok_or_else(|| self.parser.fmt_err("Empty object is not allowed", pos))?;
    self.eval_func(scope, key, val)
  }
  pub(crate) fn register(
    &mut self, name: &str, (scoped, skip_eval): (bool, bool), func: JFunc, arg_len: Arity,
  ) {
    self.builtin.insert(name.to_owned(), Builtin { arg_len, func, scoped, skip_eval });
  }
  #[inline]
  pub fn run(&mut self) -> ErrOR<()> {
    let json = self.parser.parse()?;
    mn_write!(self.data, ".intel_syntax", "noprefix");
    let msg = include_str!("txt/SEH_HANDLER_MSG.txt");
    self.data.write_all(format!(include_str!("asm/once/data.s"), msg = msg).as_bytes())?;
    self.register_all();
    let mut scope = ScopeInfo::new();
    let result = self.eval(json, &mut scope)?;
    self.data.write_all(include_bytes!("asm/once/bss.s"))?;
    for (id, size) in &self.bss {
      writeln!(self.data, "\t.lcomm\t.L{id},\t{size}")?;
    }
    self.data.write_all(include_bytes!("asm/once/main.s"))?;
    write!(
      self.data,
      include_str!("asm/common/prologue.s"),
      size = format!("{:#X}", scope.calc_alloc(8)?)
    )?;
    self.data.write_all(include_bytes!("asm/once/startup.s"))?;
    for body in &scope.body {
      self.data.write_all(body.as_bytes())?;
    }
    if let Json::Int(int) = result {
      mn_write!(
        self.data,
        "mov",
        "rcx",
        match int {
          Lit(l_int) => l_int.to_string(),
          Var(label) => format!("{label}"),
        }
      );
    } else {
      mn_write!(self.data, "xor", "ecx", "ecx");
    }
    self.data.write_all(imp_call("ExitProcess").as_bytes())?;
    mn_write!(self.data, ".seh_endproc");
    for text in &self.text {
      self.data.write_all(text.as_bytes())?;
    }
    self.data.write_all(include_bytes!("asm/once/handler.s"))?;
    self.data.flush()?;
    Ok(())
  }
}
