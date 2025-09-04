use crate::{
  Arity::Exactly,
  Bind::Lit,
  ConditionCode::*,
  DataInst::RDAlign,
  ErrOR, FuncInfo,
  Inst::*,
  Json, Jsonpiler,
  LogicByteOpcode::*,
  Operand::Args,
  Register::*,
  ScopeInfo,
  VarKind::Global,
  built_in, err, take_arg,
  utility::{mov_b, mov_d, mov_q},
};
use core::mem::discriminant;
built_in! {self, func, scope, gui;
init_gui => {"GUI", COMMON, Exactly(1), {
  let pixel_func_name = take_arg!(self, func, "String (Literal)", Json::String(Lit(x)) => x);
  let pixel_func_id;
  {
    let Some(pixel_func) = self.user_defined.get(&pixel_func_name.value) else {
      return err!(
        self, pixel_func_name.pos,
        "Undefined function: `{}`", pixel_func_name.value
      );
    };
    let int = discriminant(&Json::Int(Lit(0)));
    if pixel_func.params.len() != 5
      || pixel_func.params.first().is_some_and(|x| discriminant(x) != int)
      || pixel_func.params.get(1).is_some_and(|x| discriminant(x) != int)
      || pixel_func.params.get(2).is_some_and(|x| discriminant(x) != int)
      || pixel_func.params.get(3).is_some_and(|x| discriminant(x) != int)
      || pixel_func.params.last().is_some_and(|x| discriminant(x) != int)
      || discriminant(&pixel_func.ret) != int
    {
      return err!(self, pixel_func_name.pos, "ArityError: `GUI` function must have 5 arguments (x, y, frame, mouse_x, mouse_y).");
    }
    pixel_func_id = pixel_func.id;
  };
  scope.update_stack_args(8);
  let get_module_handle = self.import(Jsonpiler::KERNEL32, "GetModuleHandleW")?;
  let load_icon = self.import(Jsonpiler::USER32, "LoadIconW")?;
  let load_cursor = self.import(Jsonpiler::USER32, "LoadCursorW")?;
  let register_class = self.import(Jsonpiler::USER32, "RegisterClassExW")?;
  let create_window_ex = self.import(Jsonpiler::USER32, "CreateWindowExW")?;
  let adjust_window_rect_ex = self.import(Jsonpiler::USER32, "AdjustWindowRectEx")?;
  let show_window = self.import(Jsonpiler::USER32, "ShowWindow")?;
  let update_window = self.import(Jsonpiler::USER32, "UpdateWindow")?;
  let get_message = self.import(Jsonpiler::USER32, "GetMessageW")?;
  let translate_message = self.import(Jsonpiler::USER32, "TranslateMessage")?;
  let dispatch_message = self.import(Jsonpiler::USER32, "DispatchMessageW")?;
  self.data_insts.push(RDAlign(2));
  let class_name = self.global_str(String::from("J\0s\0o\0n\0p\0i\0l\0e\0r\0 \0G\0U\0I\0\0")).0;
  self.data_insts.push(RDAlign(2));
  let window_name = self.global_str(String::from("J\0s\0o\0n\0p\0i\0l\0e\0r\0 \0G\0U\0I\0\0")).0;
  let wnd_proc = if let Some(wnd_proc) = self.sym_table.get("WND_PROC").copied() {
    wnd_proc
  } else {
    self.get_wnd_proc(pixel_func_id)?
  };
  let wnd_class = self.get_bss_id(0x50, 8);
  let msg = self.get_bss_id(0x30, 8);
  let gui_handle = self.get_bss_id(8, 8);
  let size_rect = self.get_bss_id(16, 8);
  let msg_loop = self.gen_id();
  let exit_gui = self.gen_id();
  scope.extend(&[
    mov_b(Rax, Global { id: self.sym_table["FLAG_GUI"], disp: 0 }),
    LogicRbRb(Test, Rax, Rax),
    Jcc(Ne, self.get_custom_error("GUI already initialized")?),
    mov_b(Global { id: self.sym_table["FLAG_GUI"], disp: 0 }, 0xFF),
    mov_q(Rax, 0x40_0000_0050),
    mov_q(Global { id: wnd_class, disp: 0x00 }, Rax),
    LeaRM(Rax, Global { id: wnd_proc, disp: 0 }),
    mov_q(Global { id: wnd_class, disp: 0x08 }, Rax),
    Clear(Rax),
    mov_q(Global { id: wnd_class, disp: 0x10 }, Rax),
    Clear(Rcx),
  ]);
  scope.extend(&self.call_api_check_null(get_module_handle));
  scope.extend(&[
    mov_q(Global { id: wnd_class, disp: 0x18 }, Rax),
    Clear(Rcx),
    mov_d(Rdx, 0x7F00),
  ]);
  scope.extend(&self.call_api_check_null(load_icon));
  scope.extend(&[
    mov_q(Global { id: wnd_class, disp: 0x20 }, Rax),
    Clear(Rcx),
    mov_d(Rdx, 0x7F00),
  ]);
  scope.extend(&self.call_api_check_null(load_cursor));
  scope.extend(&[
    mov_q(Global { id: wnd_class, disp: 0x28 }, Rax),
    mov_d(Rax, 6),
    mov_q(Global { id: wnd_class, disp: 0x30 }, Rax),
    Clear(Rax),
    mov_q(Global { id: wnd_class, disp: 0x38 }, Rax),
    LeaRM(Rax, Global { id: class_name, disp: 0 }),
    mov_q(Global { id: wnd_class, disp: 0x40 }, Rax),
    Clear(Rax),
    mov_q(Global { id: wnd_class, disp: 0x48 }, Rax),
    LeaRM(Rcx, Global { id: wnd_class, disp: 0 }),
  ]);
  scope.extend(&self.call_api_check_null(register_class));
  scope.extend(&[
    mov_d(Global { id: size_rect, disp: 8 }, Jsonpiler::GUI_W),
    mov_d(Global { id: size_rect, disp: 12 }, Jsonpiler::GUI_H),
    LeaRM(Rcx, Global { id: size_rect, disp: 0 }),
    mov_d(Rdx, 0xCF_0000),
    Clear(R8),
    Clear(R9),
  ]);
  scope.extend(&self.call_api_check_null(adjust_window_rect_ex));
  scope.extend(&[
    mov_d(Rax, Global { id: size_rect, disp: 8 }),
    mov_d(Rcx, Global { id: size_rect, disp: 0 }),
    SubRR(Rax, Rcx),
    mov_q(Args(0x30), Rax),
    mov_d(Rax, Global { id: size_rect, disp: 12 }),
    mov_d(Rcx, Global { id: size_rect, disp: 4 }),
    SubRR(Rax, Rcx),
    mov_q(Args(0x38), Rax),
    mov_d(Rcx, 0x0004_0000),
    LeaRM(Rdx, Global { id: class_name, disp: 0 }),
    LeaRM(R8, Global { id: window_name, disp: 0 }),
    mov_d(R9, 0xCF_0000),
    mov_d(Rax, 0x8000_0000),
    mov_q(Args(0x20), Rax),
    mov_q(Args(0x28), Rax),
    Clear(Rax),
    mov_q(Args(0x40), Rax),
    mov_q(Args(0x48), Rax),
    mov_q(Args(0x58), Rax),
    mov_q(Rax, Global { id: wnd_class, disp: 0x18 }),
    mov_q(Args(0x50), Rax)
  ]);
  scope.extend(&self.call_api_check_null(create_window_ex));
  scope.extend(&[
    mov_q(Global { id: gui_handle, disp: 0 }, Rax),
    mov_q(Rcx, Global { id: gui_handle, disp: 0 }),
    mov_d(Rdx, 5),
    CallApi(show_window),
    mov_q(Rcx, Global { id: gui_handle, disp: 0 }),
  ]);
  scope.extend(&self.call_api_check_null(update_window));
  scope.extend(&[
    Lbl(msg_loop),
    LeaRM(Rcx, Global { id: msg, disp: 0 }),
    Clear(Rdx),
    Clear(R8),
    Clear(R9),
    CallApi(get_message),
    IncR(Rax),
    LogicRR(Test, Rax, Rax),
    Jcc(E, self.sym_table["WIN_HANDLER"]),
    DecR(Rax),
    LogicRR(Test, Rax, Rax),
    Jcc(E, exit_gui),
    LeaRM(Rcx, Global { id: msg, disp: 0 }),
    CallApi(translate_message),
    LeaRM(Rcx, Global { id: msg, disp: 0 }),
    CallApi(dispatch_message),
    Jmp(msg_loop),
    Lbl(exit_gui),
  ]);
  Ok(Json::Null)
}}
}
