use crate::prelude::*;
built_in! {self, func, scope, gui;
init_gui => {"GUI", SPECIAL, Exact(1), {
  let name = func.arg()?.into_ident("render")?;
  let render_id = {
    let Some(u_d) = self.user_defined.get_mut(&name.val) else {
      return err!(name.pos, UndefinedFunc(name.val));
    };
    u_d.val.refs.push(name.pos);
    let render = u_d.val.clone();
    self.use_function(scope.id, render.dep.id);
    self.use_u_d(scope.id, render.dep.id)?;
    if render.sig.params.len() != 5
    {
      return err!(
        name.pos,
        ArityError { name: "render".into(), expected: Exact(5), actual: len_u32(&render.sig.params)? }
      )
    }
    for (param_name, param_type) in &render.sig.params {
      if param_type != &IntT
      {
        return Err(type_err(format!("argument `{param_name}`"), vec![IntT], name.pos.with(param_type.clone())));
      }
    }
    if render.sig.ret_type != IntT {
      return Err(type_err(format_ret_val("render"), vec![IntT], name.pos.with(render.sig.ret_type.clone())));
    }
    render.dep.id
  };
  let flag_gui = Global(self.symbols[FLAG_GUI]);
  let class_name = Global(self.global_w_chars(TITLE));
  let window_name = Global(self.global_w_chars(name.val));
  let wnd_proc = self.get_wnd_proc(scope.id, render_id)?;
  let msg = scope.tmp(0x30, 8, func)?;
  let hwnd = scope.tmp(8, 8, func)?;
  let wnd_cls = scope.tmp_offset(0x50, 8, func)?;
  let size_rect = scope.tmp_offset(0x10, 8, func)?;
  let left = size_rect;
  let top = size_rect + 4;
  let right = size_rect + 8;
  let bottom = size_rect + 12;
  let msg_loop = self.id();
  let exit_gui = self.id();
  scope.update_args_count(12);
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
    CallApiCheck(self.api(KERNEL32, "GetModuleHandleW")),
    mov_q(Local(Tmp, wnd_cls + 0x18), Rax),
    Clear(Rcx),
    mov_d(Rdx, 0x7F00),
    CallApiCheck(self.api(USER32, "LoadIconW")),
    mov_q(Local(Tmp, wnd_cls + 0x20), Rax),
    Clear(Rcx),
    mov_d(Rdx, 0x7F00),
    CallApiCheck(self.api(USER32, "LoadCursorW")),
    mov_q(Local(Tmp, wnd_cls + 0x28), Rax),
    mov_d(Rax, 6),
    mov_q(Local(Tmp, wnd_cls + 0x30), Rax),
    LeaRM(Rax, class_name),
    mov_q(Local(Tmp, wnd_cls + 0x40), Rax),
    LeaRM(Rcx, Local(Tmp, wnd_cls)),
    CallApiCheck(self.api(USER32, "RegisterClassExW")),
    mov_d(Local(Tmp, right), GUI_W),
    mov_d(Local(Tmp, bottom), GUI_H),
    LeaRM(Rcx, Local(Tmp, size_rect)),
    mov_d(Rdx, 0xCF_0000),
    Clear(R8),
    Clear(R9),
    CallApiCheck(self.api(USER32, "AdjustWindowRectEx")),
    MovSxDRMd(Rax, Local(Tmp, right)),
    MovSxDRMd(Rcx, Local(Tmp, left)),
    SubRR(Rax, Rcx),
    mov_q(Args(7), Rax),
    MovSxDRMd(Rax, Local(Tmp, bottom)),
    MovSxDRMd(Rcx, Local(Tmp, top)),
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
    CallApiCheck(self.api(USER32, "CreateWindowExW")),
    mov_q(hwnd, Rax),
    mov_q(Rcx, hwnd),
    mov_d(Rdx, 5),
    CallApi(self.api(USER32, "ShowWindow")),
    mov_q(Rcx, hwnd),
    CallApiCheck(self.api(USER32, "UpdateWindow")),
    Lbl(msg_loop),
    LeaRM(Rcx, msg),
    Clear(Rdx),
    Clear(R8),
    Clear(R9),
    CallApi(self.api(USER32, "GetMessageW")),
    IncR(Rax),
    LogicRR(Test, Rax, Rax),
    JCc(E, self.handlers.win),
    DecR(Rax),
    LogicRR(Test, Rax, Rax),
    JCc(E, exit_gui),
    LeaRM(Rcx, msg),
    CallApi(self.api(USER32, "TranslateMessage")),
    LeaRM(Rcx, msg),
    CallApi(self.api(USER32, "DispatchMessageW")),
    Jmp(msg_loop),
    Lbl(exit_gui),
    mov_b(flag_gui, 0),
    Clear(Rcx),
    CallApiCheck(self.api(KERNEL32, "GetModuleHandleW")),
    LeaRM(Rcx, class_name),
    mov_q(Rdx, Rax),
    CallApiCheck(self.api(USER32, "UnregisterClassW")),
  ]);
  Ok(Null(Lit(())))
}}
}
