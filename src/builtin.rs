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
  Builtin, ErrOR, FuncInfo,
  Inst::{self, *},
  JFunc, Json, Jsonpiler,
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
      let mut func_info = FuncInfo { args, free_list, len, name, pos };
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
      let len = args_vec.len();
      let func_info = &FuncInfo { len, name, pos, args: vec![].into_iter(), free_list: vec![] };
      self.parser.validate_args(func_info, Exactly(params.len()))?;
      for (idx, jwp) in args_vec.into_iter().enumerate() {
        if discriminant(&jwp.value) != discriminant(&params[idx]) {
          return Err(
            self.parser.type_err(idx + 1, &func_info.name, &params[idx].type_name(), &jwp).into(),
          );
        }
        self.mov_to_args(&jwp, idx, scope)?;
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
      Jze(win_handler_exit),
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
    {
      match &jwp.value {
        Json::String(string) => match string {
          Lit(l_str) => {
            if let Some(&reg) = Jsonpiler::REGS.get(idx) {
              scope.push(LeaRM(reg, Global { id: self.global_str(l_str.to_owned()) }));
            } else {
              scope.push(LeaRM(Rax, Global { id: self.global_str(l_str.to_owned()) }));
              scope.push(MovQQ(Args((idx - 4) * 8), Rq(Rax)));
            }
          }
          Var(str_label) => {
            if let Some(&reg) = Jsonpiler::REGS.get(idx) {
              scope.push(LeaRM(reg, str_label.kind));
            } else {
              scope.push(LeaRM(Rax, str_label.kind));
              scope.push(MovQQ(Args((idx - 4) * 8), Rq(Rax)));
            }
          }
        },
        Json::Float(Var(label)) | Json::Bool(Var(label)) | Json::Int(Var(label)) => {
          if let Some(&reg) = Jsonpiler::REGS.get(idx) {
            scope.push(MovQQ(Rq(reg), Mq(label.kind)));
          } else {
            scope.push(MovQQ(Rq(Rax), Mq(label.kind)));
            scope.push(MovQQ(Args((idx - 4) * 8), Rq(Rax)));
          }
        }
        Json::Null => {
          if let Some(&reg) = Jsonpiler::REGS.get(idx) {
            scope.push(Clear(reg));
          } else {
            scope.push(Clear(Rax));
            scope.push(MovQQ(Args((idx - 4) * 8), Rq(Rax)));
          }
        }
        #[expect(clippy::cast_sign_loss)]
        Json::Int(Lit(l_int)) => {
          if let Some(&reg) = Jsonpiler::REGS.get(idx) {
            scope.push(MovQQ(Rq(reg), Iq(*l_int as u64)));
          } else {
            scope.push(MovQQ(Rq(Rax), Iq(*l_int as u64)));
            scope.push(MovQQ(Args((idx - 4) * 8), Rq(Rax)));
          }
        }
        Json::Bool(Lit(l_bool)) => {
          if let Some(&reg) = Jsonpiler::REGS.get(idx) {
            scope.push(MovRbIb(reg, if *l_bool { 0xFF } else { 0 }));
          } else {
            scope.push(MovRbIb(Rax, if *l_bool { 0xFF } else { 0 }));
            scope.push(MovQQ(Args((idx - 4) * 8), Rq(Rax)));
          }
        }
        Json::Float(Lit(l_float)) => {
          if let Some(&reg) = Jsonpiler::REGS.get(idx) {
            scope.push(MovQQ(Rq(reg), Iq(l_float.to_bits())));
          } else {
            scope.push(MovQQ(Rq(Rax), Iq(l_float.to_bits())));
            scope.push(MovQQ(Args((idx - 4) * 8), Rq(Rax)));
          }
        }
        Json::Array(_) | Json::Object(_) => {
          return err!(
            self,
            jwp.pos,
            "This type cannot be accepted as an argument of an user-defined function."
          );
        }
      }
    }
    Ok(())
  }
  pub(crate) fn register(
    &mut self, name: &str, (scoped, skip_eval): (bool, bool), func: JFunc, arg_len: Arity,
  ) {
    self.builtin.insert(name.to_owned(), Builtin { arg_len, func, scoped, skip_eval });
  }
  #[inline]
  #[expect(clippy::cast_sign_loss)]
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
    let mut insts = vec![
      MovQQ(Rq(Rbp), Rq(Rsp)),
      SubRId(Rsp, size),
      MovRId(Rcx, 65001),
      CallApi(set_console_cp),
      TestRR(Rax, Rax),
      Jze(win_handler),
      MovRId(Rcx, 65001),
      CallApi(set_console_output_cp),
      TestRR(Rax, Rax),
      Jze(win_handler),
      MovRId(Rcx, -10i32 as u32),
      CallApi(get_std_handle),
      CmpRIb(Rax, -1i8),
      Jze(win_handler),
      MovQQ(Mq(Global { id: std_i }), Rq(Rax)),
      MovRId(Rcx, -11i32 as u32),
      CallApi(get_std_handle),
      CmpRIb(Rax, -1i8),
      Jze(win_handler),
      MovQQ(Mq(Global { id: std_o }), Rq(Rax)),
      MovRId(Rcx, -12i32 as u32),
      CallApi(get_std_handle),
      CmpRIb(Rax, -1i8),
      Jze(win_handler),
      MovQQ(Mq(Global { id: std_e }), Rq(Rax)),
      CallApi(get_process_heap),
      TestRR(Rax, Rax),
      Jze(win_handler),
      MovQQ(Mq(Global { id: heap }), Rq(Rax)),
    ];
    #[expect(clippy::cast_sign_loss)]
    if let Json::Int(int) = result {
      match int {
        Lit(l_int) => scope.push(MovQQ(Rq(Rcx), Iq(l_int as u64))),
        Var(label) => scope.push(MovQQ(Rq(Rcx), Mq(label.kind))),
      }
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
