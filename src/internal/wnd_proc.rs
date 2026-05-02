use crate::prelude::*;
impl Jsonpiler {
  #[expect(clippy::too_many_lines)]
  pub(crate) fn get_wnd_proc(&mut self, caller: LabelId, render: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x200;
    const IGNORE_SIZE: i32 = 0x20;
    let gui_frame = Global(self.bss(4, 4));
    let heap = Global(self.symbols[HEAP]);
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
    let bm_info = Local(Long, -0xC0);
    let bmi_size_width = Local(Long, -0xC0);
    let bmi_height_planes_bitcount = Local(Long, -0xB8);
    let bmi1 = Local(Long, -0xB0);
    let bmi2 = Local(Long, -0xA8);
    let bmi3 = Local(Long, -0xA0);
    let bmi4 = Local(Long, -0x98);
    let paint_struct = Local(Long, -0x80);
    let ps_left = Local(Long, -0x70);
    let ps_top = Local(Long, -0x6C);
    let ps_right = Local(Long, -0x68);
    let ps_bottom = Local(Long, -0x64);
    let hdc = Local(Long, -0x38);
    let w_param = Local(Long, -0x30);
    let hwnd = Local(Long, -0x28);
    let client_rect = Local(Long, -0x20);
    let cursor_pos = Local(Long, -0x10);
    let left = Local(Long, -0x20);
    let top = Local(Long, -0x1C);
    let right = Local(Long, -0x18);
    let bottom = Local(Long, -0x14);
    let cursor_x = Local(Long, -0x10);
    let cursor_y = Local(Long, -0xC);
    let pixel_x = Local(Long, -0x8);
    let pixel_y = Local(Long, -0x4);
    let insts = &[
      mov_q(hwnd, Rcx),
      mov_q(w_param, R8),
      mov_d(Rax, 1),
      LogicRR(Cmp, Rdx, Rax),
      JCc(E, wm_create),
      mov_d(Rax, 2),
      LogicRR(Cmp, Rdx, Rax),
      JCc(E, wm_destroy),
      mov_d(Rax, 0xF),
      LogicRR(Cmp, Rdx, Rax),
      JCc(E, wm_paint),
      mov_d(Rax, 0x113),
      LogicRR(Cmp, Rdx, Rax),
      JCc(E, wm_timer),
      CallApi(self.api(USER32, "DefWindowProcW")),
      Jmp(epilogue),
      Lbl(wm_timer),
      mov_q(Rcx, hwnd),
      Clear(Rdx),
      Clear(R8),
      CallApiCheck(self.api(USER32, "InvalidateRect")),
      IncMd(gui_frame),
      Clear(Rax),
      Jmp(epilogue),
      Lbl(wm_create),
      mov_q(Rcx, hwnd),
      Clear(Rdx),
      IncR(Rdx),
      mov_d(R8, TIMER_INTERVAL_MS),
      Clear(R9),
      CallApiCheck(self.api(USER32, "SetTimer")),
      mov_q(Rcx, heap),
      mov_d(Rdx, 8),
      mov_q(R8, GUI_PIXELS_SIZE),
      CallApiCheck(self.api(KERNEL32, "HeapAlloc")),
      IncMd(Global(self.symbols[LEAK_CNT])),
      mov_q(gui_pixels, Rax),
      Clear(Rax),
      Jmp(epilogue),
      Lbl(wm_destroy),
      mov_q(Rcx, hwnd),
      Clear(Rdx),
      IncR(Rdx),
      CallApiCheck(self.api(USER32, "KillTimer")),
      mov_q(Rcx, heap),
      Clear(Rdx),
      mov_q(R8, gui_pixels),
      CallApiCheck(self.api(KERNEL32, "HeapFree")),
      DecMd(Global(self.symbols[LEAK_CNT])),
      Clear(Rcx),
      CallApi(self.api(USER32, "PostQuitMessage")),
      Clear(Rax),
      Jmp(epilogue),
      Lbl(wm_paint),
      mov_q(Rax, (u64::from(GUI_W) << 32) | 0x28),
      mov_q(bmi_size_width, Rax),
      mov_q(Rax, (0x20 << 48) | (1 << 32) | u64::from(GUI_H)),
      mov_q(bmi_height_planes_bitcount, Rax),
      Clear(Rax),
      mov_q(bmi1, Rax),
      mov_q(bmi2, Rax),
      mov_q(bmi3, Rax),
      mov_q(bmi4, Rax),
      LeaRM(Rcx, cursor_pos),
      CallApi(self.api(USER32, "GetCursorPos")),
      Call(ignore),
      mov_q(Rcx, hwnd),
      LeaRM(Rdx, cursor_pos),
      CallApiCheck(self.api(USER32, "ScreenToClient")),
      mov_q(Rcx, hwnd),
      LeaRM(Rdx, client_rect),
      CallApiCheck(self.api(USER32, "GetClientRect")),
      MovSxDRMd(Rax, cursor_x),
      mov_d(Rcx, GUI_W),
      IMulRR(Rax, Rcx),
      mov_d(Rcx, right),
      mov_d(Rdx, left),
      SubRR(Rcx, Rdx),
      Clear(Rdx),
      LogicRR(Cmp, Rcx, Rdx),
      CMovCc(Le, Rcx, Rax),
      CMovCc(Le, Rax, Rdx),
      Custom(CQO),
      IDivR(Rcx),
      mov_d(Rcx, GUI_W),
      LogicRR(Cmp, Rax, Rcx),
      CMovCc(G, Rax, Rcx),
      SubRId(Rax, GUI_W >> 1),
      mov_d(cursor_x, Rax),
      MovSxDRMd(Rax, cursor_y),
      mov_d(Rcx, GUI_H),
      IMulRR(Rax, Rcx),
      mov_d(Rcx, bottom),
      mov_d(Rdx, top),
      SubRR(Rcx, Rdx),
      Clear(Rdx),
      LogicRR(Cmp, Rcx, Rdx),
      CMovCc(Le, Rcx, Rax),
      CMovCc(Le, Rax, Rdx),
      Custom(CQO),
      IDivR(Rcx),
      mov_d(Rcx, GUI_H),
      LogicRR(Cmp, Rax, Rcx),
      CMovCc(G, Rax, Rcx),
      SubRId(Rax, GUI_H >> 1),
      UnaryR(Neg, Rax),
      mov_d(cursor_y, Rax),
      mov_d(pixel_y, 0),
      Lbl(while_y),
      mov_d(Rcx, pixel_y),
      mov_d(Rdx, GUI_H),
      LogicRR(Cmp, Rcx, Rdx),
      JCc(E, while_end_y),
      mov_d(pixel_x, 0),
      Lbl(while_x),
      mov_d(Rcx, pixel_x),
      mov_d(Rdx, GUI_W),
      LogicRR(Cmp, Rcx, Rdx),
      JCc(E, while_end_x),
      mov_d(Rcx, pixel_x),
      SubRId(Rcx, GUI_W >> 1),
      mov_d(Rdx, pixel_y),
      SubRId(Rdx, GUI_H >> 1),
      mov_d(R8, gui_frame),
      MovSxDRMd(R9, cursor_y),
      mov_q(Args(5), R9),
      MovSxDRMd(R9, cursor_x),
      Call(render),
      mov_d(R8, pixel_y),
      mov_d(Rdx, GUI_W),
      IMulRR(R8, Rdx),
      mov_d(Rcx, pixel_x),
      AddRR(R8, Rcx),
      mov_q(Rcx, gui_pixels),
      mov_d(Operand::SibDisp(Sib { base: Rcx, index: R8, scale: S4 }, Disp::Zero), Rax),
      IncMd(pixel_x),
      Jmp(while_x),
      Lbl(while_end_x),
      IncMd(pixel_y),
      Jmp(while_y),
      Lbl(while_end_y),
      mov_q(Rcx, hwnd),
      LeaRM(Rdx, paint_struct),
      CallApiCheck(self.api(USER32, "BeginPaint")),
      mov_q(hdc, Rax),
      mov_d(R9, ps_top),
      mov_d(Rcx, ps_bottom),
      SubRR(R9, Rcx),
      mov_d(R8, ps_right),
      mov_d(Rcx, ps_left),
      SubRR(R8, Rcx),
      mov_q(Args(5), R8),
      mov_q(Rcx, hdc),
      Clear(Rdx),
      Clear(R8),
      mov_q(Args(6), Rdx),
      mov_q(Args(7), Rdx),
      mov_d(Rax, GUI_W),
      mov_q(Args(8), Rax),
      mov_d(Rax, GUI_H),
      mov_q(Args(9), Rax),
      mov_q(Rax, gui_pixels),
      mov_q(Args(10), Rax),
      LeaRM(Rax, bm_info),
      mov_q(Args(11), Rax),
      mov_q(Args(12), Rdx),
      mov_q(Rax, 0xCC_0020),
      mov_q(Args(13), Rax),
      CallApi(self.api(GDI32, "StretchDIBits")),
      Call(ignore),
      mov_q(Rcx, hwnd),
      LeaRM(Rdx, paint_struct),
      CallApi(self.api(USER32, "EndPaint")),
      Clear(Rax),
      Lbl(epilogue),
    ];
    self.use_function(caller, id);
    self.link_function(id, insts, SIZE);
    let ignore_insts = &[
      LogicRR(Test, Rax, Rax),
      JCc(Ne, ignore_epilogue),
      CallApi(self.api(KERNEL32, "GetLastError")),
      mov_d(Rcx, 5),
      LogicRR(Cmp, Rax, Rcx),
      JCc(Ne, self.handlers.win),
      Lbl(ignore_epilogue),
    ];
    self.use_function(id, ignore);
    self.link_function(ignore, ignore_insts, IGNORE_SIZE);
    Ok(id)
  }
}
