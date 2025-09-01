use crate::{
  Arity::Exactly, Bind::Lit, ConditionCode::*, ErrOR, FuncInfo, Inst::*, Json, Jsonpiler, OpQ::*,
  Reg::*, ScopeInfo, VarKind::Global, built_in, err, take_arg,
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
      return err!(self, pixel_func_name.pos, "ArityError: `GUI` function must have 3 arguments (x, y, frame).");
    }
    pixel_func_id = pixel_func.id;
  };
  scope.update_stack_args(8);
  let get_module_handle = self.import(Jsonpiler::KERNEL32, "GetModuleHandleW", 0x281);
  let load_icon = self.import(Jsonpiler::USER32, "LoadIconW", 0x25A);
  let load_cursor = self.import(Jsonpiler::USER32, "LoadCursorW", 0x258);
  let register_class = self.import(Jsonpiler::USER32, "RegisterClassExW", 0x2DE);
  let create_window_ex = self.import(Jsonpiler::USER32, "CreateWindowExW", 0x76);
  let show_window = self.import(Jsonpiler::USER32, "ShowWindow", 0x38D);
  let update_window = self.import(Jsonpiler::USER32, "UpdateWindow", 0x3C7);
  let get_message = self.import(Jsonpiler::USER32, "GetMessageW", 0x189);
  let translate_message = self.import(Jsonpiler::USER32, "TranslateMessage", 0x3AD);
  let dispatch_message = self.import(Jsonpiler::USER32, "DispatchMessageW", 0xBD);
  let class_name = self.global_str("J\0s\0o\0n\0p\0i\0l\0e\0r\0 \0G\0U\0I\0\0".to_owned());
  let window_name = self.global_str("J\0s\0o\0n\0p\0i\0l\0e\0r\0 \0G\0U\0I\0\0".to_owned());
  let wnd_class = self.get_bss_id(0x50, 8);
  let msg = self.get_bss_id(0x30, 8);
  let gui_handle = self.get_bss_id(8, 8);
  let msg_loop = self.gen_id();
  let exit_gui = self.gen_id();
  scope.extend(&[
    MovRbMb(Rax, Global { id: self.sym_table["FLAG_GUI"], disp: 0 }),
    TestRbRb(Rax, Rax),
    Jcc(Ne, self.get_custom_error("GUI already initialized")),
    MovMbIb(Global { id: self.sym_table["FLAG_GUI"], disp: 0 }, 0xFF),
    MovQQ(Rq(Rax), Iq(0x40_0000_0050)),
    MovQQ(Mq(Global { id: wnd_class, disp: 0x00 }), Rq(Rax)),
    LeaRM(Rax, Global { id: self.sym_table.get("WND_PROC").copied().unwrap_or_else(|| self.get_wnd_proc(pixel_func_id)), disp: 0 }),
    MovQQ(Mq(Global { id: wnd_class, disp: 0x08 }), Rq(Rax)),
    Clear(Rax),
    MovQQ(Mq(Global { id: wnd_class, disp: 0x10 }), Rq(Rax)),
    Clear(Rcx),
  ]);
  scope.extend(&self.call_api_check_null(get_module_handle));
  scope.extend(&[
    MovQQ(Mq(Global { id: wnd_class, disp: 0x18 }), Rq(Rax)),
    Clear(Rcx),
    MovRId(Rdx, 0x7F00),
  ]);
  scope.extend(&self.call_api_check_null(load_icon));
  scope.extend(&[
    MovQQ(Mq(Global { id: wnd_class, disp: 0x20 }), Rq(Rax)),
    Clear(Rcx),
    MovRId(Rdx, 0x7F00),
  ]);
  scope.extend(&self.call_api_check_null(load_cursor));
  scope.extend(&[
    MovQQ(Mq(Global { id: wnd_class, disp: 0x28 }), Rq(Rax)),
    MovRId(Rax, 6),
    MovQQ(Mq(Global { id: wnd_class, disp: 0x30 }), Rq(Rax)),
    Clear(Rax),
    MovQQ(Mq(Global { id: wnd_class, disp: 0x38 }), Rq(Rax)),
    LeaRM(Rax, Global { id: class_name, disp: 8 }),
    MovQQ(Mq(Global { id: wnd_class, disp: 0x40 }), Rq(Rax)),
    Clear(Rax),
    MovQQ(Mq(Global { id: wnd_class, disp: 0x48 }), Rq(Rax)),
    LeaRM(Rcx, Global { id: wnd_class, disp: 0 }),
  ]);
  scope.extend(&self.call_api_check_null(register_class));
  scope.extend(&[
    MovRId(Rcx, 0x0004_0000),
    LeaRM(Rdx, Global { id: class_name, disp: 8 }),
    LeaRM(R8, Global { id: window_name, disp: 8 }),
    MovRId(R9, 0xCF_0000),
    MovRId(Rax, 0x8000_0000),
    MovQQ(Args(0x20), Rq(Rax)),
    MovQQ(Args(0x28), Rq(Rax)),
    MovQQ(Args(0x30), Rq(Rax)),
    MovQQ(Args(0x38), Rq(Rax)),
    Clear(Rax),
    MovQQ(Args(0x40), Rq(Rax)),
    MovQQ(Args(0x48), Rq(Rax)),
    MovQQ(Args(0x58), Rq(Rax)),
    MovQQ(Rq(Rax), Mq(Global { id: wnd_class, disp: 0x18 })),
    MovQQ(Args(0x50), Rq(Rax))
  ]);
  scope.extend(&self.call_api_check_null(create_window_ex));
  scope.extend(&[
    MovQQ(Mq(Global { id: gui_handle, disp: 0 }), Rq(Rax)),
    MovQQ(Rq(Rcx), Mq(Global { id: gui_handle, disp: 0 })),
    MovRId(Rdx, 5),
    CallApi(show_window),
    MovQQ(Rq(Rcx), Mq(Global { id: gui_handle, disp: 0 })),
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
    TestRR(Rax, Rax),
    Jcc(E, self.sym_table["WIN_HANDLER"]),
    DecR(Rax),
    TestRR(Rax, Rax),
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
