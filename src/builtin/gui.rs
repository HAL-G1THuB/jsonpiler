use crate::{
  Arity::Exactly,
  Bind::Lit,
  CompilationErrKind::*,
  ConditionCode::*,
  DataInst::RDAlign,
  ErrOR, FuncInfo,
  Inst::*,
  Json, Jsonpiler,
  JsonpilerErr::*,
  LogicByteOpcode::*,
  Memory::{Global, GlobalD},
  Operand::Args,
  Register::*,
  ScopeInfo, WithPos, built_in,
  dll::*,
  err, take_arg,
  utility::{args_type_error, mov_b, mov_d, mov_q},
};
use core::mem::discriminant;
built_in! {self, func, scope, gui;
init_gui => {"GUI", COMMON, Exactly(1), {
  const TITLE: &str = "J\0s\0o\0n\0p\0i\0l\0e\0r\0 \0G\0U\0I\0\0";
  let render_name = take_arg!(self, func, (String(Lit(x))) => x);
  let render_id;
  {
    let Some(render) = self.user_defined.get(&render_name.value) else {
      return err!(
        self, render_name.pos,
        UndefinedFn(render_name.value)
      );
    };
    let int = discriminant(&Json::Int(Lit(0)));
    if render.params.len() != 5
    {
      return err!(self, render_name.pos, ArityError { name: "render".into(), expected: Exactly(5), supplied: render.params.len() })
    }
    let mut nth = 0;
    for param in &render.params {
      nth += 1;
      if discriminant(param) != int
      {
        return Err(args_type_error(nth, "render", "Int".into(), &WithPos { value: param.clone(), pos: render_name.pos }));
      }
    }
    render_id = render.id;
  };
  scope.update_stack_args(8);
  let get_module_handle = self.import(KERNEL32, "GetModuleHandleW")?;
  let load_icon_w = self.import(USER32, "LoadIconW")?;
  let load_cursor_w = self.import(USER32, "LoadCursorW")?;
  let register_class_ex_w = self.import(USER32, "RegisterClassExW")?;
  let create_window_ex_w = self.import(USER32, "CreateWindowExW")?;
  let adjust_window_rect_ex = self.import(USER32, "AdjustWindowRectEx")?;
  let show_window = self.import(USER32, "ShowWindow")?;
  let update_window = self.import(USER32, "UpdateWindow")?;
  let get_message_w = self.import(USER32, "GetMessageW")?;
  let translate_message = self.import(USER32, "TranslateMessage")?;
  let dispatch_message_w = self.import(USER32, "DispatchMessageW")?;
  self.data_insts.push(RDAlign(2));
  let class_name = self.global_str(TITLE.into()).0;
  self.data_insts.push(RDAlign(2));
  let window_name = self.global_str(TITLE.into()).0;
  let wnd_proc = self.get_wnd_proc(render_id)?;
  let wnd_class = self.get_bss_id(0x50, 8);
  let msg = self.get_bss_id(0x30, 8);
  let gui_handle = self.get_bss_id(8, 8);
  let size_rect = self.get_bss_id(16, 8);
  let msg_loop = self.gen_id();
  let exit_gui = self.gen_id();
  scope.extend(&[
    mov_b(Rax, self.g_symbol("FLAG_GUI")),
    LogicRbRb(Test, Rax, Rax),
    JCc(Ne, self.get_custom_error("GUI already initialized")?),
    mov_b(self.g_symbol("FLAG_GUI"), 0xFF),
    mov_q(Rax, 0x40_0000_0050),
    mov_q(GlobalD { id: wnd_class, disp: 0x00 }, Rax),
    LeaRM(Rax, Global { id: wnd_proc }),
    mov_q(GlobalD { id: wnd_class, disp: 0x08 }, Rax),
    Clear(Rax),
    mov_q(GlobalD { id: wnd_class, disp: 0x10 }, Rax),
    Clear(Rcx),
  ]);
  scope.extend(&self.call_api_check_null(get_module_handle));
  scope.extend(&[
    mov_q(GlobalD { id: wnd_class, disp: 0x18 }, Rax),
    Clear(Rcx),
    mov_d(Rdx, 0x7F00),
  ]);
  scope.extend(&self.call_api_check_null(load_icon_w));
  scope.extend(&[
    mov_q(GlobalD { id: wnd_class, disp: 0x20 }, Rax),
    Clear(Rcx),
    mov_d(Rdx, 0x7F00),
  ]);
  scope.extend(&self.call_api_check_null(load_cursor_w));
  scope.extend(&[
    mov_q(GlobalD { id: wnd_class, disp: 0x28 }, Rax),
    mov_d(Rax, 6),
    mov_q(GlobalD { id: wnd_class, disp: 0x30 }, Rax),
    Clear(Rax),
    mov_q(GlobalD { id: wnd_class, disp: 0x38 }, Rax),
    LeaRM(Rax, Global { id: class_name }),
    mov_q(GlobalD { id: wnd_class, disp: 0x40 }, Rax),
    Clear(Rax),
    mov_q(GlobalD { id: wnd_class, disp: 0x48 }, Rax),
    LeaRM(Rcx, GlobalD { id: wnd_class, disp: 0 }),
  ]);
  scope.extend(&self.call_api_check_null(register_class_ex_w));
  scope.extend(&[
    mov_d(GlobalD { id: size_rect, disp: 8 }, Jsonpiler::GUI_W),
    mov_d(GlobalD { id: size_rect, disp: 12 }, Jsonpiler::GUI_H),
    LeaRM(Rcx, GlobalD { id: size_rect, disp: 0 }),
    mov_d(Rdx, 0xCF_0000),
    Clear(R8),
    Clear(R9),
  ]);
  scope.extend(&self.call_api_check_null(adjust_window_rect_ex));
  scope.extend(&[
    mov_d(Rax, GlobalD { id: size_rect, disp: 8 }),
    mov_d(Rcx, GlobalD { id: size_rect, disp: 0 }),
    SubRR(Rax, Rcx),
    mov_q(Args(0x30), Rax),
    mov_d(Rax, GlobalD { id: size_rect, disp: 12 }),
    mov_d(Rcx, GlobalD { id: size_rect, disp: 4 }),
    SubRR(Rax, Rcx),
    mov_q(Args(0x38), Rax),
    mov_d(Rcx, 0x0004_0000),
    LeaRM(Rdx, Global { id: class_name }),
    LeaRM(R8, Global { id: window_name }),
    mov_d(R9, 0xCF_0000),
    mov_d(Rax, 0x8000_0000),
    mov_q(Args(0x20), Rax),
    mov_q(Args(0x28), Rax),
    Clear(Rax),
    mov_q(Args(0x40), Rax),
    mov_q(Args(0x48), Rax),
    mov_q(Args(0x58), Rax),
    mov_q(Rax, GlobalD { id: wnd_class, disp: 0x18 }),
    mov_q(Args(0x50), Rax)
  ]);
  scope.extend(&self.call_api_check_null(create_window_ex_w));
  scope.extend(&[
    mov_q(Global { id: gui_handle }, Rax),
    mov_q(Rcx, Global { id: gui_handle }),
    mov_d(Rdx, 5),
    CallApi(show_window),
    mov_q(Rcx, Global { id: gui_handle }),
  ]);
  scope.extend(&self.call_api_check_null(update_window));
  scope.extend(&[
    Lbl(msg_loop),
    LeaRM(Rcx, Global { id: msg }),
    Clear(Rdx),
    Clear(R8),
    Clear(R9),
    CallApi(get_message_w),
    IncR(Rax),
    LogicRR(Test, Rax, Rax),
    JCc(E, self.sym_table["WIN_HANDLER"]),
    DecR(Rax),
    LogicRR(Test, Rax, Rax),
    JCc(E, exit_gui),
    LeaRM(Rcx, Global { id: msg }),
    CallApi(translate_message),
    LeaRM(Rcx, Global { id: msg }),
    CallApi(dispatch_message_w),
    Jmp(msg_loop),
    Lbl(exit_gui),
  ]);
  Ok(Json::Null)
}}
}
