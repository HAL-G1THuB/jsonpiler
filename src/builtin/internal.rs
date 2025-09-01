use crate::{ConditionCode::*, Inst::*, Jsonpiler, OpD::*, OpQ::*, Reg::*, VarKind::*};
use std::collections::hash_map::Entry::{Occupied, Vacant};
impl Jsonpiler {
  pub(crate) fn get_custom_error(&mut self, err_msg: &'static str) -> usize {
    let err_msg_id = self.global_str(err_msg.to_owned());
    let message_box = self.import(Jsonpiler::USER32, "MessageBoxA", 0x285);
    let mb_a = self.call_api_check_null(message_box);
    let exit_process = self.import(Jsonpiler::KERNEL32, "ExitProcess", 0x167);
    match self.sym_table.entry(err_msg) {
      Occupied(entry) => *entry.get(),
      Vacant(entry) => {
        let id = self.label_id;
        self.label_id += 1;
        self.insts.extend_from_slice(&[
          Lbl(id),
          Clear(Rcx),
          LeaRM(Rdx, Global { id: err_msg_id, disp: 8i32 }),
          Clear(R8),
          MovRId(R9, 0x10),
        ]);
        self.insts.extend_from_slice(&mb_a);
        self.insts.extend_from_slice(&[MovRId(Rcx, u32::MAX), CallApi(exit_process)]);
        entry.insert(id);
        id
      }
    }
  }
  pub(crate) fn get_msg_box(&mut self) -> usize {
    let heap = Global { id: self.sym_table["HEAP"], disp: 0i32 };
    let u8_to_16 = self.get_u8_to_16();
    let heap_free = self.import(Jsonpiler::KERNEL32, "HeapFree", 0x357);
    let message_box_w = self.import(Jsonpiler::USER32, "MessageBoxW", 0x28c);
    let msg_box_insts = self.call_api_check_null(message_box_w);
    let heap_free_insts = self.call_api_check_null(heap_free);
    match self.sym_table.entry("MSG_BOX") {
      Occupied(entry) => *entry.get(),
      Vacant(entry) => {
        let id = self.label_id;
        self.label_id += 1;
        self.insts.extend_from_slice(&[
          Lbl(id),
          Push(Rdi),
          Push(Rsi),
          Push(Rbp),
          MovQQ(Rq(Rbp), Rq(Rsp)),
          SubRId(Rsp, 0x20),
          MovQQ(Rq(Rsi), Rq(Rdx)),
          Call(u8_to_16),
          MovQQ(Rq(Rdi), Rq(Rax)),
          MovQQ(Rq(Rcx), Rq(Rsi)),
          Call(u8_to_16),
          MovQQ(Rq(Rsi), Rq(Rax)),
          Clear(Rcx),
          MovQQ(Rq(Rdx), Rq(Rsi)),
          MovQQ(Rq(R8), Rq(Rdi)),
          Clear(R9),
        ]);
        self.insts.extend_from_slice(&msg_box_insts);
        self.insts.extend_from_slice(&[
          MovQQ(Rq(Rcx), Mq(heap)),
          Clear(Rdx),
          MovQQ(Rq(R8), Rq(Rdi)),
        ]);
        self.insts.extend_from_slice(&heap_free_insts);
        self.insts.extend_from_slice(&[
          MovQQ(Rq(Rcx), Mq(heap)),
          Clear(Rdx),
          MovQQ(Rq(R8), Rq(Rsi)),
        ]);
        self.insts.extend_from_slice(&heap_free_insts);
        self.insts.extend_from_slice(&[MovQQ(Rq(Rsp), Rq(Rbp)), Pop(Rbp), Pop(Rsi), Pop(Rdi), Ret]);
        entry.insert(id);
        id
      }
    }
  }
  pub(crate) fn get_random(&mut self) -> usize {
    let random_seed = Global { id: self.sym_table["RANDOM_SEED"], disp: 0i32 };
    match self.sym_table.entry("RANDOM") {
      Occupied(entry) => *entry.get(),
      Vacant(entry) => {
        let id = self.label_id;
        self.label_id += 1;
        self.insts.extend_from_slice(&[
          Lbl(id),
          Push(Rbp),
          MovQQ(Rq(Rbp), Rq(Rsp)),
          SubRId(Rsp, 8),
          MovQQ(Rq(Rax), Mq(random_seed)),
          MovQQ(Rq(Rcx), Rq(Rax)),
          ShlRIb(Rcx, 7),
          XorRR(Rax, Rcx),
          MovQQ(Rq(Rcx), Rq(Rax)),
          ShrRIb(Rcx, 9),
          XorRR(Rax, Rcx),
          MovQQ(Rq(Rcx), Rq(Rax)),
          ShlRIb(Rcx, 13),
          XorRR(Rax, Rcx),
          MovQQ(Mq(random_seed), Rq(Rax)),
          MovQQ(Rq(Rsp), Rq(Rbp)),
          Pop(Rbp),
          Ret,
        ]);
        entry.insert(id);
        id
      }
    }
  }
  pub(crate) fn get_u8_to_16(&mut self) -> usize {
    let heap = self.sym_table["HEAP"];
    let multi_byte_to_wide_char = self.import(Jsonpiler::KERNEL32, "MultiByteToWideChar", 0x3F8);
    let mb_t_wc = self.call_api_check_null(multi_byte_to_wide_char);
    let heap_alloc = self.import(Jsonpiler::KERNEL32, "HeapAlloc", 0x353);
    let h_a = self.call_api_check_null(heap_alloc);
    match self.sym_table.entry("U8TO16") {
      Occupied(entry) => *entry.get(),
      Vacant(entry) => {
        let id = self.label_id;
        self.label_id += 1;
        self.insts.extend_from_slice(&[
          Lbl(id),
          Push(Rdi),
          Push(Rsi),
          Push(Rbx),
          Push(Rbp),
          MovQQ(Rq(Rbp), Rq(Rsp)),
          SubRId(Rsp, 0x48),
          MovQQ(Rq(Rdi), Rq(Rcx)),
          MovRId(Rcx, 65001),
          Clear(Rdx),
          MovQQ(Rq(R8), Rq(Rdi)),
          MovRId(R9, u32::MAX),
          Clear(Rax),
          MovQQ(Args(0x20), Rq(Rax)),
          MovQQ(Args(0x28), Rq(Rax)),
        ]);
        self.insts.extend_from_slice(&mb_t_wc);
        self.insts.extend_from_slice(&[
          Shl1R(Rax),
          MovQQ(Rq(Rsi), Rq(Rax)),
          MovQQ(Rq(Rcx), Mq(Global { id: heap, disp: 0i32 })),
          Clear(Rdx),
          MovQQ(Rq(R8), Rq(Rsi)),
        ]);
        self.insts.extend_from_slice(&h_a);
        self.insts.extend_from_slice(&[
          MovQQ(Rq(Rbx), Rq(Rax)),
          MovRId(Rcx, 65001),
          Clear(Rdx),
          MovQQ(Rq(R8), Rq(Rdi)),
          MovRId(R9, u32::MAX),
          MovQQ(Args(0x20), Rq(Rbx)),
          MovQQ(Args(0x28), Rq(Rsi)),
        ]);
        self.insts.extend_from_slice(&mb_t_wc);
        self.insts.extend_from_slice(&[
          MovQQ(Rq(Rax), Rq(Rbx)),
          MovQQ(Rq(Rsp), Rq(Rbp)),
          Pop(Rbp),
          Pop(Rbx),
          Pop(Rsi),
          Pop(Rdi),
          Ret,
        ]);
        entry.insert(id);
        id
      }
    }
  }
  #[expect(clippy::too_many_lines)]
  pub(crate) fn get_wnd_proc(&mut self, pixel_func: usize) -> usize {
    let def_window_proc = self.import(Jsonpiler::USER32, "DefWindowProcW", 0xA7);
    let post_quit_message = self.import(Jsonpiler::USER32, "PostQuitMessage", 0x2B0);
    let set_timer = self.import(Jsonpiler::USER32, "SetTimer", 0x36B);
    let kill_timer = self.import(Jsonpiler::USER32, "KillTimer", 0x250);
    let begin_paint = self.import(Jsonpiler::USER32, "BeginPaint", 0x11);
    let end_paint = self.import(Jsonpiler::USER32, "EndPaint", 0xF4);
    let heap_alloc = self.import(Jsonpiler::KERNEL32, "HeapAlloc", 0x353);
    let heap_free = self.import(Jsonpiler::KERNEL32, "HeapFree", 0x357);
    let invalidate_rect = self.import(Jsonpiler::USER32, "InvalidateRect", 0x222);
    let get_cursor_pos = self.import(Jsonpiler::USER32, "GetCursorPos", 0x141);
    let screen_to_client = self.import(Jsonpiler::USER32, "ScreenToClient", 0x309);
    let get_client_rect = self.import(Jsonpiler::USER32, "GetClientRect", 0x133);
    let stretch_di_bits = self.import(Jsonpiler::GDI32, "StretchDIBits", 0x3A4);
    let gui_frame = self.get_bss_id(8, 8);
    self.sym_table.insert("GUI_TICK", gui_frame);
    let hwnd = self.get_bss_id(8, 8);
    self.sym_table.insert("GUI_HWND", hwnd);
    let heap = self.sym_table["HEAP"];
    let gui_pixels = self.get_bss_id(8, 8);
    let bm_info = self.get_bss_id(44, 8);
    let paint_struct = self.get_bss_id(72, 8);
    let hdc = self.get_bss_id(8, 8);
    let wnd_proc = self.gen_id();
    let handle_wm_destroy = self.gen_id();
    let handle_wm_timer = self.gen_id();
    let handle_wm_paint = self.gen_id();
    let handle_wm_create = self.gen_id();
    let end_wnd_proc = self.gen_id();
    let idiv_zero_w = self.gen_id();
    let idiv_zero_h = self.gen_id();
    let idiv_end_w = self.gen_id();
    let idiv_end_h = self.gen_id();
    let while_x = self.gen_id();
    let while_y = self.gen_id();
    let while_end_x = self.gen_id();
    let while_end_y = self.gen_id();
    self.insts.extend_from_slice(&[
      Lbl(wnd_proc),
      Push(Rbp),
      MovQQ(Rq(Rbp), Rq(Rsp)),
      SubRId(Rsp, 0x90),
      MovQQ(Mq(Global { id: hwnd, disp: 0i32 }), Rq(Rcx)),
      CmpRIb(Rdx, 0x2),
      Jcc(E, handle_wm_destroy),
      CmpRIb(Rdx, 0xf),
      Jcc(E, handle_wm_paint),
      CmpRIb(Rdx, 0x1),
      Jcc(E, handle_wm_create),
      MovRId(Rax, 0x113),
      CmpRR(Rdx, Rax),
      Jcc(E, handle_wm_timer),
      CallApi(def_window_proc),
      Jmp(end_wnd_proc),
      Lbl(handle_wm_destroy),
      MovQQ(Rq(Rdx), Iq(1)),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(kill_timer));
    self.insts.extend_from_slice(&[
      MovQQ(Rq(Rcx), Mq(Global { id: heap, disp: 0i32 })),
      Clear(Rdx),
      MovQQ(Rq(R8), Mq(Global { id: gui_pixels, disp: 0i32 })),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(heap_free));
    self.insts.extend_from_slice(&[
      Clear(Rcx),
      CallApi(post_quit_message),
      Clear(Rax),
      Jmp(end_wnd_proc),
      Lbl(handle_wm_create),
      MovRId(Rdx, 1),
      MovRId(R8, 125),
      Clear(R9),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(set_timer));
    self.insts.extend_from_slice(&[
      MovQQ(Rq(Rax), Iq(0x28 | (u64::from(Jsonpiler::GUI_W) << 32))),
      MovQQ(Mq(Global { id: bm_info, disp: 0i32 }), Rq(Rax)),
      MovQQ(Rq(Rax), Iq((0x0020_0001 << 32) | u64::from(Jsonpiler::GUI_H))),
      MovQQ(Mq(Global { id: bm_info, disp: 8i32 }), Rq(Rax)),
    ]);
    self.insts.extend_from_slice(&[
      MovQQ(Rq(Rcx), Mq(Global { id: heap, disp: 0i32 })),
      MovRId(Rdx, 8),
      MovQQ(Rq(R8), Iq(u64::from(Jsonpiler::GUI_W) * u64::from(Jsonpiler::GUI_H) * 4)),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(heap_alloc));
    self.insts.extend_from_slice(&[
      MovQQ(Mq(Global { id: gui_pixels, disp: 0i32 }), Rq(Rax)),
      Clear(Rax),
      Jmp(end_wnd_proc),
      Lbl(handle_wm_paint),
      LeaRM(Rcx, Local { offset: 16i32 }),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(get_cursor_pos));
    self.insts.extend_from_slice(&[
      MovQQ(Rq(Rcx), Mq(Global { id: hwnd, disp: 0i32 })),
      LeaRM(Rdx, Local { offset: 0x10i32 }),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(screen_to_client));
    self.insts.extend_from_slice(&[
      MovQQ(Rq(Rcx), Mq(Global { id: hwnd, disp: 0i32 })),
      LeaRM(Rdx, Local { offset: 0x20i32 }),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(get_client_rect));
    self.insts.extend_from_slice(&[
      MovDD(Rd(Rax), Md(Local { offset: 0x10i32 })),
      MovDD(Rd(Rcx), Id(Jsonpiler::GUI_W)),
      IMulRR(Rax, Rcx),
      MovDD(Rd(Rcx), Md(Local { offset: 0x18i32 })),
      MovDD(Rd(Rdx), Md(Local { offset: 0x20i32 })),
      SubRR(Rcx, Rdx),
      TestRR(Rcx, Rcx),
      Jcc(E, idiv_zero_w),
      Custom(Jsonpiler::CQO.to_vec()),
      IDivR(Rcx),
      Jmp(idiv_end_w),
      Lbl(idiv_zero_w),
      Clear(Rax),
      Lbl(idiv_end_w),
      MovDD(Md(Local { offset: 0x10i32 }), Rd(Rax)),
      MovDD(Rd(Rax), Md(Local { offset: 0xCi32 })),
      MovDD(Rd(Rcx), Id(Jsonpiler::GUI_H)),
      IMulRR(Rax, Rcx),
      MovDD(Rd(Rcx), Md(Local { offset: 0x14i32 })),
      MovDD(Rd(Rdx), Md(Local { offset: 0x1Ci32 })),
      SubRR(Rcx, Rdx),
      TestRR(Rcx, Rcx),
      Jcc(E, idiv_zero_h),
      Custom(Jsonpiler::CQO.to_vec()),
      IDivR(Rcx),
      Jmp(idiv_end_h),
      Lbl(idiv_zero_h),
      Clear(Rax),
      Lbl(idiv_end_h),
      MovDD(Md(Local { offset: 0xCi32 }), Rd(Rax)),
      MovDD(Md(Local { offset: 4i32 }), Id(0)),
      Lbl(while_y),
      MovDD(Rd(Rcx), Md(Local { offset: 4i32 })),
      MovDD(Rd(Rdx), Id(Jsonpiler::GUI_H)),
      CmpRR(Rcx, Rdx),
      Jcc(E, while_end_y),
      MovDD(Md(Local { offset: 8i32 }), Id(0)),
      Lbl(while_x),
      MovDD(Rd(Rcx), Md(Local { offset: 8i32 })),
      MovDD(Rd(Rdx), Id(Jsonpiler::GUI_W)),
      CmpRR(Rcx, Rdx),
      Jcc(E, while_end_x),
      MovDD(Rd(Rcx), Md(Local { offset: 8i32 })),
      SubRId(Rcx, Jsonpiler::GUI_W >> 1),
      MovDD(Rd(Rdx), Md(Local { offset: 4i32 })),
      SubRId(Rdx, Jsonpiler::GUI_H >> 1),
      MovQQ(Rq(R8), Mq(Global { id: gui_frame, disp: 0i32 })),
      MovDD(Rd(Rax), Id(Jsonpiler::GUI_H >> 1)),
      MovDD(Rd(R9), Md(Local { offset: 0xCi32 })),
      SubRR(Rax, R9),
      MovQQ(Args(0x20), Rq(Rax)),
      MovDD(Rd(R9), Md(Local { offset: 0x10i32 })),
      SubRId(R9, Jsonpiler::GUI_W >> 1),
      Call(pixel_func),
      MovDD(Rd(Rcx), Md(Local { offset: 4i32 })),
      MovRId(Rdx, Jsonpiler::GUI_W),
      IMulRR(Rcx, Rdx),
      MovDD(Rd(R8), Md(Local { offset: 8i32 })),
      AddRR(R8, Rcx),
      MovRId(Rdx, 4),
      IMulRR(R8, Rdx),
      MovQQ(Rq(Rcx), Mq(Global { id: gui_pixels, disp: 0i32 })),
      AddRR(Rcx, R8),
      MovDD(RefD(Rcx), Rd(Rax)),
      MovDD(Rd(Rcx), Md(Local { offset: 8i32 })),
      IncR(Rcx),
      MovDD(Md(Local { offset: 8i32 }), Rd(Rcx)),
      Jmp(while_x),
      Lbl(while_end_x),
      MovDD(Rd(Rcx), Md(Local { offset: 4i32 })),
      IncR(Rcx),
      MovDD(Md(Local { offset: 4i32 }), Rd(Rcx)),
      Jmp(while_y),
      Lbl(while_end_y),
      MovQQ(Rq(Rcx), Mq(Global { id: hwnd, disp: 0i32 })),
      LeaRM(Rdx, Global { id: paint_struct, disp: 0i32 }),
      CallApi(begin_paint),
      MovQQ(Mq(Global { id: hdc, disp: 0i32 }), Rq(Rax)),
      MovDD(Rd(R9), Md(Global { id: paint_struct, disp: 0x14i32 })),
      MovDD(Rd(Rcx), Md(Global { id: paint_struct, disp: 0xCi32 })),
      SubRR(R9, Rcx),
      MovDD(Rd(R8), Md(Global { id: paint_struct, disp: 0x18i32 })),
      MovDD(Rd(Rcx), Md(Global { id: paint_struct, disp: 0x10i32 })),
      SubRR(R8, Rcx),
      MovQQ(Args(0x20), Rq(R8)),
      MovQQ(Rq(Rcx), Mq(Global { id: hdc, disp: 0i32 })),
      Clear(Rdx),
      Clear(R8),
      MovQQ(Args(0x28), Rq(Rdx)),
      MovQQ(Args(0x30), Rq(Rdx)),
      MovRId(Rax, Jsonpiler::GUI_W),
      MovQQ(Args(0x38), Rq(Rax)),
      MovRId(Rax, Jsonpiler::GUI_H),
      MovQQ(Args(0x40), Rq(Rax)),
      MovQQ(Rq(Rax), Mq(Global { id: gui_pixels, disp: 0i32 })),
      MovQQ(Args(0x48), Rq(Rax)),
      LeaRM(Rax, Global { id: bm_info, disp: 0i32 }),
      MovQQ(Args(0x50), Rq(Rax)),
      MovQQ(Args(0x58), Rq(Rdx)),
      MovQQ(Rq(Rax), Iq(0xCC_0020)),
      MovQQ(Args(0x60), Rq(Rax)),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(stretch_di_bits));
    self.insts.extend_from_slice(&[
      MovQQ(Rq(Rcx), Mq(Global { id: hwnd, disp: 0i32 })),
      LeaRM(Rdx, Global { id: paint_struct, disp: 0i32 }),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(end_paint));
    self.insts.extend_from_slice(&[
      Clear(Rax),
      MovQQ(Mq(Global { id: hdc, disp: 0i32 }), Rq(Rax)),
      Jmp(end_wnd_proc),
      Lbl(handle_wm_timer),
      Clear(Rdx),
      Clear(R8),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(invalidate_rect));
    self.insts.extend_from_slice(&[
      Clear(Rax),
      MovQQ(Rq(Rcx), Mq(Global { id: gui_frame, disp: 0i32 })),
      IncR(Rcx),
      MovQQ(Mq(Global { id: gui_frame, disp: 0i32 }), Rq(Rcx)),
      Lbl(end_wnd_proc),
      MovQQ(Rq(Rsp), Rq(Rbp)),
      Pop(Rbp),
      Ret,
    ]);
    self.sym_table.insert("WND_PROC", wnd_proc);
    wnd_proc
  }
}
