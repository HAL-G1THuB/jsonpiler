mod arithmetic;
mod compare;
mod compound;
mod control;
mod define;
mod evaluator;
mod gui;
mod intrinsic;
mod io;
mod logic;
mod module;
mod string;
mod variable;
use crate::prelude::*;
impl Jsonpiler {
  fn check_stack_leak(&mut self, scope_id: LabelId, tmp: i32) -> ErrOR<Vec<Inst>> {
    let leak = self.symbols[LEAK_CNT];
    let epilogue = self.id();
    let print_e = self.get_print_e(scope_id)?;
    Ok(vec![
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
    ])
  }
  pub(crate) fn compile(&mut self, json: Pos<Json>) -> ErrOR<()> {
    const BSS_SYMBOLS: &[(&str, u32)] =
      &[(FLAG_GUI, 1), (HEAP, 8), (STD_O, 8), (STD_E, 8), (STD_I, 8), (LEAK_CNT, 4)];
    let data_minimum = self.id();
    self.data.push(Byte(data_minimum, 0x00));
    for (name, size) in BSS_SYMBOLS {
      self.bss_symbol(name, *size);
    }
    let mut scope = Scope::new(self.first_parser()?.val.dep.id);
    self.seh_handler(scope.id)?;
    self.win_handler(scope.id)?;
    self.ctrl_c_handler(scope.id)?;
    let result = self.eval(json, &mut scope)?.val;
    let tmp = scope.alloc(8, 8)?;
    scope.extend(&mov_int(Rcx, if let Int(int) = result { int } else { Lit(0) }));
    scope.push(mov_q(Local(Tmp, tmp), Rcx));
    self.drop_all(result, &mut scope)?;
    scope.extend(&self.check_stack_leak(scope.id, tmp)?);
    scope.free(tmp, MemoryType { heap: Value, size: Small(RQ) });
    scope.check_free()?;
    self.check_unused_functions(&self.first_parser()?.val.dep.clone())?;
    let stack_size = scope.resolve_stack_size()?;
    let mut insts = self.startup()?;
    insts.extend_from_slice(&scope.take_body());
    self.link_label(self.first_parser()?.val.dep.id, &insts, stack_size, true, FN_NOT_RETURN);
    Ok(())
  }
  pub(crate) fn first_parser(&self) -> ErrOR<&Pos<Parser>> {
    self.parsers.first().ok_or(Internal(MissingFirstParser))
  }
  pub(crate) fn first_parser_mut(&mut self) -> ErrOR<&mut Pos<Parser>> {
    self.parsers.first_mut().ok_or(Internal(MissingFirstParser))
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
  #[must_use]
  #[inline]
  pub fn new(analysis: bool) -> Self {
    let mut jsonpiler = Self {
      analysis: analysis.then(|| Analysis { symbols: vec![] }),
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
    builtin_ptr: BuiltInPtr,
    arity: Arity,
  ) {
    self.builtin.insert(name, BuiltInInfo { arity, builtin_ptr, scoped, skip_eval });
    // TODO
    self.push_symbol(SymbolInfo {
      definition: None,
      json_type: FuncT(vec![], NullT.into()),
      kind: BuiltInFunc,
      name: name.to_owned(),
      refs: vec![],
    });
  }
  fn startup(&mut self) -> ErrOR<Vec<Inst>> {
    let std_i = self.symbols[STD_I];
    let std_o = self.symbols[STD_O];
    let std_e = self.symbols[STD_E];
    let heap = self.symbols[HEAP];
    let set_console_cp = self.import(KERNEL32, "SetConsoleCP");
    let set_console_output_cp = self.import(KERNEL32, "SetConsoleOutputCP");
    let get_process_heap = self.import(KERNEL32, "GetProcessHeap");
    let get_std_handle = self.import(KERNEL32, "GetStdHandle");
    let set_ctrl_c_handler = self.import(KERNEL32, "SetConsoleCtrlHandler");
    let mut insts = vec![
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
    ];
    extend!(
      insts,
      self.get_std_any(get_std_handle, (-10i32).cast_unsigned(), std_i),
      self.get_std_any(get_std_handle, (-11i32).cast_unsigned(), std_o),
      self.get_std_any(get_std_handle, (-12i32).cast_unsigned(), std_e),
      // self.get_std_any(get_std_handle, 0, std_e),
      // [Clear(Rcx), IDivR(Rcx)],
      take(&mut self.startup),
    );
    Ok(insts)
  }
}
impl Default for Jsonpiler {
  #[inline]
  fn default() -> Self {
    Jsonpiler::new(false)
  }
}
