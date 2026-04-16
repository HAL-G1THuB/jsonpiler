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
  pub(crate) fn compile(&mut self, json: Pos<Json>) -> ErrOR<()> {
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
    let mut scope = Scope::new(self.parsers[0].val.dep.id);
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
    scope.free(tmp, MemoryType { heap: Value, size: Small(RQ) });
    scope.check_free()?;
    self.check_unused_functions(&self.parsers[0].val.dep.clone());
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
    self.link_not_return_function(self.parsers[0].val.dep.id, &insts, stack_size);
    Ok(())
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
  }
}
impl Default for Jsonpiler {
  #[inline]
  fn default() -> Self {
    Jsonpiler::new()
  }
}
