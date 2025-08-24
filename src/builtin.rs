mod arithmetic;
mod compare;
mod control;
mod evaluate;
mod logical;
mod output;
mod string;
mod variable;
use crate::{
  Arity,
  Arity::Exactly,
  AsmFunc, Assembler,
  Bind::{Lit, Var},
  Builtin, BuiltinPtr,
  ConditionCode::*,
  ErrOR, FuncInfo,
  Inst::{self, *},
  Json, Jsonpiler,
  OpQ::{Args, Iq, Mq, Rq},
  Position,
  Reg::*,
  ScopeInfo,
  VarKind::Global,
  WithPos, err,
};
use core::mem::{discriminant, take};
use std::{fs::File, io::Write as _};
impl Jsonpiler {
  pub(crate) fn register_all(&mut self) {
    self.arithmetic();
    self.compare();
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
      let mut func_info = FuncInfo { args, free_list, len, name, pos, nth: 0 };
      self.parser.validate_args(&func_info, arg_len)?;
      let result = func(self, &mut func_info, scope)?;
      for label in func_info.free_list {
        scope.free(label.0, label.1)?;
      }
      if let Some(tmp) = maybe_tmp {
        scope.end(tmp)?;
      }
      Ok(result)
    } else {
      let args_vec = self
        .eval_args(if let Json::Array(Lit(arr)) = val.value { arr } else { vec![val] }, scope)?;
      let Some(AsmFunc { id, params, ret }) = self.user_defined.get(&name).cloned() else {
        return err!(self, key.pos, "Undefined function");
      };
      if params.len() >= 16 {
        return err!(self, key.pos, "Too many arguments: Up to 16 arguments are allowed.");
      }
      let len = args_vec.len();
      let mut func =
        FuncInfo { len, name, pos, args: args_vec.into_iter(), free_list: vec![], nth: 0 };
      self.parser.validate_args(&func, Exactly(params.len()))?;
      for param in &params {
        let jwp = func.arg()?;
        if discriminant(&jwp.value) != discriminant(param) {
          return Err(self.parser.type_err(func.nth, &func.name, &param.type_name(), &jwp).into());
        }
        self.mov_to_args(&jwp, func.nth - 1, scope)?;
      }
      scope.push(Call(id));
      match ret {
        Json::Int(_) => Ok(Json::Int(Var(scope.mov_tmp(Rax)?))),
        Json::Bool(_) => scope.mov_tmp_bool(Rax),
        Json::Float(_) => Ok(Json::Float(Var(scope.mov_tmp(Rax)?))),
        Json::String(_) => Ok(Json::String(Var(scope.mov_tmp(Rax)?))),
        Json::Null => Ok(Json::Null),
        Json::Array(_) | Json::Object(_) => {
          err!(self, key.pos, "Unsupported return type: `{}`", ret.type_name())
        }
      }
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
  #[expect(clippy::cast_sign_loss)]
  fn get_std_any(&mut self, get_std_handle: (usize, usize), std_any: i32, id: usize) -> [Inst; 5] {
    [
      MovRId(Rcx, std_any as u32),
      CallApi(get_std_handle),
      CmpRIb(Rax, -1i8),
      Jcc(E, self.sym_table["WIN_HANDLER"]),
      MovQQ(Mq(Global { id }), Rq(Rax)),
    ]
  }
  fn handler(&mut self) -> [Inst; 25] {
    let exit_process = self.import(Jsonpiler::KERNEL32, "ExitProcess", 0x167);
    let format_message = self.import(Jsonpiler::KERNEL32, "FormatMessageW", 0x1B0);
    let get_last_error = self.import(Jsonpiler::KERNEL32, "GetLastError", 0x26A);
    let message_box = self.import(Jsonpiler::USER32, "MessageBoxW", 0x28c);
    let local_free = self.import(Jsonpiler::KERNEL32, "LocalFree", 0x3D8);
    let win_handler_exit = self.gen_id();
    [
      Lbl(self.sym_table["WIN_HANDLER"]),
      CallApi(get_last_error),
      MovQQ(Rq(Rdi), Rq(Rax)),
      MovRId(Rcx, 0x1300),
      Clear(Rdx),
      MovQQ(Rq(R8), Rq(Rdi)),
      Clear(R9),
      LeaRM(Rax, Global { id: self.sym_table["WIN_HANDLER_MSG"] }),
      MovQQ(Args(0x20), Rq(Rax)),
      MovQQ(Rq(Rax), Iq(0)),
      MovQQ(Args(0x28), Rq(Rax)),
      MovQQ(Args(0x30), Rq(Rax)),
      CallApi(format_message),
      TestRdRd(Rax, Rax),
      Jcc(E, win_handler_exit),
      Clear(Rcx),
      MovQQ(Rq(Rdx), Mq(Global { id: self.sym_table["WIN_HANDLER_MSG"] })),
      Clear(R8),
      MovRId(R9, 0x10),
      CallApi(message_box),
      Lbl(win_handler_exit),
      MovQQ(Rq(Rcx), Mq(Global { id: self.sym_table["WIN_HANDLER_MSG"] })),
      CallApi(local_free),
      MovQQ(Rq(Rcx), Rq(Rdi)),
      CallApi(exit_process),
    ]
  }
  fn mov_to_args(&mut self, jwp: &WithPos<Json>, idx: usize, scope: &mut ScopeInfo) -> ErrOR<()> {
    let reg = *Jsonpiler::REGS.get(idx).unwrap_or(&Rax);
    match &jwp.value {
      Json::String(string) => scope.push(LeaRM(
        reg,
        match string {
          Lit(l_str) => Global { id: self.global_str(l_str.to_owned()) },
          Var(str_label) => str_label.kind,
        },
      )),
      Json::Float(Var(label)) | Json::Bool(Var(label)) | Json::Int(Var(label)) => {
        scope.push(MovQQ(Rq(reg), Mq(label.kind)));
      }
      Json::Null => scope.push(Clear(reg)),
      #[expect(clippy::cast_sign_loss)]
      Json::Int(Lit(l_int)) => scope.push(MovQQ(Rq(reg), Iq(*l_int as u64))),
      Json::Bool(Lit(l_bool)) => scope.push(MovRbIb(reg, if *l_bool { 0xFF } else { 0 })),
      Json::Float(Lit(l_float)) => scope.push(MovQQ(Rq(reg), Iq(l_float.to_bits()))),
      Json::Array(_) | Json::Object(_) => {
        return err!(
          self,
          jwp.pos,
          "This type cannot be accepted as an argument of an user-defined function."
        );
      }
    }
    if reg == Rax {
      scope.push(MovQQ(Args(idx * 8), Rq(Rax)));
    }
    Ok(())
  }
  pub(crate) fn register(
    &mut self, name: &str, (scoped, skip_eval): (bool, bool), func: BuiltinPtr, arg_len: Arity,
  ) {
    self.builtin.insert(name.to_owned(), Builtin { arg_len, func, scoped, skip_eval });
  }
  #[inline]
  pub fn run(&mut self, exe: &str, is_jspl: bool) -> ErrOR<()> {
    let json = self.parser.parse(is_jspl)?;
    /*
    let msg = self.global_str(include_str!("txt/SEH_HANDLER_MSG.txt").to_owned());
    self.sym_table.insert("SEH_HANDLER_MSG", msg);
    */
    self.global_num(0);
    let std_o = self.get_bss_id(8);
    self.sym_table.insert("STDO", std_o);
    let std_e = self.get_bss_id(8);
    self.sym_table.insert("STDE", std_e);
    let std_i = self.get_bss_id(8);
    self.sym_table.insert("STDI", std_i);
    let heap = self.get_bss_id(8);
    self.sym_table.insert("HEAP", heap);
    let win = self.get_bss_id(8);
    self.sym_table.insert("WIN_HANDLER_MSG", win);
    let win_handler = self.gen_id();
    self.sym_table.insert("WIN_HANDLER", win_handler);
    self.register_all();
    let mut scope = ScopeInfo::new();
    // handler
    scope.update_stack_args(3);
    let set_console_cp = self.import(Jsonpiler::KERNEL32, "SetConsoleCP", 0x4FB);
    let set_console_output_cp = self.import(Jsonpiler::KERNEL32, "SetConsoleOutputCP", 0x511);
    let get_process_heap = self.import(Jsonpiler::KERNEL32, "GetProcessHeap", 0x2BE);
    let get_std_handle = self.import(Jsonpiler::KERNEL32, "GetStdHandle", 0x2DC);
    let result = self.eval(json, &mut scope)?;
    let size = scope.resolve_stack_size(8)?;
    let mut insts = vec![MovQQ(Rq(Rbp), Rq(Rsp)), SubRId(Rsp, size)];
    insts.push(MovRId(Rcx, 65001));
    insts.extend_from_slice(&self.call_api_check_null(set_console_cp));
    insts.push(MovRId(Rcx, 65001));
    insts.extend_from_slice(&self.call_api_check_null(set_console_output_cp));
    insts.extend_from_slice(&self.get_std_any(get_std_handle, -10, std_i));
    insts.extend_from_slice(&self.get_std_any(get_std_handle, -11, std_o));
    insts.extend_from_slice(&self.get_std_any(get_std_handle, -12, std_e));
    insts.extend_from_slice(&self.call_api_check_null(get_process_heap));
    insts.push(MovQQ(Mq(Global { id: heap }), Rq(Rax)));
    #[expect(clippy::cast_sign_loss)]
    if let Json::Int(int) = result {
      scope.push(MovQQ(
        Rq(Rcx),
        match int {
          Lit(l_int) => Iq(l_int as u64),
          Var(label) => Mq(label.kind),
        },
      ));
    } else {
      scope.push(Clear(Rcx));
    }
    let exit_process = self.import(Jsonpiler::KERNEL32, "ExitProcess", 0x167);
    scope.push(CallApi(exit_process));
    insts.extend(scope.take_code());
    let handler = &self.handler();
    self.insts.extend_from_slice(handler);
    let mut file = File::create(exe)?;
    let assembler = Assembler::new(take(&mut self.import_table));
    file.write_all(&assembler.assemble_and_link(insts.iter().chain(self.insts.iter()))?)?;
    Ok(())
  }
}
