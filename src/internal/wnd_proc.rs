use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn get_flag_gui(&mut self) -> ErrOR<LabelId> {
    if let Some(flag_gui) = self.symbols.get(FLAG_GUI) {
      Ok(*flag_gui)
    } else {
      Ok(self.bss_symbol(FLAG_GUI, 1, 1))
    }
  }
  pub(crate) fn get_gui_x(
    &mut self,
    caller: LabelId,
    func_pos: Position,
    name: &str,
    render_id: LabelId,
  ) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x100;
    let id = self.id();
    let flag_gui = Global(self.get_flag_gui()?);
    let class_name = self.global_w_chars(TITLE);
    let window_name = self.global_w_chars(name);
    let wnd_proc = self.get_wnd_proc(caller, render_id)?;
    let msg = Local(Tmp, -0x30);
    let hwnd = Local(Tmp, -0x38);
    let wnd_cls = -0x88;
    let size_rect = -0x98;
    let left = -0x88;
    let top = -0x84;
    let right = -0x80;
    let bottom = -0x7C;
    let msg_loop = self.id();
    let exit_gui = self.id();
    let insts = vec![
      vec![
        load(S1, Rax, flag_gui),
        TestRR(S1, Rax),
        JCc(Ne, self.runtime_err(SecondaryGUIErr, None, func_pos, caller)?),
        MovMIb(Mem(flag_gui), bool2byte(true)),
        Clear(Rax),
        store(S8, Local(Tmp, size_rect), Rax),
        store(S8, Local(Tmp, right), Rax),
        MovRI(Rax, 0x40_0000_0050),
        store(S8, Local(Tmp, wnd_cls), Rax),
        LeaRM(Rax, Global(wnd_proc)),
        store(S8, Local(Tmp, wnd_cls + 0x08), Rax),
        Clear(Rax),
        store(S8, Local(Tmp, wnd_cls + 0x10), Rax),
        store(S8, Local(Tmp, wnd_cls + 0x38), Rax),
        store(S8, Local(Tmp, wnd_cls + 0x48), Rax),
        Clear(Rcx),
        CallApiCheck(self.api(KERNEL32, "GetModuleHandleW")),
        store(S8, Local(Tmp, wnd_cls + 0x18), Rax),
        Clear(Rcx),
        MovMId(Reg(Rdx), 0x7F00),
        CallApiCheck(self.api(USER32, "LoadIconW")),
        store(S8, Local(Tmp, wnd_cls + 0x20), Rax),
        Clear(Rcx),
        MovMId(Reg(Rdx), 0x7F00),
        CallApiCheck(self.api(USER32, "LoadCursorW")),
        store(S8, Local(Tmp, wnd_cls + 0x28), Rax),
        MovMId(Reg(Rax), 6),
        store(S8, Local(Tmp, wnd_cls + 0x30), Rax),
      ],
      load_x(Rax, class_name)?,
      vec![
        store(S8, Local(Tmp, wnd_cls + 0x40), Rax),
        LeaRM(Rcx, Local(Tmp, wnd_cls)),
        CallApiCheck(self.api(USER32, "RegisterClassExW")),
        MovMId(Mem(Local(Tmp, right)), GUI_W),
        MovMId(Mem(Local(Tmp, bottom)), GUI_H),
        LeaRM(Rcx, Local(Tmp, size_rect)),
        MovMId(Reg(Rdx), 0xCF_0000),
        Clear(R8),
        Clear(R9),
        CallApiCheck(self.api(USER32, "AdjustWindowRectEx")),
        MovSxDRMd(Rax, Local(Tmp, right)),
        r_m(S8, Sub, Rax, Local(Tmp, left)),
        store(S8, Args(7), Rax),
        MovSxDRMd(Rax, Local(Tmp, bottom)),
        r_m(S8, Sub, Rax, Local(Tmp, top)),
        store(S8, Args(8), Rax),
        MovMId(Reg(Rcx), 0x0004_0000),
      ],
      load_x(Rdx, class_name)?,
      load_x(R8, window_name)?,
      vec![
        MovMId(Reg(R9), 0xCF_0000),
        MovMId(Reg(Rax), 0x8000_0000),
        store(S8, Args(5), Rax),
        store(S8, Args(6), Rax),
        Clear(Rax),
        store(S8, Args(9), Rax),
        store(S8, Args(10), Rax),
        store(S8, Args(12), Rax),
        store(S8, Local(Tmp, wnd_cls + 0x18), Rax),
        store(S8, Args(11), Rax),
        CallApiCheck(self.api(USER32, "CreateWindowExW")),
        store(S8, hwnd, Rax),
        load(S8, Rcx, hwnd),
        MovMId(Reg(Rdx), 5),
        CallApi(self.api(USER32, "ShowWindow")),
        load(S8, Rcx, hwnd),
        CallApiCheck(self.api(USER32, "UpdateWindow")),
        LblX(msg_loop),
        LeaRM(Rcx, msg),
        Clear(Rdx),
        Clear(R8),
        Clear(R9),
        CallApi(self.api(USER32, "GetMessageW")),
        IncR(Rax),
        TestRR(S8, Rax),
        JCc(E, self.handlers.os),
        DecR(Rax),
        TestRR(S8, Rax),
        JCc(E, exit_gui),
        LeaRM(Rcx, msg),
        CallApi(self.api(USER32, "TranslateMessage")),
        LeaRM(Rcx, msg),
        CallApi(self.api(USER32, "DispatchMessageW")),
        Jmp(msg_loop),
        LblX(exit_gui),
        MovMIb(Mem(flag_gui), bool2byte(false)),
        Clear(Rcx),
        CallApiCheck(self.api(KERNEL32, "GetModuleHandleW")),
      ],
      load_x(Rcx, class_name)?,
      vec![mov(S8, Rdx, Rax), CallApiCheck(self.api(USER32, "UnregisterClassW"))],
    ];
    self.use_func(caller, id);
    self.link_lbl_x(id, insts, SIZE, true, FN_RETURN);
    Ok(id)
  }
  #[expect(clippy::too_many_lines)]
  pub(crate) fn get_wnd_proc(&mut self, caller: LabelId, render: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x200;
    const IGNORE_SIZE: i32 = 0x20;
    let gui_frame = Global(self.bss(4, 4));
    let gui_pixels = Global(self.bss(8, 8));
    let id = self.id();
    let ignore = self.id();
    let ignore_epilogue = self.id();
    let wm_destroy = self.id();
    let wm_timer = self.id();
    let wm_paint = self.id();
    let wm_create = self.id();
    let epilogue = self.id();
    let while_x = self.id();
    let while_y = self.id();
    let while_end_x = self.id();
    let while_end_y = self.id();
    let bm_info = Local(Tmp, -0xC0);
    let bmi_size = Local(Tmp, -0xC0);
    let bmi_width = Local(Tmp, -0xBC);
    let bmi_height = Local(Tmp, -0xB8);
    let bmi_planes_bitcount = Local(Tmp, -0xB4);
    let bmi1 = Local(Tmp, -0xB0);
    let bmi2 = Local(Tmp, -0xA8);
    let bmi3 = Local(Tmp, -0xA0);
    let bmi4 = Local(Tmp, -0x98);
    let paint_struct = Local(Tmp, -0x80);
    let ps_left = Local(Tmp, -0x70);
    let ps_top = Local(Tmp, -0x6C);
    let ps_right = Local(Tmp, -0x68);
    let ps_bottom = Local(Tmp, -0x64);
    let hdc = Local(Tmp, -0x38);
    let w_param = Local(Tmp, -0x30);
    let hwnd = Local(Tmp, -0x28);
    let client_rect = Local(Tmp, -0x20);
    let cursor_pos = Local(Tmp, -0x10);
    let left = Local(Tmp, -0x20);
    let top = Local(Tmp, -0x1C);
    let right = Local(Tmp, -0x18);
    let bottom = Local(Tmp, -0x14);
    let cursor_x = Local(Tmp, -0x10);
    let cursor_y = Local(Tmp, -0xC);
    let pixel_x = Local(Tmp, -0x8);
    let pixel_y = Local(Tmp, -0x4);
    let insts = vec![
      vec![
        store(S8, hwnd, Rcx),
        store(S8, w_param, R8),
        m8i(Cmp, Rdx, 1),
        JCc(E, wm_create),
        m8i(Cmp, Rdx, 2),
        JCc(E, wm_destroy),
        m8i(Cmp, Rdx, 0xF),
        JCc(E, wm_paint),
        m8i(Cmp, Rdx, 0x113),
        JCc(E, wm_timer),
        CallApi(self.api(USER32, "DefWindowProcW")),
        Jmp(epilogue),
        LblX(wm_timer),
        load(S8, Rcx, hwnd),
        Clear(Rdx),
        Clear(R8),
        CallApiCheck(self.api(USER32, "InvalidateRect")),
        IncMd(gui_frame),
        Clear(Rax),
        Jmp(epilogue),
        LblX(wm_create),
        load(S8, Rcx, hwnd),
        MovMId(Reg(Rdx), 1),
        MovMId(Reg(R8), TIMER_INTERVAL_MS),
        Clear(R9),
        CallApiCheck(self.api(USER32, "SetTimer")),
        MovRI(R8, GUI_PIXELS_SIZE),
      ],
      self.call_alloc_r8_x()?,
      vec![
        store(S8, gui_pixels, Rax),
        Clear(Rax),
        Jmp(epilogue),
        LblX(wm_destroy),
        load(S8, Rcx, hwnd),
        MovMId(Reg(Rdx), 1),
        CallApiCheck(self.api(USER32, "KillTimer")),
        load(S8, R8, gui_pixels),
      ],
      self.call_free_r8_x()?,
      vec![
        Clear(Rcx),
        CallApi(self.api(USER32, "PostQuitMessage")),
        Clear(Rax),
        Jmp(epilogue),
        LblX(wm_paint),
        MovMId(Mem(bmi_size), 0x28),
        MovMId(Mem(bmi_width), GUI_W),
        MovMId(Mem(bmi_height), GUI_H),
        MovMId(Mem(bmi_planes_bitcount), (32 << 16) | 1),
        Clear(Rax),
        store(S8, bmi1, Rax),
        store(S8, bmi2, Rax),
        store(S8, bmi3, Rax),
        store(S8, bmi4, Rax),
        LeaRM(Rcx, cursor_pos),
        CallApi(self.api(USER32, "GetCursorPos")),
        Call(ignore),
        load(S8, Rcx, hwnd),
        LeaRM(Rdx, cursor_pos),
        CallApiCheck(self.api(USER32, "ScreenToClient")),
        load(S8, Rcx, hwnd),
        LeaRM(Rdx, client_rect),
        CallApiCheck(self.api(USER32, "GetClientRect")),
      ],
      adjust_xy(cursor_x, right, left, GUI_W, false)?,
      adjust_xy(cursor_y, bottom, top, GUI_H, true)?,
      vec![
        MovMId(Mem(pixel_y), 0),
        LblX(while_y),
        m4i(Cmp, pixel_y, i32::try_from(GUI_H)?),
        JCc(E, while_end_y),
        MovMId(Mem(pixel_x), 0),
        LblX(while_x),
        m4i(Cmp, pixel_x, i32::try_from(GUI_W)?),
        JCc(E, while_end_x),
        load(S4, Rcx, pixel_x),
        m8i(Sub, Rcx, i32::try_from(GUI_W >> 1)?),
        load(S4, Rdx, pixel_y),
        m8i(Sub, Rdx, i32::try_from(GUI_H >> 1)?),
        load(S4, R8, gui_frame),
        MovSxDRMd(R9, cursor_y),
        store(S8, Args(5), R9),
        MovSxDRMd(R9, cursor_x),
        Call(render),
        load(S4, R8, pixel_y),
        MovMId(Reg(Rdx), GUI_W),
        IMulR2(R8, Rdx),
        load(S4, Rcx, pixel_x),
        RR(S8, Add, R8, Rcx),
        load(S8, Rcx, gui_pixels),
        store(S4, SibDisp(Sib { base: Rcx, index: R8, scale: S4 }, Disp::Zero), Rax),
        IncMd(pixel_x),
        Jmp(while_x),
        LblX(while_end_x),
        IncMd(pixel_y),
        Jmp(while_y),
        LblX(while_end_y),
        load(S8, Rcx, hwnd),
        LeaRM(Rdx, paint_struct),
        CallApiCheck(self.api(USER32, "BeginPaint")),
        store(S8, hdc, Rax),
        mov(S8, Rcx, Rax),
        MovMId(Reg(Rdx), 0x4),
        CallApiCheck(self.api(GDI32, "SetStretchBltMode")),
        load(S8, Rcx, hdc),
        Clear(Rdx),
        Clear(R8),
        Clear(R9),
        CallApiCheck(self.api(GDI32, "SetBrushOrgEx")),
        load(S8, Rcx, hdc),
        Clear(Rdx),
        Clear(R8),
        load(S4, R9, ps_top),
        r_m(S4, Sub, R9, ps_bottom),
        load(S4, Rax, ps_right),
        r_m(S4, Sub, Rax, ps_left),
        store(S8, Args(5), Rax),
        store(S8, Args(6), Rdx),
        store(S8, Args(7), Rdx),
        MovMId(Args(8), GUI_W),
        MovMId(Args(9), GUI_H),
        load(S8, Rax, gui_pixels),
        store(S8, Args(10), Rax),
        LeaRM(Rax, bm_info),
        store(S8, Args(11), Rax),
        store(S8, Args(12), Rdx),
        MovMId(Args(13), 0xCC_0020),
        CallApi(self.api(GDI32, "StretchDIBits")),
        Call(ignore),
        load(S8, Rcx, hwnd),
        LeaRM(Rdx, paint_struct),
        CallApi(self.api(USER32, "EndPaint")),
        Clear(Rax),
        LblX(epilogue),
      ],
    ];
    self.use_func(caller, id);
    self.link_func_x(id, insts, SIZE);
    let ignore_insts = vec![
      TestRR(S8, Rax),
      JCc(Ne, ignore_epilogue),
      CallApi(self.api(KERNEL32, "GetLastError")),
      m8i(Cmp, Rax, 5),
      JCc(Ne, self.handlers.os),
      LblX(ignore_epilogue),
    ];
    self.use_func(id, ignore);
    self.link_func_x(ignore, vec![ignore_insts], IGNORE_SIZE);
    Ok(id)
  }
}
pub(crate) fn adjust_xy(
  cursor: Address,
  window_a: Address,
  window_b: Address,
  size: u32,
  neg: bool,
) -> ErrOR<Vec<X64Inst>> {
  let mut insts = vec![
    MovSxDRMd(Rax, cursor),
    MovMId(Reg(Rcx), size),
    IMulR2(Rax, Rcx),
    load(S4, Rcx, window_a),
    r_m(S4, Sub, Rcx, window_b),
    Clear(Rdx),
    RR(S8, Cmp, Rcx, Rdx),
    CMovCc(Le, Rcx, Rax),
    CMovCc(Le, Rax, Rdx),
    Cqo,
    IDivR(Rcx),
    MovMId(Reg(Rcx), size),
    RR(S8, Cmp, Rax, Rcx),
    CMovCc(G, Rax, Rcx),
    m8i(Sub, Rax, i32::try_from(size >> 1)?),
  ];
  if neg {
    insts.push(Unary(S8, Neg, Rax));
  }
  insts.push(store(S4, cursor, Rax));
  Ok(insts)
}
