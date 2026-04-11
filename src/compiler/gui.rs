use crate::prelude::*;
built_in! {self, func, scope, gui;
init_gui => {"GUI", SPECIAL, Exact(1), {
  let name = func.arg()?.into_ident("render")?;
  let render_id = {
    let Some(render) = self.user_defined.get(&name.val).map(|u_d|u_d.val.clone()) else {
      return err!(name.pos, UndefinedFunc(name.val));
    };
    self.use_function(scope.id, render.id);
    self.use_u_d(render.id, scope.id)?;
    if render.params.len() != 5
    {
      return err!(
        name.pos,
        ArityError { name: "render".into(), expected: Exact(5), actual: len_u32(&render.params)? }
      )
    }
    let mut nth = 0;
    for param in &render.params {
      nth += 1;
      if param != &IntT
      {
        return Err(args_type_err(nth, "render", vec![IntT], name.pos.with(param.clone())));
      }
    }
    if render.ret_type != IntT {
      return Err(type_err("`render`'s return value".into(), vec![IntT], name.pos.with(render.ret_type.clone())));
    }
    render.id
  };
  scope.update_args_count(12);
  let get_module_handle = self.import(KERNEL32, "GetModuleHandleW");
  let load_icon_w = self.import(USER32, "LoadIconW");
  let load_cursor_w = self.import(USER32, "LoadCursorW");
  let register_class_ex_w = self.import(USER32, "RegisterClassExW");
  let unregister_class_w = self.import(USER32, "UnregisterClassW");
  let create_window_ex_w = self.import(USER32, "CreateWindowExW");
  let adjust_window_rect_ex = self.import(USER32, "AdjustWindowRectEx");
  let show_window = self.import(USER32, "ShowWindow");
  let update_window = self.import(USER32, "UpdateWindow");
  let get_message_w = self.import(USER32, "GetMessageW");
  let translate_message = self.import(USER32, "TranslateMessage");
  let dispatch_message_w = self.import(USER32, "DispatchMessageW");
  let flag_gui = Global(self.symbols[FLAG_GUI]);
  let class_name = Global(self.global_w_chars(TITLE));
  let window_name = Global(self.global_w_chars(name.val));
  let wnd_proc = self.get_wnd_proc(scope.id, render_id)?;
  let wnd_cls = scope.alloc(0x50, 8)?;
  func.push_free_tmp(Memory(Local(Tmp, wnd_cls), Size(0x50)));
  let msg = Local(Tmp, scope.alloc(0x30, 8)?);
  func.push_free_tmp(Memory(msg, Size(0x30)));
  let hwnd = Local(Tmp, scope.alloc(8, 8)?);
  func.push_free_tmp(Memory(hwnd, Size(8)));
  let size_rect = scope.alloc(0x10, 8)?;
  func.push_free_tmp(Memory(Local(Tmp, size_rect), Size(0x10)));
  let left = size_rect;
  let top = size_rect + 4;
  let right = size_rect + 8;
  let bottom = size_rect + 12;
  let msg_loop = self.id();
  let exit_gui = self.id();
  scope.extend(&[
    mov_b(Rax, flag_gui),
    LogicRbRb(Test, Rax, Rax),
    JCc(Ne, self.custom_err(SecondaryGUIErr, None, func.pos, scope.id)?),
    mov_b(flag_gui, 0xFF),
    Clear(Rax),
    mov_q(Local(Tmp, size_rect), Rax),
    mov_q(Local(Tmp, right), Rax),
    mov_q(Rax, 0x40_0000_0050),
    mov_q(Local(Tmp, wnd_cls), Rax),
    LeaRM(Rax, Global(wnd_proc)),
    mov_q(Local(Tmp, wnd_cls + 0x08), Rax),
    Clear(Rax),
    mov_q(Local(Tmp, wnd_cls + 0x10), Rax),
    mov_q(Local(Tmp, wnd_cls + 0x38), Rax),
    mov_q(Local(Tmp, wnd_cls + 0x48), Rax),
    Clear(Rcx),
    CallApiCheck(get_module_handle),
    mov_q(Local(Tmp, wnd_cls + 0x18), Rax),
    Clear(Rcx),
    mov_d(Rdx, 0x7F00),
    CallApiCheck(load_icon_w),
    mov_q(Local(Tmp, wnd_cls + 0x20), Rax),
    Clear(Rcx),
    mov_d(Rdx, 0x7F00),
    CallApiCheck(load_cursor_w),
    mov_q(Local(Tmp, wnd_cls + 0x28), Rax),
    mov_d(Rax, 6),
    mov_q(Local(Tmp, wnd_cls + 0x30), Rax),
    LeaRM(Rax, class_name),
    mov_q(Local(Tmp, wnd_cls + 0x40), Rax),
    LeaRM(Rcx, Local(Tmp, wnd_cls)),
    CallApiCheck(register_class_ex_w),
    mov_d(Local(Tmp, right), GUI_W),
    mov_d(Local(Tmp, bottom), GUI_H),
    LeaRM(Rcx, Local(Tmp, size_rect)),
    mov_d(Rdx, 0xCF_0000),
    Clear(R8),
    Clear(R9),
    CallApiCheck(adjust_window_rect_ex),
    mov_d(Rax, Local(Tmp, right)),
    mov_d(Rcx, Local(Tmp, left)),
    SubRR(Rax, Rcx),
    mov_q(Args(7), Rax),
    mov_d(Rax, Local(Tmp, bottom)),
    mov_d(Rcx, Local(Tmp, top)),
    SubRR(Rax, Rcx),
    mov_q(Args(8), Rax),
    mov_d(Rcx, 0x0004_0000),
    LeaRM(Rdx, class_name),
    LeaRM(R8, window_name),
    mov_d(R9, 0xCF_0000),
    mov_d(Rax, 0x8000_0000),
    mov_q(Args(5), Rax),
    mov_q(Args(6), Rax),
    Clear(Rax),
    mov_q(Args(9), Rax),
    mov_q(Args(10), Rax),
    mov_q(Args(12), Rax),
    mov_q(Rax, Local(Tmp, wnd_cls + 0x18)),
    mov_q(Args(11), Rax),
    CallApiCheck(create_window_ex_w),
    mov_q(hwnd, Rax),
    mov_q(Rcx, hwnd),
    mov_d(Rdx, 5),
    CallApi(show_window),
    mov_q(Rcx, hwnd),
    CallApiCheck(update_window),
    Lbl(msg_loop),
    LeaRM(Rcx, msg),
    Clear(Rdx),
    Clear(R8),
    Clear(R9),
    CallApi(get_message_w),
    IncR(Rax),
    LogicRR(Test, Rax, Rax),
    JCc(E, self.handlers.win),
    DecR(Rax),
    LogicRR(Test, Rax, Rax),
    JCc(E, exit_gui),
    LeaRM(Rcx, msg),
    CallApi(translate_message),
    LeaRM(Rcx, msg),
    CallApi(dispatch_message_w),
    Jmp(msg_loop),
    Lbl(exit_gui),
    mov_b(flag_gui, 0),
    Clear(Rcx),
    CallApiCheck(get_module_handle),
    LeaRM(Rcx, class_name),
    mov_q(Rdx, Rax),
    CallApiCheck(unregister_class_w),
  ]);
  Ok(Null(Lit(())))
}}
}
