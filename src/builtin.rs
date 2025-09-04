mod arithmetic;
mod compare;
mod control;
mod evaluate;
mod file;
mod gui;
mod internal;
mod logic;
mod output;
mod string;
mod variable;
use crate::{
  Arity::{self, Exactly},
  AsmFunc, Assembler,
  Bind::{Lit, Var},
  Builtin, BuiltinPtr,
  ConditionCode::*,
  ErrOR, FuncInfo,
  Inst::{self, *},
  Json, Jsonpiler, Label,
  Operand::Args,
  Position,
  Register::*,
  ScopeInfo,
  VarKind::*,
  WithPos, err,
  utility::{mov_b, mov_d, mov_q},
};
use core::mem::{discriminant, take};
use std::{fs::File, io::Write as _};
impl Jsonpiler {
  pub(crate) fn register_all(&mut self) {
    self.arithmetic();
    self.compare();
    self.control();
    self.evaluate();
    self.file();
    self.gui();
    self.logic();
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
      let Builtin { scoped, skip_eval, ptr: builtin_ptr, arg_len } = *builtin;
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
      let mut free_list = vec![];
      if !skip_eval {
        for arg in &args_vec {
          if let Some(Label { mem: Tmp { offset, .. }, size }) = arg.value.get_label() {
            free_list.push((offset, size));
          }
        }
      }
      let len = args_vec.len();
      let args = args_vec.into_iter();
      let mut func = FuncInfo { args, free_list, len, name, pos, nth: 0 };
      self.parser[func.pos.file].validate_args(&func, arg_len)?;
      let result = builtin_ptr(self, &mut func, scope)?;
      for label in func.free_list {
        scope.free(label.0, label.1)?;
      }
      if let Some(tmp) = maybe_tmp {
        scope.end(tmp)?;
      }
      Ok(result)
    } else {
      let args_vec = self
        .eval_args(if let Json::Array(Lit(arr)) = val.value { arr } else { vec![val] }, scope)?;
      let Some(AsmFunc { id, params, ret, .. }) = self.user_defined.get(&name).cloned() else {
        return err!(self, key.pos, "Undefined function: {name}");
      };
      if params.len() >= 16 {
        return err!(self, key.pos, "Too many arguments: Up to 16 arguments are allowed.");
      }
      scope.update_stack_args(i32::try_from(params.len().saturating_sub(4))?);
      let mut free_list = vec![];
      let len = args_vec.len();
      for arg in &args_vec {
        if let Some(Label { mem: Tmp { offset, .. }, size }) = arg.value.get_label() {
          free_list.push((offset, size));
        }
      }
      let mut func = FuncInfo { len, name, pos, args: args_vec.into_iter(), free_list, nth: 0 };
      self.parser[func.pos.file].validate_args(&func, Exactly(params.len()))?;
      for param in &params {
        let jwp = func.arg()?;
        if discriminant(&jwp.value) != discriminant(param) {
          return Err(
            self.parser[jwp.pos.file]
              .args_type_error(func.nth, &func.name, &param.type_name(), &jwp)
              .into(),
          );
        }
        self.mov_to_args(&jwp, func.nth - 1, scope)?;
      }
      scope.push(Call(id));
      for label in func.free_list {
        scope.free(label.0, label.1)?;
      }
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
    let (key, val) = object
      .pop()
      .ok_or_else(|| self.parser[pos.file].fmt_err("Empty object is not allowed", pos))?;
    self.eval_func(scope, key, val)
  }
  #[expect(clippy::cast_sign_loss)]
  fn get_std_any(&mut self, get_std_handle: (u32, u32), std_any: i32, id: u32) -> [Inst; 5] {
    [
      mov_d(Rcx, std_any as u32),
      CallApi(get_std_handle),
      CmpRIb(Rax, -1i8),
      Jcc(E, self.sym_table["WIN_HANDLER"]),
      mov_q(Global { id, disp: 0i32 }, Rax),
    ]
  }
  fn handler(&mut self) -> ErrOR<[Inst; 25]> {
    let exit_process = self.import(Jsonpiler::KERNEL32, "ExitProcess")?;
    let format_message = self.import(Jsonpiler::KERNEL32, "FormatMessageW")?;
    let get_last_error = self.import(Jsonpiler::KERNEL32, "GetLastError")?;
    let message_box = self.import(Jsonpiler::USER32, "MessageBoxW")?;
    let local_free = self.import(Jsonpiler::KERNEL32, "LocalFree")?;
    let win_handler_exit = self.gen_id();
    let msg = Global { id: self.sym_table["TMP"], disp: 0i32 };
    Ok([
      Lbl(self.sym_table["WIN_HANDLER"]),
      CallApi(get_last_error),
      mov_q(Rdi, Rax),
      mov_d(Rcx, 0x1300),
      Clear(Rdx),
      mov_q(R8, Rdi),
      Clear(R9),
      LeaRM(Rax, msg),
      mov_q(Args(0x20), Rax),
      Clear(Rax),
      mov_q(Args(0x28), Rax),
      mov_q(Args(0x30), Rax),
      CallApi(format_message),
      TestRdRd(Rax, Rax),
      Jcc(E, win_handler_exit),
      Clear(Rcx),
      mov_q(Rdx, msg),
      Clear(R8),
      mov_d(R9, 0x10),
      CallApi(message_box),
      Lbl(win_handler_exit),
      mov_q(Rcx, msg),
      CallApi(local_free),
      mov_q(Rcx, Rdi),
      CallApi(exit_process),
    ])
  }
  fn mov_to_args(&mut self, jwp: &WithPos<Json>, idx: usize, scope: &mut ScopeInfo) -> ErrOR<()> {
    let reg = *Jsonpiler::REGS.get(idx).unwrap_or(&Rax);
    match &jwp.value {
      Json::String(Lit(l_str)) => {
        let id = self.global_str(l_str.clone()).0;
        scope.push(LeaRM(reg, Global { id, disp: 0i32 }));
      }
      Json::String(Var(label))
      | Json::Float(Var(label))
      | Json::Bool(Var(label))
      | Json::Int(Var(label)) => scope.push(mov_q(reg, label.mem)),
      Json::Null => scope.push(Clear(reg)),
      #[expect(clippy::cast_sign_loss)]
      Json::Int(Lit(l_int)) => scope.push(mov_q(reg, *l_int as u64)),
      Json::Bool(Lit(l_bool)) => scope.push(mov_b(reg, if *l_bool { 0xFF } else { 0 })),
      Json::Float(Lit(l_float)) => scope.push(mov_q(reg, l_float.to_bits())),
      Json::Array(_) | Json::Object(_) => {
        return err!(
          self,
          jwp.pos,
          "This type cannot be accepted as an argument of an user-defined function."
        );
      }
    }
    if reg == Rax {
      scope.push(mov_q(Args(idx * 8), Rax));
    }
    Ok(())
  }
  pub(crate) fn register(
    &mut self, name: &str, (scoped, skip_eval): (bool, bool), builtin_ptr: BuiltinPtr,
    arg_len: Arity,
  ) {
    self.builtin.insert(name.to_owned(), Builtin { arg_len, ptr: builtin_ptr, scoped, skip_eval });
  }
  #[inline]
  pub fn run(&mut self, exe: &str, is_jspl: bool) -> ErrOR<()> {
    let json = self.parser[0].parse(is_jspl)?;
    /*
    let msg = self.global_str_raw_ptr(include_str!("txt/SEH_HANDLER_MSG.txt").to_owned());
    self.sym_table.insert("SEH_HANDLER_MSG", msg);
    */
    self.global_bool(false);
    self.global_str(String::new());
    let std_o = self.get_bss_id(8, 8);
    self.sym_table.insert("STDO", std_o);
    let std_e = self.get_bss_id(8, 8);
    self.sym_table.insert("STDE", std_e);
    let std_i = self.get_bss_id(8, 8);
    self.sym_table.insert("STDI", std_i);
    let heap = self.get_bss_id(8, 8);
    self.sym_table.insert("HEAP", heap);
    let tmp = self.get_bss_id(8, 8);
    self.sym_table.insert("TMP", tmp);
    let flag_gui = self.get_bss_id(1, 1);
    self.sym_table.insert("FLAG_GUI", flag_gui);
    let win_handler = self.gen_id();
    self.sym_table.insert("WIN_HANDLER", win_handler);
    self.register_all();
    let mut scope = ScopeInfo::new();
    // handler
    scope.update_stack_args(3);
    let set_console_cp = self.import(Jsonpiler::KERNEL32, "SetConsoleCP")?;
    let set_console_output_cp = self.import(Jsonpiler::KERNEL32, "SetConsoleOutputCP")?;
    let exit_process = self.import(Jsonpiler::KERNEL32, "ExitProcess")?;
    let get_process_heap = self.import(Jsonpiler::KERNEL32, "GetProcessHeap")?;
    let get_std_handle = self.import(Jsonpiler::KERNEL32, "GetStdHandle")?;
    let result = self.eval(json, &mut scope)?;
    let size = scope.resolve_stack_size()?;
    let mut insts = vec![Push(Rbp), mov_q(Rbp, Rsp), SubRId(Rsp, size)];
    insts.push(mov_d(Rcx, 65001));
    insts.extend_from_slice(&self.call_api_check_null(set_console_cp));
    insts.push(mov_d(Rcx, 65001));
    insts.extend_from_slice(&self.call_api_check_null(set_console_output_cp));
    insts.extend_from_slice(&self.get_std_any(get_std_handle, -10, std_i));
    insts.extend_from_slice(&self.get_std_any(get_std_handle, -11, std_o));
    insts.extend_from_slice(&self.get_std_any(get_std_handle, -12, std_e));
    insts.extend_from_slice(&self.call_api_check_null(get_process_heap));
    insts.push(mov_q(Global { id: heap, disp: 0i32 }, Rax));
    insts.extend_from_slice(&take(&mut self.startup));
    #[expect(clippy::cast_sign_loss)]
    if let Json::Int(int) = result {
      scope.push(match int {
        Lit(l_int) => mov_q(Rcx, l_int as u64),
        Var(label) => mov_q(Rcx, label.mem),
      });
    } else {
      scope.push(Clear(Rcx));
    }
    scope.push(CallApi(exit_process));
    insts.extend(scope.take_code());
    let handler = &self.handler()?;
    self.insts.extend_from_slice(handler);
    let mut file = File::create(exe)?;
    let assembler = Assembler::new(take(&mut self.import_table));
    file.write_all(
      &assembler
        .assemble_and_link(insts.iter().chain(self.insts.iter()), take(&mut self.data_insts))?,
    )?;
    Ok(())
  }
}
