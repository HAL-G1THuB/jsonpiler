mod arithmetic;
mod compare;
mod compound;
mod control;
mod define;
mod evaluate;
mod gui;
mod intrinsic;
mod io;
mod logic;
mod module;
mod string;
mod variable;
use crate::prelude::*;
use std::{env, io::Error, process::Command};
macro_rules! next_file {
  ($args:ident, $program_name:ident) => {{
    let Some(next_file) = $args.next() else {
      help_message(&$program_name);
      return Ok(None);
    };
    next_file
  }};
}
impl Jsonpiler {
  #[expect(clippy::print_stdout)]
  fn command_line(&mut self) -> Result<Option<(String, bool, env::Args)>, String> {
    let mut args = env::args();
    let program_name = args.next().unwrap_or("jsonpiler.exe".into());
    let mut file = next_file!(args, program_name);
    let mut build_only = false;
    match file.as_ref() {
      "server" => {
        let mut server = Server::new();
        server.main();
      }
      "help" => help_message(&program_name),
      "version" => println!("jsonpiler version {}", version!()),
      "format" => {
        file = next_file!(args, program_name);
        let source = fs::read(&file).map_err(io_err)?;
        self.parsers.push(Parser::new(
          source,
          0,
          full_path(&file).map_err(io_err)?,
          self.parsers[0].file.clone(),
        ));
        if let Some(out) = self.parsers[0].format() {
          fs::write(file, out).map_err(io_err)?;
        }
      }
      _ => {
        match file.as_ref() {
          "build" => {
            build_only = true;
            file = next_file!(args, program_name);
            if file == "release" {
              self.release = true;
              file = next_file!(args, program_name);
            }
          }
          "release" => {
            self.release = true;
            file = next_file!(args, program_name);
            if file == "build" {
              build_only = true;
              file = next_file!(args, program_name);
            }
          }
          _ => (),
        }
        return Ok(Some((file, build_only, args)));
      }
    }
    Ok(None)
  }
  fn get_std_any(&mut self, get_std_handle: Api, std_id: u32, std_n: LabelId) -> [Inst; 7] {
    [
      mov_d(Rcx, std_id),
      CallApi(get_std_handle),
      Clear(Rcx),
      DecR(Rcx),
      LogicRR(Cmp, Rax, Rcx),
      JCc(E, self.handlers.win),
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
    self.module();
    self.gui();
    self.logic();
    self.io();
    self.string();
    self.variable();
    self.intrinsic();
  }
}
impl Jsonpiler {
  pub(crate) fn compile(&mut self, json: WithPos<Json>) -> ErrOR<()> {
    let data_minimum = self.id();
    self.data.push(Byte(data_minimum, 0x00));
    let heap = self.bss(8, 8);
    self.symbols.insert(HEAP, heap);
    let flag_gui = self.bss(1, 1);
    self.symbols.insert(FLAG_GUI, flag_gui);
    let std_o = self.bss(8, 8);
    self.symbols.insert(STD_O, std_o);
    let std_e = self.bss(8, 8);
    self.symbols.insert(STD_E, std_e);
    let std_i = self.bss(8, 8);
    self.symbols.insert(STD_I, std_i);
    let leak = self.bss(4, 4);
    self.symbols.insert(LEAK_CNT, leak);
    let set_console_cp = self.import(KERNEL32, "SetConsoleCP");
    let set_console_output_cp = self.import(KERNEL32, "SetConsoleOutputCP");
    let get_process_heap = self.import(KERNEL32, "GetProcessHeap");
    let get_std_handle = self.import(KERNEL32, "GetStdHandle");
    let set_ctrl_c_handler = self.import(KERNEL32, "SetConsoleCtrlHandler");
    let root_id = self.id();
    self.root_id = vec![(root_id, vec![])];
    let epilogue = self.id();
    self.seh_handler(root_id)?;
    self.win_handler(root_id)?;
    self.ctrl_c_handler(root_id)?;
    let print_e = self.get_print_e(root_id)?;
    let mut scope = Scope::new(root_id);
    let result = self.eval(json, &mut scope)?.val;
    let tmp = scope.alloc(8, 8)?;
    scope.extend(&mov_int(Rcx, if let Int(int) = result { int } else { Lit(0) }));
    scope.push(mov_q(Local(Tmp, tmp), Rcx));
    self.drop_json(result, &mut scope, false);
    self.drop_all_scope(&mut scope);
    self.drop_global(&mut scope);
    scope.extend(&[
      mov_q(Rax, Local(Tmp, tmp)),
      mov_d(Rcx, Global(leak)),
      LogicRR(Test, Rcx, Rcx),
      CMovCc(E, Rcx, Rax),
      JCc(E, epilogue),
      self.mov_str(Rcx, Lit(INTERNAL_ERR.into())),
      Call(print_e),
      self.mov_str(Rcx, Lit("\n| Memory leak detected".into())),
      Call(print_e),
      self.mov_str(Rcx, Lit(ERR_END.into())),
      Call(print_e),
      self.mov_str(Rcx, Lit(ISSUE.into())),
      Call(print_e),
      self.mov_str(Rcx, Lit("LEAK`\n".into())),
      Call(print_e),
      mov_d(Rcx, Global(leak)),
      Lbl(epilogue),
    ]);
    scope.free(tmp, Size(8));
    scope.check_free()?;
    self.check_unused_functions(self.root_id[0].1.clone());
    let stack_size = scope.resolve_stack_size()?;
    let mut insts = vec![];
    extend!(
      insts,
      [
        mov_d(Global(leak), 0),
        mov_d(Rcx, 65001),
        CallApiCheck(set_console_cp),
        mov_d(Rcx, 65001),
        CallApiCheck(set_console_output_cp),
        CallApiCheck(get_process_heap),
        mov_q(Global(heap), Rax),
        LeaRM(Rcx, Global(self.handlers.ctrl_c)),
        Clear(Rdx),
        IncR(Rdx),
        CallApiCheck(set_ctrl_c_handler),
      ],
      self.get_std_any(get_std_handle, (-10i32).cast_unsigned(), std_i),
      self.get_std_any(get_std_handle, (-11i32).cast_unsigned(), std_o),
      self.get_std_any(get_std_handle, (-12i32).cast_unsigned(), std_e),
      // self.get_std_any(get_std_handle, 0, std_e),
      // [Clear(Rcx), IDivR(Rcx)],
      take(&mut self.startup),
      scope.take_body(),
    );
    self.link_not_return_function(self.root_id[0].0, &insts, stack_size);
    Ok(())
  }
  pub(crate) fn drop_all_scope(&mut self, scope: &mut Scope) {
    for _ in 0..scope.locals.len() {
      self.drop_scope(scope);
    }
    for (name, local) in take(&mut scope.local_top) {
      if !local.val.used && !name.starts_with('_') {
        self.warn(local.pos, UnusedName(LocalVar, name));
      }
      self.drop_json(local.val.val, scope, true);
    }
  }
  pub(crate) fn drop_global(&mut self, scope: &mut Scope) {
    for (name, global) in take(&mut self.globals) {
      if !global.val.used && !name.starts_with('_') {
        self.warn(global.pos, UnusedName(GlobalVar, name));
      }
      if let Some(Memory(addr, Heap(_))) = global.val.val.memory() {
        self.heap_free(addr, scope);
      }
    }
  }
  #[expect(clippy::needless_pass_by_value)]
  pub(crate) fn drop_json(&mut self, json: Json, scope: &mut Scope, force: bool) {
    if let Some(Memory(Local(lifetime, offset), size)) = json.memory()
      && (force || lifetime == Tmp)
    {
      scope.free(offset, size);
      if matches!(size, Heap(_)) {
        self.heap_free(Local(lifetime, offset), scope);
      }
    }
  }
  pub(crate) fn drop_scope(&mut self, scope: &mut Scope) {
    for (name, local) in scope.locals.pop().unwrap_or_default() {
      if !local.val.used && !name.starts_with('_') {
        self.warn(local.pos, UnusedName(LocalVar, name));
      }
      self.drop_json(local.val.val, scope, true);
    }
  }
  pub(crate) fn eval(&mut self, json: WithPos<Json>, scope: &mut Scope) -> ErrOR<WithPos<Json>> {
    Ok(if let Array(Lit(array)) = json.val {
      json.pos.with(Array(Lit(self.eval_args(array, scope)?)))
    } else if let Object(Lit(object)) = json.val {
      self.eval_object(json.pos.with(object), scope)?
    } else {
      json
    })
  }
  fn eval_args(
    &mut self,
    mut args: Vec<WithPos<Json>>,
    scope: &mut Scope,
  ) -> ErrOR<Vec<WithPos<Json>>> {
    for arg in &mut args {
      *arg = self.eval(take(arg), scope)?;
    }
    Ok(args)
  }
  fn eval_func(&mut self, scope: &mut Scope, (name, args): KeyVal) -> ErrOR<Json> {
    if let Some(builtin) = self.builtin.get(&name.val) {
      let BuiltInInfo { scoped, skip_eval, builtin_ptr, arity } = *builtin;
      if scoped {
        scope.locals.push(BTreeMap::new());
      }
      let mut func = self.func_info((name, args), skip_eval, scope)?;
      func.validate_args(arity)?;
      let result = builtin_ptr(self, &mut func, scope)?;
      if scoped {
        self.drop_scope(scope);
      }
      self.free_all(&mut func, scope);
      return Ok(result);
    }
    let Some(UserDefinedInfo { id, params, ret_type, .. }) =
      self.user_defined.get_mut(&name.val).map(|u_d| u_d.val.clone())
    else {
      return err!(name.pos, UndefinedFunc(name.val.clone()));
    };
    self.use_function(scope.id, id);
    self.use_u_d(id, scope.id)?;
    let ret = name.pos.with(ret_type);
    let mut func = self.func_info((name, args), false, scope)?;
    let params_len = len_u32(&params)?;
    scope.update_args_count(params_len);
    func.validate_args(Exact(params_len))?;
    for param in params {
      let arg = func.arg()?;
      if arg.val.as_type() != param {
        return Err(args_type_err(func.nth, &func.name, vec![param], arg.map_ref(Json::as_type)));
      }
      self.mov_args_json(func.nth - 1, scope, arg, true)?;
    }
    scope.push(Call(id));
    let ret_json = scope.ret_json(&ret, Rax)?;
    self.free_all(&mut func, scope);
    Ok(ret_json)
  }
  fn eval_object(
    &mut self,
    object: WithPos<Vec<KeyVal>>,
    scope: &mut Scope,
  ) -> ErrOR<WithPos<Json>> {
    let mut tmp_json = object.pos.with(Null(Lit(())));
    for key_val in object.val {
      self.drop_json(tmp_json.val, scope, false);
      tmp_json.val = self.eval_func(scope, key_val)?;
    }
    Ok(tmp_json)
  }
  pub(crate) fn free_all(&mut self, func: &mut BuiltIn, scope: &mut Scope) {
    for (start, size) in &take(&mut func.free_vec) {
      if matches!(*size, Heap(_)) {
        self.heap_free(Local(Tmp, *start), scope);
      }
      scope.free(*start, *size);
    }
  }
  pub(crate) fn func_info(
    &mut self,
    (WithPos { val: name, pos }, arg): KeyVal,
    skip_eval: bool,
    scope: &mut Scope,
  ) -> ErrOR<BuiltIn> {
    let args_vec = if let Array(Lit(args)) = arg.val { args } else { vec![arg] };
    let args = if skip_eval { args_vec } else { self.eval_args(args_vec, scope)? };
    let mut func = BuiltIn {
      len: len_u32(&args)?,
      name,
      pos,
      args: vec![].into_iter(),
      free_vec: vec![],
      nth: 0,
    };
    if !skip_eval {
      for memory in args.iter().filter_map(|var_arg| var_arg.val.memory()) {
        func.push_free_tmp(memory);
      }
    }
    func.args = args.into_iter();
    Ok(func)
  }
  #[inline]
  pub fn main(&mut self) -> Result<i32, String> {
    let Some((file, build_only, args)) = self.command_line()? else {
      return Ok(0);
    };
    if fs::metadata(&file).map_err(io_err)?.len() > u64::from(GB) {
      return Err(format!("{COMPILATION_ERR}\n| {TooLargeFile}{ERR_END}"));
    }
    let source = fs::read(&file).map_err(io_err)?;
    let exe_path = Path::new(&file).with_extension("exe");
    let exe = exe_path.to_string_lossy().to_string();
    let full = full_path(&file).map_err(io_err)?;
    let first_parser = Parser::new(source, 0, full.clone(), full);
    self.parsers.push(first_parser);
    let parsed = match Path::new(&file).extension().map(|ext| ext.to_string_lossy()) {
      Some(jspl) if jspl == "jspl" => self.parsers[0].parse_jspl(),
      Some(json) if json == "json" => self.parsers[0].parse_json(),
      _ => return Err(format!("{COMPILATION_ERR}\n| {UnsupportedFile}{ERR_END}")),
    }
    .map_err(|err| self.format_err(&err.into()))?;
    self.compile(parsed).map_err(|err| self.format_err(&err))?;
    let (insts, seh) = self.resolve_calls();
    Assembler::new(take(&mut self.dlls), self.root_id[0].0, self.handlers)
      .assemble(&insts, take(&mut self.data), &self.parsers[0].file, seh)
      .map_err(|err| self.format_err(&err))?;
    if build_only {
      return Ok(0);
    }
    check_platform()?;
    let exe_full = env::current_dir().map_err(io_err)?.join(exe);
    Ok(Command::new(exe_full).args(args).status().map_err(io_err)?.code().unwrap_or(0))
  }
  #[must_use]
  #[inline]
  pub fn new() -> Self {
    let mut jsonpiler = Self {
      builtin: HashMap::new(),
      data: vec![],
      dlls: vec![],
      functions: BTreeMap::new(),
      globals: BTreeMap::new(),
      id_seed: 0,
      parsers: vec![],
      release: false,
      root_id: vec![],
      startup: vec![],
      str_cache: HashMap::new(),
      symbols: HashMap::new(),
      handlers: Handlers::default(),
      user_defined: BTreeMap::new(),
    };
    jsonpiler.register_builtin();
    jsonpiler.handlers =
      Handlers { ctrl_c: jsonpiler.id(), seh: jsonpiler.id(), win: jsonpiler.id(), err: None };
    jsonpiler
  }
  pub(crate) fn register_func(
    &mut self,
    name: &str,
    (scoped, skip_eval): (bool, bool),
    builtin_ptr: BuiltinPtr,
    arity: Arity,
  ) {
    self.builtin.insert(name.into(), BuiltInInfo { arity, builtin_ptr, scoped, skip_eval });
  }
}
impl Default for Jsonpiler {
  #[inline]
  fn default() -> Self {
    Jsonpiler::new()
  }
}
impl Jsonpiler {
  pub(crate) fn check_unused_functions(&mut self, root_uses: Vec<LabelId>) {
    let mut visited = root_uses;
    let mut reachable = BTreeSet::new();
    while let Some(id) = visited.pop() {
      if !reachable.insert(id) {
        continue;
      }
      if let Some((_, u_d)) = self.user_defined.iter().find(|(_, u_d)| u_d.val.id == id) {
        for &next in &u_d.val.uses {
          visited.push(next);
        }
      }
    }
    for (name, u_d) in self.user_defined.clone() {
      if !reachable.contains(&u_d.val.id)
        && !name.starts_with('_')
        && !self.parsers[u_d.pos.file as usize].exports.contains_key(&name)
      {
        self.warn(u_d.pos, UnusedName(UserDefinedFunc, name.clone()));
      }
    }
  }
  pub(crate) fn resolve_calls(&mut self) -> (Vec<Inst>, Vec<(LabelId, LabelId, i32)>) {
    let (reachable, seh) = self.resolve_reachable_func();
    let insts = reachable
      .into_iter()
      .filter_map(|reachable_id| self.functions.remove(&reachable_id))
      .flat_map(|asm_func| asm_func.insts)
      .collect::<Vec<_>>();
    (insts, seh)
  }
  fn resolve_reachable_func(&self) -> (BTreeSet<LabelId>, Vec<(LabelId, LabelId, i32)>) {
    let mut seh = vec![];
    let mut visited = vec![self.root_id[0].0];
    let mut reachable = BTreeSet::new();
    while let Some(id) = visited.pop() {
      if !reachable.insert(id) {
        continue;
      }
      if let Some(asm_func) = self.functions.get(&id) {
        if let Some((end, stack_size)) = asm_func.seh {
          seh.push((id, end, stack_size));
        }
        for &next in &asm_func.uses {
          visited.push(next);
        }
      }
    }
    (reachable, seh)
  }
  pub(crate) fn use_u_d(&mut self, id: LabelId, caller: LabelId) -> ErrOR<()> {
    if let Some((_, root_uses)) = self.root_id.iter_mut().rfind(|(root_id, _)| *root_id == caller) {
      root_uses.push(id);
    } else if let Some((_, u_d)) =
      self.user_defined.iter_mut().find(|(_, u_d)| u_d.val.id == caller)
    {
      u_d.val.uses.push(id);
    } else {
      return Err(Internal(UnknownLabel));
    }
    Ok(())
  }
}
#[expect(clippy::print_stdout)]
fn help_message(program_name: &str) {
  println!("Usage: {program_name} <input.jspl | input.json> [args for .exe]{COMMAND}");
}
fn check_platform() -> Result<(), String> {
  if !cfg!(target_os = "windows") {
    return Err(platform_err("Windows x64"));
  }
  if !cfg!(target_arch = "x86_64") {
    return Err(platform_err("x86_64 architecture"));
  }
  if !is_x86_feature_detected!("sse2") {
    return Err(platform_err("a CPU with SSE2 support"));
  }
  Ok(())
}
fn platform_err(reason: &'static str) -> String {
  format!("{PLATFORM_ERR}\n| The generated executable requires {reason}{ERR_END}")
}
#[expect(clippy::needless_pass_by_value)]
fn io_err(err: Error) -> String {
  format!("{IO_ERR}{}{ERR_END}", wrap_text(&err.to_string(), 28))
}
