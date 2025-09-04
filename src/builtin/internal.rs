use crate::{
  ConditionCode::*,
  ErrOR,
  Inst::*,
  Jsonpiler,
  LogicByteOpcode::*,
  Operand::{Args, Ref},
  Register::*,
  VarKind::*,
  utility::{mov_d, mov_q},
};
use std::collections::hash_map::Entry::{Occupied, Vacant};
impl Jsonpiler {
  pub(crate) fn get_critical_section(&mut self) -> ErrOR<u32> {
    if let Some(id) = self.sym_table.get("CRITICAL_SECTION") {
      return Ok(*id);
    }
    let initialize_critical_section =
      self.import(Jsonpiler::KERNEL32, "InitializeCriticalSection")?;
    let critical_section = self.get_bss_id(40, 8);
    self.sym_table.insert("CRITICAL_SECTION", critical_section);
    self.startup.extend_from_slice(&[
      LeaRM(Rcx, Global { id: critical_section, disp: 0i32 }),
      CallApi(initialize_critical_section),
    ]);
    self.startup.extend_from_slice(&[
      LeaRM(Rcx, Global { id: critical_section, disp: 0i32 }),
      CallApi(initialize_critical_section),
    ]);
    Ok(critical_section)
  }
  pub(crate) fn get_custom_error(&mut self, err_msg: &'static str) -> ErrOR<u32> {
    let err_msg_id = self.global_str(err_msg.to_owned()).0;
    let message_box = self.import(Jsonpiler::USER32, "MessageBoxA")?;
    let mb_a = self.call_api_check_null(message_box);
    let exit_process = self.import(Jsonpiler::KERNEL32, "ExitProcess")?;
    match self.sym_table.entry(err_msg) {
      Occupied(entry) => Ok(*entry.get()),
      Vacant(entry) => {
        let id = self.label_id;
        self.label_id += 1;
        self.insts.extend_from_slice(&[
          Lbl(id),
          Clear(Rcx),
          LeaRM(Rdx, Global { id: err_msg_id, disp: 0i32 }),
          Clear(R8),
          mov_d(R9, 0x10),
        ]);
        self.insts.extend_from_slice(&mb_a);
        self.insts.extend_from_slice(&[mov_d(Rcx, u32::MAX), CallApi(exit_process)]);
        entry.insert(id);
        Ok(id)
      }
    }
  }
  pub(crate) fn get_msg_box(&mut self) -> ErrOR<u32> {
    let heap = Global { id: self.sym_table["HEAP"], disp: 0i32 };
    let u8_to_16 = self.get_u8_to_16()?;
    let heap_free = self.import(Jsonpiler::KERNEL32, "HeapFree")?;
    let message_box_w = self.import(Jsonpiler::USER32, "MessageBoxW")?;
    let msg_box_insts = self.call_api_check_null(message_box_w);
    let heap_free_insts = self.call_api_check_null(heap_free);
    match self.sym_table.entry("MSG_BOX") {
      Occupied(entry) => Ok(*entry.get()),
      Vacant(entry) => {
        let id = self.label_id;
        self.label_id += 1;
        self.insts.extend_from_slice(&[
          Lbl(id),
          Push(Rdi),
          Push(Rsi),
          Push(Rbp),
          mov_q(Rbp, Rsp),
          SubRId(Rsp, 0x20),
          mov_q(Rsi, Rdx),
          Call(u8_to_16),
          mov_q(Rdi, Rax),
          mov_q(Rcx, Rsi),
          Call(u8_to_16),
          mov_q(Rsi, Rax),
          Clear(Rcx),
          mov_q(Rdx, Rsi),
          mov_q(R8, Rdi),
          Clear(R9),
        ]);
        self.insts.extend_from_slice(&msg_box_insts);
        self.insts.extend_from_slice(&[mov_q(Rcx, heap), Clear(Rdx), mov_q(R8, Rdi)]);
        self.insts.extend_from_slice(&heap_free_insts);
        self.insts.extend_from_slice(&[mov_q(Rcx, heap), Clear(Rdx), mov_q(R8, Rsi)]);
        self.insts.extend_from_slice(&heap_free_insts);
        self.insts.extend_from_slice(&[
          mov_q(Rsp, Rbp),
          Pop(Rbp),
          Pop(Rsi),
          Pop(Rdi),
          Custom(&Jsonpiler::RET),
        ]);
        entry.insert(id);
        Ok(id)
      }
    }
  }
  pub(crate) fn get_random(&mut self) -> ErrOR<u32> {
    if let Some(id) = self.sym_table.get("RANDOM") {
      return Ok(*id);
    }
    let query_perf_cnt = self.import(Jsonpiler::KERNEL32, "QueryPerformanceCounter")?;
    let random_seed = self.get_bss_id(8, 8);
    let id = self.gen_id();
    self.startup.push(LeaRM(Rcx, Global { id: random_seed, disp: 0i32 }));
    self.startup.extend_from_slice(&self.call_api_check_null(query_perf_cnt));
    self.startup.push(Call(id));
    self.sym_table.insert("RANDOM", id);
    self.insts.extend_from_slice(&[
      Lbl(id),
      Push(Rbp),
      mov_q(Rbp, Rsp),
      SubRId(Rsp, 8),
      mov_q(Rax, Global { id: random_seed, disp: 0i32 }),
      mov_q(Rcx, Rax),
      ShlRIb(Rcx, 7),
      LogicRR(Xor, Rax, Rcx),
      mov_q(Rcx, Rax),
      ShrRIb(Rcx, 9),
      LogicRR(Xor, Rax, Rcx),
      mov_q(Rcx, Rax),
      ShlRIb(Rcx, 13),
      LogicRR(Xor, Rax, Rcx),
      mov_q(Global { id: random_seed, disp: 0i32 }, Rax),
      mov_q(Rsp, Rbp),
      Pop(Rbp),
      Custom(&Jsonpiler::RET),
    ]);
    Ok(id)
  }
  pub(crate) fn get_u8_to_16(&mut self) -> ErrOR<u32> {
    let heap = self.sym_table["HEAP"];
    let multi_byte_to_wide_char = self.import(Jsonpiler::KERNEL32, "MultiByteToWideChar")?;
    let mb_t_wc = self.call_api_check_null(multi_byte_to_wide_char);
    let heap_alloc = self.import(Jsonpiler::KERNEL32, "HeapAlloc")?;
    let h_a = self.call_api_check_null(heap_alloc);
    match self.sym_table.entry("U8TO16") {
      Occupied(entry) => Ok(*entry.get()),
      Vacant(entry) => {
        let id = self.label_id;
        self.label_id += 1;
        self.insts.extend_from_slice(&[
          Lbl(id),
          Push(Rdi),
          Push(Rsi),
          Push(Rbx),
          Push(Rbp),
          mov_q(Rbp, Rsp),
          SubRId(Rsp, 0x48),
          mov_q(Rdi, Rcx),
          mov_d(Rcx, 65001),
          Clear(Rdx),
          mov_q(R8, Rdi),
          mov_d(R9, u32::MAX),
          Clear(Rax),
          mov_q(Args(0x20), Rax),
          mov_q(Args(0x28), Rax),
        ]);
        self.insts.extend_from_slice(&mb_t_wc);
        self.insts.extend_from_slice(&[
          Shl1R(Rax),
          mov_q(Rsi, Rax),
          mov_q(Rcx, Global { id: heap, disp: 0i32 }),
          Clear(Rdx),
          mov_q(R8, Rsi),
        ]);
        self.insts.extend_from_slice(&h_a);
        self.insts.extend_from_slice(&[
          mov_q(Rbx, Rax),
          mov_d(Rcx, 65001),
          Clear(Rdx),
          mov_q(R8, Rdi),
          mov_d(R9, u32::MAX),
          mov_q(Args(0x20), Rbx),
          mov_q(Args(0x28), Rsi),
        ]);
        self.insts.extend_from_slice(&mb_t_wc);
        self.insts.extend_from_slice(&[
          mov_q(Rax, Rbx),
          mov_q(Rsp, Rbp),
          Pop(Rbp),
          Pop(Rbx),
          Pop(Rsi),
          Pop(Rdi),
          Custom(&Jsonpiler::RET),
        ]);
        entry.insert(id);
        Ok(id)
      }
    }
  }
  #[expect(clippy::too_many_lines)]
  pub(crate) fn get_wnd_proc(&mut self, pixel_func: u32) -> ErrOR<u32> {
    let def_window_proc = self.import(Jsonpiler::USER32, "DefWindowProcW")?;
    let post_quit_message = self.import(Jsonpiler::USER32, "PostQuitMessage")?;
    let set_timer = self.import(Jsonpiler::USER32, "SetTimer")?;
    let kill_timer = self.import(Jsonpiler::USER32, "KillTimer")?;
    let begin_paint = self.import(Jsonpiler::USER32, "BeginPaint")?;
    let end_paint = self.import(Jsonpiler::USER32, "EndPaint")?;
    let heap_alloc = self.import(Jsonpiler::KERNEL32, "HeapAlloc")?;
    let heap_free = self.import(Jsonpiler::KERNEL32, "HeapFree")?;
    let invalidate_rect = self.import(Jsonpiler::USER32, "InvalidateRect")?;
    let get_cursor_pos = self.import(Jsonpiler::USER32, "GetCursorPos")?;
    let screen_to_client = self.import(Jsonpiler::USER32, "ScreenToClient")?;
    let get_client_rect = self.import(Jsonpiler::USER32, "GetClientRect")?;
    let stretch_di_bits = self.import(Jsonpiler::GDI32, "StretchDIBits")?;
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
      mov_q(Rbp, Rsp),
      SubRId(Rsp, 0x90),
      mov_q(Global { id: hwnd, disp: 0i32 }, Rcx),
      CmpRIb(Rdx, 0x2),
      Jcc(E, handle_wm_destroy),
      CmpRIb(Rdx, 0xf),
      Jcc(E, handle_wm_paint),
      CmpRIb(Rdx, 0x1),
      Jcc(E, handle_wm_create),
      mov_d(Rax, 0x113),
      LogicRR(Cmp, Rdx, Rax),
      Jcc(E, handle_wm_timer),
      CallApi(def_window_proc),
      Jmp(end_wnd_proc),
      Lbl(handle_wm_destroy),
      mov_q(Rdx, 1),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(kill_timer));
    self.insts.extend_from_slice(&[
      mov_q(Rcx, Global { id: heap, disp: 0i32 }),
      Clear(Rdx),
      mov_q(R8, Global { id: gui_pixels, disp: 0i32 }),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(heap_free));
    self.insts.extend_from_slice(&[
      Clear(Rcx),
      CallApi(post_quit_message),
      Clear(Rax),
      Jmp(end_wnd_proc),
      Lbl(handle_wm_create),
      mov_d(Rdx, 1),
      mov_d(R8, 100),
      Clear(R9),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(set_timer));
    self.insts.extend_from_slice(&[
      mov_q(Rax, 0x28 | (u64::from(Jsonpiler::GUI_W) << 32u8)),
      mov_q(Global { id: bm_info, disp: 0i32 }, Rax),
      mov_q(Rax, (0x0020_0001 << 32u8) | u64::from(Jsonpiler::GUI_H)),
      mov_q(Global { id: bm_info, disp: 8i32 }, Rax),
    ]);
    self.insts.extend_from_slice(&[
      mov_q(Rcx, Global { id: heap, disp: 0i32 }),
      mov_d(Rdx, 8),
      mov_q(R8, u64::from(Jsonpiler::GUI_W) * u64::from(Jsonpiler::GUI_H) * 4),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(heap_alloc));
    self.insts.extend_from_slice(&[
      mov_q(Global { id: gui_pixels, disp: 0i32 }, Rax),
      Clear(Rax),
      Jmp(end_wnd_proc),
      Lbl(handle_wm_paint),
      LeaRM(Rcx, Local { offset: 16i32 }),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(get_cursor_pos));
    self.insts.extend_from_slice(&[
      mov_q(Rcx, Global { id: hwnd, disp: 0i32 }),
      LeaRM(Rdx, Local { offset: 0x10i32 }),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(screen_to_client));
    self.insts.extend_from_slice(&[
      mov_q(Rcx, Global { id: hwnd, disp: 0i32 }),
      LeaRM(Rdx, Local { offset: 0x20i32 }),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(get_client_rect));
    self.insts.extend_from_slice(&[
      mov_d(Rax, Local { offset: 0x10i32 }),
      mov_d(Rcx, Jsonpiler::GUI_W),
      IMulRR(Rax, Rcx),
      mov_d(Rcx, Local { offset: 0x18i32 }),
      mov_d(Rdx, Local { offset: 0x20i32 }),
      SubRR(Rcx, Rdx),
      LogicRR(Test, Rcx, Rcx),
      Jcc(E, idiv_zero_w),
      Custom(&Jsonpiler::CQO),
      IDivR(Rcx),
      Jmp(idiv_end_w),
      Lbl(idiv_zero_w),
      Clear(Rax),
      Lbl(idiv_end_w),
      mov_d(Local { offset: 0x10i32 }, Rax),
      mov_d(Rax, Local { offset: 0xCi32 }),
      mov_d(Rcx, Jsonpiler::GUI_H),
      IMulRR(Rax, Rcx),
      mov_d(Rcx, Local { offset: 0x14i32 }),
      mov_d(Rdx, Local { offset: 0x1Ci32 }),
      SubRR(Rcx, Rdx),
      LogicRR(Test, Rcx, Rcx),
      Jcc(E, idiv_zero_h),
      Custom(&Jsonpiler::CQO),
      IDivR(Rcx),
      Jmp(idiv_end_h),
      Lbl(idiv_zero_h),
      Clear(Rax),
      Lbl(idiv_end_h),
      mov_d(Local { offset: 0xCi32 }, Rax),
      mov_d(Local { offset: 4i32 }, 0),
      Lbl(while_y),
      mov_d(Rcx, Local { offset: 4i32 }),
      mov_d(Rdx, Jsonpiler::GUI_H),
      LogicRR(Cmp, Rcx, Rdx),
      Jcc(E, while_end_y),
      mov_d(Local { offset: 8i32 }, 0),
      Lbl(while_x),
      mov_d(Rcx, Local { offset: 8i32 }),
      mov_d(Rdx, Jsonpiler::GUI_W),
      LogicRR(Cmp, Rcx, Rdx),
      Jcc(E, while_end_x),
      mov_d(Rcx, Local { offset: 8i32 }),
      SubRId(Rcx, Jsonpiler::GUI_W >> 1),
      mov_d(Rdx, Local { offset: 4i32 }),
      SubRId(Rdx, Jsonpiler::GUI_H >> 1),
      mov_q(R8, Global { id: gui_frame, disp: 0i32 }),
      mov_d(Rax, Jsonpiler::GUI_H >> 1u8),
      mov_d(R9, Local { offset: 0xCi32 }),
      SubRR(Rax, R9),
      mov_q(Args(0x20), Rax),
      mov_d(R9, Local { offset: 0x10i32 }),
      SubRId(R9, Jsonpiler::GUI_W >> 1),
      Call(pixel_func),
      mov_d(Rcx, Local { offset: 4i32 }),
      mov_d(Rdx, Jsonpiler::GUI_W),
      IMulRR(Rcx, Rdx),
      mov_d(R8, Local { offset: 8i32 }),
      AddRR(R8, Rcx),
      mov_d(Rdx, 4),
      IMulRR(R8, Rdx),
      mov_q(Rcx, Global { id: gui_pixels, disp: 0i32 }),
      AddRR(Rcx, R8),
      mov_d(Ref(Rcx), Rax),
      mov_d(Rcx, Local { offset: 8i32 }),
      IncR(Rcx),
      mov_d(Local { offset: 8i32 }, Rcx),
      Jmp(while_x),
      Lbl(while_end_x),
      mov_d(Rcx, Local { offset: 4i32 }),
      IncR(Rcx),
      mov_d(Local { offset: 4i32 }, Rcx),
      Jmp(while_y),
      Lbl(while_end_y),
      mov_q(Rcx, Global { id: hwnd, disp: 0i32 }),
      LeaRM(Rdx, Global { id: paint_struct, disp: 0i32 }),
      CallApi(begin_paint),
      mov_q(Global { id: hdc, disp: 0i32 }, Rax),
      mov_d(R9, Global { id: paint_struct, disp: 0x14i32 }),
      mov_d(Rcx, Global { id: paint_struct, disp: 0xCi32 }),
      SubRR(R9, Rcx),
      mov_d(R8, Global { id: paint_struct, disp: 0x18i32 }),
      mov_d(Rcx, Global { id: paint_struct, disp: 0x10i32 }),
      SubRR(R8, Rcx),
      mov_q(Args(0x20), R8),
      mov_q(Rcx, Global { id: hdc, disp: 0i32 }),
      Clear(Rdx),
      Clear(R8),
      mov_q(Args(0x28), Rdx),
      mov_q(Args(0x30), Rdx),
      mov_d(Rax, Jsonpiler::GUI_W),
      mov_q(Args(0x38), Rax),
      mov_d(Rax, Jsonpiler::GUI_H),
      mov_q(Args(0x40), Rax),
      mov_q(Rax, Global { id: gui_pixels, disp: 0i32 }),
      mov_q(Args(0x48), Rax),
      LeaRM(Rax, Global { id: bm_info, disp: 0i32 }),
      mov_q(Args(0x50), Rax),
      mov_q(Args(0x58), Rdx),
      mov_q(Rax, 0xCC_0020),
      mov_q(Args(0x60), Rax),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(stretch_di_bits));
    self.insts.extend_from_slice(&[
      mov_q(Rcx, Global { id: hwnd, disp: 0i32 }),
      LeaRM(Rdx, Global { id: paint_struct, disp: 0i32 }),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(end_paint));
    self.insts.extend_from_slice(&[
      Clear(Rax),
      mov_q(Global { id: hdc, disp: 0i32 }, Rax),
      Jmp(end_wnd_proc),
      Lbl(handle_wm_timer),
      Clear(Rdx),
      Clear(R8),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(invalidate_rect));
    self.insts.extend_from_slice(&[
      Clear(Rax),
      mov_q(Rcx, Global { id: gui_frame, disp: 0i32 }),
      IncR(Rcx),
      mov_q(Global { id: gui_frame, disp: 0i32 }, Rcx),
      Lbl(end_wnd_proc),
      mov_q(Rsp, Rbp),
      Pop(Rbp),
      Custom(&Jsonpiler::RET),
    ]);
    self.sym_table.insert("WND_PROC", wnd_proc);
    Ok(wnd_proc)
  }
}
