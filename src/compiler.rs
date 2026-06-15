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
impl Jsonpiler {
  pub(crate) fn compile(&mut self, json: Pos<Json>) -> ErrOR<()> {
    if self.flags.a64 { self.compile_a(json) } else { self.compile_x(json) }
  }
  pub(crate) fn compile_a(&mut self, json: Pos<Json>) -> ErrOR<()> {
    let mut scope = Scope::new(self.first_file()?.dep.id, true);
    self.sig_handler(scope.id)?;
    self.unix_handler(scope.id)?;
    self.ctrl_c_handler_a(scope.id)?;
    let result = self.eval(json, &mut scope)?.val;
    let tmp = scope.alloc(8, 8)?;
    scope.e_a(load_int_a(X0, if let Int(int) = result { int } else { Lit(0) })?)?;
    scope.e_a(store_a(Local(Tmp, tmp).v_rq(), X1, X0)?)?;
    self.drop_all(result, &mut scope)?;
    scope.ee_a(self.stack_leak_a(scope.id, tmp)?)?;
    scope.free(tmp, 8)?;
    scope.check_free()?;
    self.check_unused_functions(0)?;
    let stack_size = scope.resolve_a64_stack_size()?;
    let mut insts = self.startup_a()?;
    insts.extend(scope.take_body().a64()?);
    self.link_func_a(scope.id, insts, stack_size);
    self.push_sym_builtin();
    Ok(())
  }
  pub(crate) fn compile_x(&mut self, json: Pos<Json>) -> ErrOR<()> {
    let data_minimum = self.id();
    self.data.push((data_minimum, Byte(0x00)));
    let mut scope = Scope::new(self.first_file()?.dep.id, false);
    self.seh_handler(scope.id)?;
    self.win_handler(scope.id)?;
    self.ctrl_c_handler_x(scope.id)?;
    let result = self.eval(json, &mut scope)?.val;
    let tmp = scope.alloc(8, 8)?;
    scope.e_x(load_int_x(Rcx, if let Int(int) = result { int } else { Lit(0) })?)?;
    scope.p_x(store(S8, Local(Tmp, tmp), Rcx))?;
    self.drop_all(result, &mut scope)?;
    scope.e_x(self.stack_leak_x(scope.id, tmp)?)?;
    scope.free(tmp, 8)?;
    scope.check_free()?;
    self.check_unused_functions(0)?;
    let stack_size = scope.resolve_x64_stack_size()?;
    let mut insts = self.startup_x()?;
    insts.extend(scope.take_body().x64()?);
    self.link_lbl_x(scope.id, insts, stack_size, true, FN_NOT_RETURN);
    self.push_sym_builtin();
    Ok(())
  }
  #[must_use]
  #[inline]
  pub fn new(analysis: bool) -> Self {
    let a64 = cfg!(target_arch = "aarch64") && cfg!(target_os = "macos");
    let mut jsonpiler = Self {
      analysis: analysis.then(Analysis::default),
      builtin: BTreeMap::new(),
      data: vec![],
      apis: vec![],
      functions: BTreeMap::new(),
      globals: BTreeMap::new(),
      id_seed: 0,
      files: vec![],
      flags: JsonpilerFlags { debug: true, build_only: false, a64 },
      startup: AorX::new(a64),
      str_cache: HashMap::new(),
      symbols: HashMap::new(),
      handlers: Handlers::default(),
      user_defined: BTreeMap::new(),
    };
    jsonpiler.register_builtin();
    jsonpiler.handlers =
      Handlers { ctrl_c: jsonpiler.id(), internal: jsonpiler.id(), os: jsonpiler.id(), err: None };
    jsonpiler
  }
  fn push_sym_builtin(&mut self) {
    let name_refs: Vec<_> =
      self.builtin.iter().map(|(name, builtin)| (name.clone(), builtin.refs.clone())).collect();
    // dummy
    let dummy = FuncT(Signature { params: vec![], ret_type: NullT }.into());
    for (name, refs) in name_refs {
      self.push_sym(SymbolInfo {
        def: None,
        json_type: dummy.clone(),
        kind: BuiltInFunc,
        name,
        refs,
      });
    }
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
  pub(crate) fn register_func(
    &mut self,
    name: &'static str,
    (scoped, skip_eval): (bool, bool),
    builtin_a64: Option<BuiltInPtr>,
    builtin_x64: BuiltInPtr,
    arity: Arity,
  ) {
    let builtin = BuiltInInfo { arity, builtin_a64, builtin_x64, refs: vec![], scoped, skip_eval };
    self.builtin.insert(name.into(), builtin);
  }
  fn sig_action_a(&mut self, sig: i64, sig_action: LabelId) -> ErrOR<Vec<A64Inst>> {
    let insts: &[&[A64Inst]] = &[
      &load_imm_a(X0, sig),
      &load_ref_a(X1, Global(sig_action).ptr())?,
      &[MovRR(X2, Xzr), BApi(self.api(SYS_B, "_sigaction"))],
    ];
    Ok(insts.concat())
  }
  fn stack_leak_a(&mut self, scope_id: LabelId, tmp: i32) -> ErrOR<Vec<Vec<A64Inst>>> {
    let leak = Global(self.get_leak()?).v_rd();
    let epilogue = self.id();
    let print_e = self.get_print_e_a(scope_id)?;
    let mut insts = vec![
      load_a(X0, Local(Tmp, tmp).v_rq())?,
      load_a(X1, leak)?,
      vec![CmpRR(X1, Xzr), BCc(E.into(), epilogue)],
    ];
    let msgs = [&make_header(INTERNAL_ERR), "\n| Memory leak detected", ERR_END, ISSUE, "LEAK`\n"];
    for msg in msgs {
      insts.extend_from_slice(&[self.load_str_a(X0, &Lit(msg.into()))?, vec![Bl(print_e)]]);
    }
    insts.extend_from_slice(&[load_a(X0, leak)?, vec![LblA(epilogue)]]);
    Ok(insts)
  }
  fn stack_leak_x(&mut self, scope_id: LabelId, tmp: i32) -> ErrOR<Vec<X64Inst>> {
    let leak = self.get_leak()?;
    let epilogue = self.id();
    let print_e = self.get_print_e_x(scope_id)?;
    let mut insts = vec![
      load(S8, Rcx, Local(Tmp, tmp)),
      load(S4, Rax, Global(leak)),
      TestRR(S8, Rax),
      JCc(E, epilogue),
    ];
    let msgs = [&make_header(INTERNAL_ERR), "\n| Memory leak detected", ERR_END, ISSUE, "LEAK`\n"];
    for msg in msgs {
      insts.extend(self.load_str_x(Rcx, &Lit(msg.into()))?);
      insts.push(Call(print_e));
    }
    insts.extend([load(S4, Rcx, Global(leak)), LblX(epilogue)]);
    Ok(insts)
  }
  fn startup_a(&mut self) -> ErrOR<Vec<Vec<A64Inst>>> {
    let sig_action_ctrl_c = self.bss(16, 16);
    let sig_action_internal = self.bss(16, 16);
    let mut startup = vec![
      load_ref_a(X0, Global(sig_action_ctrl_c).ptr())?,
      load_ref_a(X1, Global(self.handlers.ctrl_c).ptr())?,
      vec![StR(S8, X1, X0, 0)],
      load_ref_a(X0, Global(sig_action_internal).ptr())?,
      load_ref_a(X1, Global(self.handlers.internal).ptr())?,
      vec![StR(S8, X1, X0, 0)],
      self.sig_action_a(SIGHUP, sig_action_ctrl_c)?,
      self.sig_action_a(SIGINT, sig_action_ctrl_c)?,
      self.sig_action_a(SIGQUIT, sig_action_ctrl_c)?,
      self.sig_action_a(SIGABRT, sig_action_ctrl_c)?,
      self.sig_action_a(SIGTERM, sig_action_ctrl_c)?,
      self.sig_action_a(SIGSEGV, sig_action_internal)?,
      self.sig_action_a(SIGILL, sig_action_internal)?,
      // vec![B_(self.handlers.os)],
    ];
    startup.extend(take(&mut self.startup).a64()?);
    Ok(startup)
  }
  fn startup_x(&mut self) -> ErrOR<Vec<Vec<X64Inst>>> {
    let mut startup = vec![vec![
      MovMId(Reg(Rcx), CP_UTF8),
      CallApiCheck(self.api(KERNEL32, "SetConsoleCP")),
      MovMId(Reg(Rcx), CP_UTF8),
      CallApiCheck(self.api(KERNEL32, "SetConsoleOutputCP")),
      LeaRM(Rcx, Global(self.handlers.ctrl_c)),
      MovMId(Reg(Rdx), 1),
      CallApiCheck(self.api(KERNEL32, "SetConsoleCtrlHandler")),
    ]];
    startup.extend(take(&mut self.startup).x64()?);
    // startup.push({
    //   let bss = self.bss(8, 8);
    //   self.get_std_n_x(u32::MAX, bss)?
    // });
    // startup.push(vec![Clear(Rcx), IDivR(Rcx)]);
    Ok(startup)
  }
}
impl Default for Jsonpiler {
  #[inline]
  fn default() -> Self {
    Jsonpiler::new(false)
  }
}
