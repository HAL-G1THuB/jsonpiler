mod arithmetic;
mod compare;
mod compound;
mod control;
mod define;
mod evaluate;
mod file;
mod gui;
mod io;
mod logic;
mod string;
mod variable;
use crate::prelude::*;
use std::{env, fs, path::Path, process::Command};
impl Jsonpiler {
  fn get_std_any(&mut self, get_std_handle: (u32, u32), std_id: u32, std_n: u32) -> [Inst; 5] {
    [
      mov_d(Rcx, std_id),
      CallApi(get_std_handle),
      CmpRIb(Rax, -1i8),
      JCc(E, self.symbols[WIN_HANDLER]),
      mov_q(Global(std_n), Rax),
    ]
  }
  pub(crate) fn register_builtin(&mut self) {
    self.arithmetic();
    self.compare();
    self.compound();
    self.control();
    self.define();
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
  fn compile(&mut self, is_jspl: bool) -> ErrOR<()> {
    let json = self.parser[0].parse(is_jspl)?;
    self.register_builtin();
    let data_at_least = self.id();
    self.data_insts.push(Byte(data_at_least, 0x00));
    let heap = self.bss(8, 8);
    self.symbols.insert(HEAP, heap);
    let flag_gui = self.bss(1, 1);
    self.symbols.insert(FLAG_GUI, flag_gui);
    let seh_handler = self.id();
    self.symbols.insert(SEH_HANDLER, seh_handler);
    let win_handler = self.id();
    self.symbols.insert(WIN_HANDLER, win_handler);
    let std_o = self.bss(8, 8);
    self.symbols.insert(STD_O, std_o);
    let std_e = self.bss(8, 8);
    self.symbols.insert(STD_E, std_e);
    let std_i = self.bss(8, 8);
    self.symbols.insert(STD_I, std_i);
    // let ignore_handler = self.id();
    // self.symbols.insert(IGNORE_HANDLER, ignore_handler);
    let ctrl_c_handler = self.id();
    self.symbols.insert(CTRL_C_HANDLER, ctrl_c_handler);
    let set_console_cp = self.import(KERNEL32, "SetConsoleCP")?;
    let set_console_output_cp = self.import(KERNEL32, "SetConsoleOutputCP")?;
    let exit_process = self.import(KERNEL32, "ExitProcess")?;
    let get_process_heap = self.import(KERNEL32, "GetProcessHeap")?;
    let get_std_handle = self.import(KERNEL32, "GetStdHandle")?;
    let set_ctrl_c_handler = self.import(KERNEL32, "SetConsoleCtrlHandler")?;
    let win_handler_insts = self.win_handler()?;
    let seh_handler_insts = self.seh_handler()?;
    let ctrl_c_handler_insts = self.ctrl_c_handler()?;
    let mut scope = Scope::default();
    let result = self.eval(json, &mut scope)?;
    let exit_insts = mov_int(Rcx, if let Int(int) = result { int } else { Lit(0) });
    self.drop_json(result, &mut scope, false)?;
    for local in take(&mut scope.local_top).into_values() {
      self.drop_json(local, &mut scope, true)?;
    }
    scope.check_free()?;
    let stack_size = scope.resolve_stack_size()?;
    let id = self.id();
    let end = self.id();
    self.data_insts.push(Seh(id, end, stack_size));
    let mut insts = vec![];
    extend!(
      insts,
      [
        Lbl(id),
        Push(Rbp),
        mov_q(Rbp, Rsp),
        SubRId(Rsp, stack_size),
        mov_d(Rcx, 65001),
        CallApiNull(set_console_cp),
        mov_d(Rcx, 65001),
        CallApiNull(set_console_output_cp),
        CallApiNull(get_process_heap),
        mov_q(Global(heap), Rax),
        LeaRM(Rcx, Global(ctrl_c_handler)),
        Clear(Rdx),
        IncR(Rdx),
        CallApiNull(set_ctrl_c_handler),
      ],
      self.get_std_any(get_std_handle, 0xFFFF_FFF6, std_i),
      self.get_std_any(get_std_handle, 0xFFFF_FFF5, std_o),
      self.get_std_any(get_std_handle, 0xFFFF_FFF4, std_e),
      // self.get_std_any(get_std_handle, 0, std_e),
      // [Clear(Rcx), IDivR(Rcx)],
      take(&mut self.startup),
      take(&mut scope.body),
      exit_insts,
      [
        CallApi(exit_process),
        Lbl(end),
        // Lbl(ignore_handler),
        // Clear(Rcx),
        // IncR(Rcx),
        // CallApi(exit_process)
      ],
      take(&mut self.insts),
      win_handler_insts,
      seh_handler_insts,
      ctrl_c_handler_insts,
    );
    let assembler = Assembler::from(take(&mut self.dlls), win_handler);
    assembler.assemble(&insts, take(&mut self.data_insts), seh_handler, &self.parser[0].file)
  }
  pub(crate) fn drop_json(&mut self, mut json: Json, scope: &mut Scope, var: bool) -> ErrOR<()> {
    if let Some(Label(Local(lifetime, offset), size)) = json.label().copied()
      && (var || lifetime == Tmp)
    {
      scope.free(offset, size);
      if size == Heap {
        let free = self.import(KERNEL32, "HeapFree")?;
        scope.heap_free(offset, (self.symbols[HEAP], free));
      }
    }
    Ok(())
  }
  pub(crate) fn eval(&mut self, json: WithPos<Json>, scope: &mut Scope) -> ErrOR<Json> {
    if let Array(Lit(array)) = json.val {
      Ok(Array(Lit(self.eval_args(array, scope)?)))
    } else if let Object(Lit(object)) = json.val {
      self.eval_object(object, scope)
    } else {
      Ok(json.val)
    }
  }
  fn eval_args(
    &mut self,
    mut args: Vec<WithPos<Json>>,
    scope: &mut Scope,
  ) -> ErrOR<Vec<WithPos<Json>>> {
    for arg in &mut args {
      *arg = arg.pos.with(self.eval(take(arg), scope)?);
    }
    Ok(args)
  }
  fn eval_func(&mut self, scope: &mut Scope, (name, jwp): KeyVal) -> ErrOR<Json> {
    if let Some(builtin) = self.builtin.get(&name.val) {
      let BuiltinFunc { scoped, skip_eval, builtin_ptr, arity } = *builtin;
      if scoped {
        scope.locals.push(HashMap::new());
      }
      let mut func = self.function(name, jwp, skip_eval, scope)?;
      validate_args(&func, arity)?;
      let result = builtin_ptr(self, &mut func, scope)?;
      self.free_all(&func, scope)?;
      if scoped {
        for local in scope.locals.pop().unwrap_or_default().into_values() {
          self.drop_json(local, scope, true)?;
        }
      }
      return Ok(result);
    }
    let mut func = self.function(name.clone(), jwp, false, scope)?;
    let Some(AsmFunc { id, params, ret }) =
      self.user_defined.get(&name.val).map(|wp| wp.val.clone())
    else {
      return err!(name.pos, UndefinedFn(name.val));
    };
    scope.update_args_count(u32::try_from(params.len())?);
    validate_args(&func, Exactly(params.len()))?;
    for param in &params {
      let arg = func.arg()?;
      if discriminant(&arg.val) != discriminant(param) {
        return Err(args_type_err(func.nth, &func.name, param.describe(), &arg));
      }
      let reg = *REGS.get(func.nth - 1).unwrap_or(&Rax);
      let mut tmp_opt = None;
      if matches!(arg.val, Str(_)) {
        let tmp = scope.alloc(0x20, 8)?;
        tmp_opt = Some(tmp);
        for (idx, tmp_reg) in REGS.iter().enumerate() {
          if *tmp_reg != reg {
            scope.push(mov_q(Local(Tmp, tmp + i32::try_from(idx * 8)?), *tmp_reg));
          }
        }
      }
      scope.extend(&self.mov_deep_json(reg, arg)?);
      if reg == Rax {
        scope.push(mov_q(Args(i32::try_from(func.nth)?), Rax));
      }
      if let Some(tmp) = tmp_opt {
        for (idx, tmp_reg) in REGS.iter().enumerate() {
          if *tmp_reg != reg {
            scope.push(mov_q(*tmp_reg, Local(Tmp, tmp + i32::try_from(idx * 8)?)));
          }
        }
        scope.free(tmp, Size(0x20));
      }
    }
    scope.push(Call(id));
    let ret_json = scope.ret_json(Rax, &name.pos.with(ret))?;
    self.free_all(&func, scope)?;
    Ok(ret_json)
  }
  fn eval_object(&mut self, object: Vec<KeyVal>, scope: &mut Scope) -> ErrOR<Json> {
    let mut tmp_json = Null;
    for key_val in object {
      self.drop_json(tmp_json, scope, false)?;
      tmp_json = self.eval_func(scope, key_val)?;
    }
    Ok(tmp_json)
  }
  fn eval_object_with_drop(&mut self, object: Vec<KeyVal>, scope: &mut Scope) -> ErrOR<()> {
    for key_val in object {
      let tmp_json = self.eval_func(scope, key_val)?;
      self.drop_json(tmp_json, scope, false)?;
    }
    Ok(())
  }
  pub(crate) fn free_all(&mut self, func: &Function, scope: &mut Scope) -> ErrOR<()> {
    for (start, size) in &func.free_vec {
      if *size == Heap {
        let free = self.import(KERNEL32, "HeapFree")?;
        scope.heap_free(*start, (self.symbols[HEAP], free));
      }
      scope.free(*start, *size);
    }
    Ok(())
  }
  pub(crate) fn function(
    &mut self,
    WithPos { val: name, pos }: WithPos<String>,
    jwp: WithPos<Json>,
    skip_eval: bool,
    scope: &mut Scope,
  ) -> ErrOR<Function> {
    let args_vec = if let Array(Lit(arr)) = jwp.val { arr } else { vec![jwp] };
    let mut args = if skip_eval { args_vec } else { self.eval_args(args_vec, scope)? };
    let mut func =
      Function { len: args.len(), name, pos, args: vec![].into_iter(), free_vec: vec![], nth: 0 };
    if !skip_eval {
      for label in args.iter_mut().filter_map(|arg| arg.val.label().copied()) {
        func.push_free_tmp(label);
      }
    }
    func.args = args.into_iter();
    Ok(func)
  }
  pub(crate) fn register(
    &mut self,
    name: &str,
    (scoped, skip_eval): (bool, bool),
    builtin_ptr: BuiltinPtr,
    arity: Arity,
  ) {
    self.builtin.insert(name.into(), BuiltinFunc { arity, builtin_ptr, scoped, skip_eval });
  }
  #[inline]
  #[expect(clippy::print_stdout)]
  pub fn run(&mut self) -> Result<i32, String> {
    if !is_x86_feature_detected!("sse2") {
      return Err("Error: SSE2 not supported on this CPU. CPU may not be x64.".into());
    }
    let io_err = |err| format!("{COMPILATION_ERROR}{}{ERR_END}", IncludeIOError(err));
    let mut args = env::args();
    let program_name = args.next().unwrap_or("jsonpiler.exe".into());
    let Some(mut file) = args.next() else {
      help_message(&program_name);
      return Ok(0);
    };
    if file == "help" {
      help_message(&program_name);
      return Ok(0);
    }
    if file == "version" {
      println!("jsonpiler version {}", version!());
      return Ok(0);
    }
    let mut build_only = false;
    if file == "build" {
      build_only = true;
      let Some(next_file) = args.next() else {
        help_message(&program_name);
        return Ok(0);
      };
      file = next_file;
    }
    if file == "release" {
      self.release = true;
      let Some(next_file) = args.next() else {
        help_message(&program_name);
        return Ok(0);
      };
      file = next_file;
    }
    if fs::metadata(&file).map_err(io_err)?.len() > 1 << 30u8 {
      return Err(format!("{COMPILATION_ERROR}{TooLargeFile}{ERR_END}"));
    }
    let source = fs::read(&file).map_err(io_err)?;
    let exe_path = Path::new(&file).with_extension("exe");
    let exe = exe_path.to_string_lossy().to_string();
    let is_jspl = match Path::new(&file).extension().map(|ext| ext.to_string_lossy()) {
      Some(jspl) if jspl == "jspl" => true,
      Some(json) if json == "json" => false,
      _ => return Err(format!("{COMPILATION_ERROR}{UnsupportedFile}{ERR_END}")),
    };
    self.parser.push(Parser::from(source, 0, full_path(&file).map_err(io_err)?));
    self.files.push(HashMap::new());
    self.compile(is_jspl).map_err(|err| match err {
      Compilation(kind, pos) => {
        let (file_str, l_c, code, carets) = self.err_info(pos);
        format!("{COMPILATION_ERROR}{kind}{ERR_SEPARATE}{file_str}{l_c}{ERR_SEPARATE}{code}| {carets}{ERR_END}")
      }
      Internal(kind) => format!("{INTERNAL_ERROR}{kind}{ERR_END}\n{REPORT_MSG}{}`", kind.err_code()),
      IO(err_str) => format!("{IO_ERROR}{err_str}{ERR_END}"),
    })?;
    if build_only {
      return Ok(0);
    }
    Ok(
      Command::new(env::current_dir().map_err(io_err)?.join(exe))
        .args(args)
        .status()
        .map_err(io_err)?
        .code()
        .unwrap_or(0),
    )
  }
}
#[expect(clippy::print_stdout)]
fn help_message(program_name: &str) {
  println!("Usage: {program_name} <input.jspl | input.json> [args for .exe]\n{COMMAND}");
}
