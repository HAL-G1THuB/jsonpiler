mod arithmetic;
mod compare;
mod compound;
mod control;
mod define;
mod gui;
mod intrinsic;
mod io;
mod logic;
mod module;
mod string;
mod variable;
use crate::prelude::*;
use std::{env, process::Command};
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
    let program_name = args.next().unwrap_or(PKG_NAME.into());
    let mut file = next_file!(args, program_name);
    let mut build_only = false;
    match file.as_ref() {
      "server" => {
        let mut server = Server::new();
        server.main();
      }
      "help" => help_message(&program_name),
      "version" => println!("{PKG_NAME} version {VERSION}"),
      "format" => {
        file = next_file!(args, program_name);
        let source = fs::read(&file).map_err(|err| self.io_err(err))?;
        let full_path = full_path(&file).map_err(|err| self.io_err(err))?;
        self.parsers.push(Parser::new(source, 0, full_path, self.parsers[0].file.clone(), 0));
        if let Some(out) = self.parsers[0].format() {
          fs::write(file, out).map_err(|err| self.io_err(err))?;
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
    self.symbol(FLAG_GUI, 1);
    let heap = self.symbol(HEAP, 8);
    let std_o = self.symbol(STD_O, 8);
    let std_e = self.symbol(STD_E, 8);
    let std_i = self.symbol(STD_I, 8);
    let leak = self.symbol(LEAK_CNT, 4);
    let set_console_cp = self.import(KERNEL32, "SetConsoleCP");
    let set_console_output_cp = self.import(KERNEL32, "SetConsoleOutputCP");
    let get_process_heap = self.import(KERNEL32, "GetProcessHeap");
    let get_std_handle = self.import(KERNEL32, "GetStdHandle");
    let set_ctrl_c_handler = self.import(KERNEL32, "SetConsoleCtrlHandler");
    let epilogue = self.id();
    let mut scope = Scope::new(self.parsers[0].dep.id);
    self.seh_handler(scope.id)?;
    self.win_handler(scope.id)?;
    self.ctrl_c_handler(scope.id)?;
    let print_e = self.get_print_e(scope.id)?;
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
      self.mov_str(Rcx, Lit(make_header(INTERNAL_ERR))),
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
    self.check_unused_functions(self.parsers[0].dep.clone());
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
    self.link_not_return_function(self.parsers[0].dep.id, &insts, stack_size);
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
      if let Some(memory) = global.val.val.memory() {
        self.heap_free_memory(memory, scope);
      }
    }
  }
  #[expect(clippy::needless_pass_by_value)]
  pub(crate) fn drop_json(&mut self, json: Json, scope: &mut Scope, force: bool) {
    if let Some(memory @ Memory(Local(lifetime, offset), size)) = json.memory()
      && (force || lifetime == Tmp)
    {
      scope.free(offset, size);
      self.heap_free_memory(memory, scope);
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
    let Some(UserDefinedInfo { dep, params, ret_type }) =
      self.user_defined.get_mut(&name.val).map(|u_d| u_d.val.clone())
    else {
      return err!(name.pos, UndefinedFunc(name.val.clone()));
    };
    self.use_function(scope.id, dep.id);
    self.use_u_d(scope.id, dep.id)?;
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
    scope.push(Call(dep.id));
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
    for memory in &take(&mut func.free_list) {
      self.heap_free_memory(*memory, scope);
      if let Memory(Local(Tmp, start), mem_type) = memory {
        scope.free(*start, *mem_type);
      }
    }
  }
  pub(crate) fn func_info(
    &mut self,
    (WithPos { val: name, pos }, val): KeyVal,
    skip_eval: bool,
    scope: &mut Scope,
  ) -> ErrOR<BuiltIn> {
    let args_vec = if let Array(Lit(args)) = val.val { args } else { vec![val] };
    let args = if skip_eval { args_vec } else { self.eval_args(args_vec, scope)? };
    let mut func = BuiltIn {
      len: len_u32(&args)?,
      name,
      pos,
      args: vec![].into_iter(),
      free_list: BTreeSet::new(),
      nth: 0,
    };
    if !skip_eval {
      for arg in &args {
        func.push_free_tmp(arg.val.memory());
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
    if fs::metadata(&file).map_err(|err| self.io_err(err))?.len() > u64::from(GB) {
      return Err(self.format_err(&Compilation(TooLargeFile, vec![])));
    }
    let source = fs::read(&file).map_err(|err| self.io_err(err))?;
    let exe_path = Path::new(&file).with_extension("exe");
    let exe = exe_path.to_string_lossy().to_string();
    let full = full_path(&file).map_err(|err| self.io_err(err))?;
    let first_parser = Parser::new(source, 0, full.clone(), full, self.id());
    self.parsers.push(first_parser);
    let parsed = match Path::new(&file).extension().map(|ext| ext.to_string_lossy()) {
      Some(jspl) if jspl == "jspl" => self.parsers[0].parse_jspl(),
      Some(json) if json == "json" => self.parsers[0].parse_json(),
      _ => return Err(self.format_err(&Compilation(UnsupportedFile, vec![]))),
    }
    .map_err(|err| self.format_err(&err.into()))?;
    self.compile(parsed).map_err(|err| self.format_err(&err))?;
    let (insts, seh) = self.resolve_calls();
    Assembler::new(take(&mut self.dlls), self.parsers[0].dep.id, self.handlers)
      .assemble(&insts, take(&mut self.data), &self.parsers[0].file, seh)
      .map_err(|err| self.format_err(&err))?;
    if build_only {
      return Ok(0);
    }
    check_platform()?;
    let exe_full = env::current_dir().map_err(|err| self.io_err(err))?.join(exe);
    let status = Command::new(exe_full).args(args).status().map_err(|err| self.io_err(err))?;
    Ok(status.code().unwrap_or(0))
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
    builtin_ptr: BuiltInPtr,
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
  pub(crate) fn check_unused_functions(&mut self, mut root_dep: Dependency) {
    let reachable = root_dep.reachable(
      &self
        .user_defined
        .values()
        .map(|u_d| &u_d.val.dep)
        .chain(self.parsers.iter().map(|parser| &parser.dep))
        .collect::<Vec<&Dependency>>(),
    );
    for (name, u_d) in self.user_defined.clone() {
      if !reachable.contains(&u_d.val.dep.id)
        && !name.starts_with('_')
        && !self.parsers[u_d.pos.file as usize].exports.contains_key(&name)
      {
        self.warn(u_d.pos, UnusedName(UserDefinedFunc, name.clone()));
      }
    }
  }
  pub(crate) fn resolve_calls(&mut self) -> (Vec<Inst>, Vec<(LabelId, LabelId, i32)>) {
    let reachable = self.parsers[0].dep.clone().reachable(
      &self.functions.values().map(|compiled| &compiled.dep).collect::<Vec<&Dependency>>(),
    );
    let mut seh = vec![];
    for compiled in self.functions.values().filter(|compiled| reachable.contains(&compiled.dep.id))
    {
      if let Some((end, stack_size)) = compiled.seh {
        seh.push((compiled.dep.id, end, stack_size));
      }
    }
    let insts = reachable
      .into_iter()
      .filter_map(|id| self.functions.remove(&id))
      .flat_map(|compiled| compiled.insts)
      .collect::<Vec<Inst>>();
    (insts, seh)
  }
  pub(crate) fn use_u_d(&mut self, caller: LabelId, id: LabelId) -> ErrOR<()> {
    let dep = if let Some(root) = self.parsers.iter_mut().rfind(|root| root.dep.id == caller) {
      &mut root.dep
    } else if let Some(u_d) = self.user_defined.values_mut().find(|u_d| u_d.val.dep.id == caller) {
      &mut u_d.val.dep
    } else {
      return Err(Internal(UnknownLabel));
    };
    dep.uses.push(id);
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
fn platform_err(requirement: &'static str) -> String {
  format!(
    "{}\n| The generated executable requires {requirement}{ERR_END}",
    make_header("PlatformError")
  )
}
