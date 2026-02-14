use crate::{
  ConditionCode::*,
  DataInst::Seh,
  ErrOR,
  Inst::{self, *},
  Jsonpiler,
  LogicByteOpcode::*,
  Memory::*,
  Operand::{Args, Ref},
  Register::*,
  dll::*,
  utility::{mov_d, mov_q},
};
impl Jsonpiler {
  pub(crate) fn get_critical_section(&mut self) -> ErrOR<u32> {
    if let Some(id) = self.sym_table.get("CRITICAL_SECTION") {
      return Ok(*id);
    }
    let initialize_critical_section = self.import(KERNEL32, "InitializeCriticalSection")?;
    let critical_section = self.get_bss_id(40, 8);
    self.sym_table.insert("CRITICAL_SECTION", critical_section);
    self.startup.extend_from_slice(&[
      LeaRM(Rcx, Global { id: critical_section }),
      CallApi(initialize_critical_section),
    ]);
    Ok(critical_section)
  }
  pub(crate) fn get_custom_error(&mut self, err_msg: &'static str) -> ErrOR<u32> {
    if let Some(id) = self.sym_table.get(err_msg) {
      return Ok(*id);
    }
    let err_msg_id = self.global_str(err_msg.into()).0;
    let message_box = self.import(USER32, "MessageBoxA")?;
    let exit_process = self.import(KERNEL32, "ExitProcess")?;
    let id = self.gen_id();
    self.sym_table.insert(err_msg, id);
    self.insts.extend_from_slice(&[
      Lbl(id),
      Clear(Rcx),
      LeaRM(Rdx, Global { id: err_msg_id }),
      Clear(R8),
      mov_d(R9, 0x10),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(message_box));
    self.insts.extend_from_slice(&[mov_d(Rcx, u32::MAX), CallApi(exit_process)]);
    Ok(id)
  }
  pub(crate) fn get_msg_box(&mut self) -> ErrOR<u32> {
    if let Some(id) = self.sym_table.get("MSG_BOX") {
      return Ok(*id);
    }
    let heap = self.g_symbol("HEAP");
    let u8_to_16 = self.get_u8_to_16()?;
    let heap_free = self.import(KERNEL32, "HeapFree")?;
    let message_box_w = self.import(USER32, "MessageBoxW")?;
    let msg_box_insts = self.call_api_check_null(message_box_w);
    let heap_free_insts = self.call_api_check_null(heap_free);
    let id = self.gen_id();
    let end = self.gen_id();
    self.sym_table.insert("MSG_BOX", id);
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
      Lbl(end),
    ]);
    self.data_insts.extend_from_slice(&[Seh(id, end, 20)]);
    Ok(id)
  }
  pub(crate) fn get_random(&mut self) -> ErrOR<u32> {
    if let Some(id) = self.sym_table.get("RANDOM") {
      return Ok(*id);
    }
    let query_perf_cnt = self.import(KERNEL32, "QueryPerformanceCounter")?;
    let random_seed = Global { id: self.get_bss_id(8, 8) };
    let id = self.gen_id();
    let end = self.gen_id();
    self.startup.push(LeaRM(Rcx, random_seed));
    self.startup.extend_from_slice(&self.call_api_check_null(query_perf_cnt));
    self.startup.push(Call(id));
    self.sym_table.insert("RANDOM", id);
    self.insts.extend_from_slice(&[
      Lbl(id),
      Push(Rbp),
      mov_q(Rbp, Rsp),
      SubRId(Rsp, 8),
      mov_q(Rax, random_seed),
      mov_q(Rcx, Rax),
      ShlRIb(Rcx, 7),
      LogicRR(Xor, Rax, Rcx),
      mov_q(Rcx, Rax),
      ShrRIb(Rcx, 9),
      LogicRR(Xor, Rax, Rcx),
      mov_q(Rcx, Rax),
      ShlRIb(Rcx, 13),
      LogicRR(Xor, Rax, Rcx),
      mov_q(random_seed, Rax),
      mov_q(Rsp, Rbp),
      Pop(Rbp),
      Custom(&Jsonpiler::RET),
      Lbl(end),
    ]);
    self.data_insts.extend_from_slice(&[Seh(id, end, 20)]);
    Ok(id)
  }
  pub(crate) fn get_u8_to_16(&mut self) -> ErrOR<u32> {
    if let Some(id) = self.sym_table.get("U8TO16") {
      return Ok(*id);
    }
    let heap = self.g_symbol("HEAP");
    let multi_byte_to_wide_char = self.import(KERNEL32, "MultiByteToWideChar")?;
    let heap_alloc = self.import(KERNEL32, "HeapAlloc")?;
    let id = self.gen_id();
    let end = self.gen_id();
    self.sym_table.insert("U8TO16", id);
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
    self.insts.extend_from_slice(&self.call_api_check_null(multi_byte_to_wide_char));
    self.insts.extend_from_slice(&[
      Shl1R(Rax),
      mov_q(Rsi, Rax),
      mov_q(Rcx, heap),
      Clear(Rdx),
      mov_q(R8, Rsi),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(heap_alloc));
    self.insts.extend_from_slice(&[
      mov_q(Rbx, Rax),
      mov_d(Rcx, 65001),
      Clear(Rdx),
      mov_q(R8, Rdi),
      mov_d(R9, u32::MAX),
      mov_q(Args(0x20), Rbx),
      mov_q(Args(0x28), Rsi),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(multi_byte_to_wide_char));
    self.insts.extend_from_slice(&[
      mov_q(Rax, Rbx),
      mov_q(Rsp, Rbp),
      Pop(Rbp),
      Pop(Rbx),
      Pop(Rsi),
      Pop(Rdi),
      Custom(&Jsonpiler::RET),
      Lbl(end),
    ]);
    self.data_insts.extend_from_slice(&[Seh(id, end, 20)]);
    Ok(id)
  }
  #[expect(clippy::too_many_lines)]
  pub(crate) fn get_wnd_proc(&mut self, render: u32) -> ErrOR<u32> {
    let def_window_proc = self.import(USER32, "DefWindowProcW")?;
    let post_quit_message = self.import(USER32, "PostQuitMessage")?;
    let set_timer = self.import(USER32, "SetTimer")?;
    let kill_timer = self.import(USER32, "KillTimer")?;
    let begin_paint = self.import(USER32, "BeginPaint")?;
    let end_paint = self.import(USER32, "EndPaint")?;
    let heap_alloc = self.import(KERNEL32, "HeapAlloc")?;
    let heap_free = self.import(KERNEL32, "HeapFree")?;
    let invalidate_rect = self.import(USER32, "InvalidateRect")?;
    let get_cursor_pos = self.import(USER32, "GetCursorPos")?;
    let screen_to_client = self.import(USER32, "ScreenToClient")?;
    let get_client_rect = self.import(USER32, "GetClientRect")?;
    let stretch_di_bits = self.import(GDI32, "StretchDIBits")?;
    let gui_frame = self.get_bss_id(8, 8);
    let hwnd = self.get_bss_id(8, 8);
    let heap = self.g_symbol("HEAP");
    let gui_pixels = self.get_bss_id(8, 8);
    let bm_info = self.get_bss_id(44, 8);
    let paint_struct = self.get_bss_id(72, 8);
    let hdc = self.get_bss_id(8, 8);
    let id = self.gen_id();
    let end = self.gen_id();
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
    let cursor_pos = Local { offset: 0x10i32 };
    let cursor_x = Local { offset: 0x10i32 };
    let cursor_y = Local { offset: 0xCi32 };
    let pixel_x = Local { offset: 0x8i32 };
    let pixel_y = Local { offset: 0x4i32 };
    let client_rect = Local { offset: 0x20i32 };
    let left = Local { offset: 0x20i32 };
    let top = Local { offset: 0x1Ci32 };
    let right = Local { offset: 0x18i32 };
    let bottom = Local { offset: 0x14i32 };
    self.insts.extend_from_slice(&[
      Lbl(id),
      Push(Rbp),
      mov_q(Rbp, Rsp),
      SubRId(Rsp, 0x90),
      mov_q(Global { id: hwnd }, Rcx),
      CmpRIb(Rdx, 0x2),
      JCc(E, handle_wm_destroy),
      CmpRIb(Rdx, 0xf),
      JCc(E, handle_wm_paint),
      CmpRIb(Rdx, 0x1),
      JCc(E, handle_wm_create),
      mov_d(Rax, 0x113),
      LogicRR(Cmp, Rdx, Rax),
      JCc(E, handle_wm_timer),
      CallApi(def_window_proc),
      Jmp(end_wnd_proc),
      Lbl(handle_wm_destroy),
      mov_d(Rdx, 1),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(kill_timer));
    self.insts.extend_from_slice(&[
      mov_q(Rcx, heap),
      Clear(Rdx),
      mov_q(R8, Global { id: gui_pixels }),
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
      mov_q(GlobalD { id: bm_info, disp: 0i32 }, Rax),
      mov_q(Rax, (0x0020_0001 << 32u8) | u64::from(Jsonpiler::GUI_H)),
      mov_q(GlobalD { id: bm_info, disp: 8i32 }, Rax),
    ]);
    self.insts.extend_from_slice(&[
      mov_q(Rcx, heap),
      mov_d(Rdx, 8),
      mov_q(R8, u64::from(Jsonpiler::GUI_W) * u64::from(Jsonpiler::GUI_H) * 4),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(heap_alloc));
    self.insts.extend_from_slice(&[
      mov_q(Global { id: gui_pixels }, Rax),
      Clear(Rax),
      Jmp(end_wnd_proc),
      Lbl(handle_wm_paint),
      LeaRM(Rcx, cursor_pos),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(get_cursor_pos));
    self.insts.extend_from_slice(&[mov_q(Rcx, Global { id: hwnd }), LeaRM(Rdx, cursor_pos)]);
    self.insts.extend_from_slice(&self.call_api_check_null(screen_to_client));
    self.insts.extend_from_slice(&[mov_q(Rcx, Global { id: hwnd }), LeaRM(Rdx, client_rect)]);
    self.insts.extend_from_slice(&self.call_api_check_null(get_client_rect));
    self.insts.extend_from_slice(&[
      mov_d(Rax, cursor_x),
      mov_d(Rcx, Jsonpiler::GUI_W),
      IMulRR(Rax, Rcx),
      mov_d(Rcx, right),
      mov_d(Rdx, left),
      SubRR(Rcx, Rdx),
      Clear(Rdx),
      LogicRR(Cmp, Rcx, Rdx),
      JCc(Le, idiv_zero_w),
      Custom(&Jsonpiler::CQO),
      IDivR(Rcx),
      Jmp(idiv_end_w),
      Lbl(idiv_zero_w),
      Clear(Rax),
      Lbl(idiv_end_w),
      mov_d(cursor_x, Rax),
      mov_d(Rax, cursor_y),
      mov_d(Rcx, Jsonpiler::GUI_H),
      IMulRR(Rax, Rcx),
      mov_d(Rcx, bottom),
      mov_d(Rdx, top),
      SubRR(Rcx, Rdx),
      Clear(Rdx),
      LogicRR(Cmp, Rcx, Rdx),
      JCc(Le, idiv_zero_h),
      Custom(&Jsonpiler::CQO),
      IDivR(Rcx),
      Jmp(idiv_end_h),
      Lbl(idiv_zero_h),
      Clear(Rax),
      Lbl(idiv_end_h),
      mov_d(cursor_y, Rax),
      mov_d(pixel_y, 0),
      Lbl(while_y),
      mov_d(Rcx, pixel_y),
      mov_d(Rdx, Jsonpiler::GUI_H),
      LogicRR(Cmp, Rcx, Rdx),
      JCc(E, while_end_y),
      mov_d(pixel_x, 0),
      Lbl(while_x),
      mov_d(Rcx, pixel_x),
      mov_d(Rdx, Jsonpiler::GUI_W),
      LogicRR(Cmp, Rcx, Rdx),
      JCc(E, while_end_x),
      mov_d(Rcx, pixel_x),
      SubRId(Rcx, Jsonpiler::GUI_W >> 1),
      mov_d(Rdx, pixel_y),
      SubRId(Rdx, Jsonpiler::GUI_H >> 1),
      mov_q(R8, Global { id: gui_frame }),
      mov_d(R9, cursor_y),
      mov_d(Rax, Jsonpiler::GUI_H),
      LogicRR(Cmp, R9, Rax),
      CMovCc(G, R9, Rax),
      SubRId(R9, Jsonpiler::GUI_H >> 1),
      NegR(R9),
      mov_q(Args(0x20), R9),
      mov_d(R9, cursor_x),
      mov_d(Rax, Jsonpiler::GUI_W),
      LogicRR(Cmp, R9, Rax),
      CMovCc(G, R9, Rax),
      SubRId(R9, Jsonpiler::GUI_W >> 1),
      Call(render),
      mov_d(Rcx, pixel_y),
      mov_d(Rdx, Jsonpiler::GUI_W),
      IMulRR(Rcx, Rdx),
      mov_d(R8, pixel_x),
      AddRR(R8, Rcx),
      mov_d(Rdx, 4),
      IMulRR(R8, Rdx),
      mov_q(Rcx, Global { id: gui_pixels }),
      AddRR(Rcx, R8),
      mov_d(Ref(Rcx), Rax),
      mov_d(Rcx, pixel_x),
      IncR(Rcx),
      mov_d(pixel_x, Rcx),
      Jmp(while_x),
      Lbl(while_end_x),
      mov_d(Rcx, pixel_y),
      IncR(Rcx),
      mov_d(pixel_y, Rcx),
      Jmp(while_y),
      Lbl(while_end_y),
      mov_q(Rcx, Global { id: hwnd }),
      LeaRM(Rdx, GlobalD { id: paint_struct, disp: 0i32 }),
      CallApi(begin_paint),
      mov_q(Global { id: hdc }, Rax),
      mov_d(R9, GlobalD { id: paint_struct, disp: 0x14i32 }),
      mov_d(Rcx, GlobalD { id: paint_struct, disp: 0xCi32 }),
      SubRR(R9, Rcx),
      mov_d(R8, GlobalD { id: paint_struct, disp: 0x18i32 }),
      mov_d(Rcx, GlobalD { id: paint_struct, disp: 0x10i32 }),
      SubRR(R8, Rcx),
      mov_q(Args(0x20), R8),
      mov_q(Rcx, Global { id: hdc }),
      Clear(Rdx),
      Clear(R8),
      mov_q(Args(0x28), Rdx),
      mov_q(Args(0x30), Rdx),
      mov_d(Rax, Jsonpiler::GUI_W),
      mov_q(Args(0x38), Rax),
      mov_d(Rax, Jsonpiler::GUI_H),
      mov_q(Args(0x40), Rax),
      mov_q(Rax, Global { id: gui_pixels }),
      mov_q(Args(0x48), Rax),
      LeaRM(Rax, Global { id: bm_info }),
      mov_q(Args(0x50), Rax),
      mov_q(Args(0x58), Rdx),
      mov_q(Rax, 0xCC_0020),
      mov_q(Args(0x60), Rax),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(stretch_di_bits));
    self.insts.extend_from_slice(&[
      mov_q(Rcx, Global { id: hwnd }),
      LeaRM(Rdx, Global { id: paint_struct }),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(end_paint));
    self.insts.extend_from_slice(&[
      Clear(Rax),
      mov_q(Global { id: hdc }, Rax),
      Jmp(end_wnd_proc),
      Lbl(handle_wm_timer),
      Clear(Rdx),
      Clear(R8),
    ]);
    self.insts.extend_from_slice(&self.call_api_check_null(invalidate_rect));
    self.insts.extend_from_slice(&[
      Clear(Rax),
      mov_q(Rcx, Global { id: gui_frame }),
      IncR(Rcx),
      mov_q(Global { id: gui_frame }, Rcx),
      Lbl(end_wnd_proc),
      mov_q(Rsp, Rbp),
      Pop(Rbp),
      Custom(&Jsonpiler::RET),
      Lbl(end),
    ]);
    self.data_insts.extend_from_slice(&[Seh(id, end, 20)]);
    Ok(id)
  }
  pub(crate) fn seh_handler(&mut self) -> ErrOR<[Inst; 23]> {
    let exit_process = self.import(KERNEL32, "ExitProcess")?;
    let exception_match_end = self.gen_id();
    let id = self.sym_table["SEH_HANDLER"];
    let exit = self.gen_id();
    let end = self.gen_id();
    self.data_insts.push(Seh(id, end, 20));
    Ok([
      Lbl(id),
      Push(Rbp),
      mov_q(Rbp, Rsp),
      SubRId(Rsp, 0x20),
      mov_q(Rdi, Ref(Rcx)),
      mov_d(Rax, 0xC000_0094),
      LeaRM(Rcx, Global { id: self.global_str("ZeroDivisionError".into()).0 }),
      LogicRR(Cmp, Rdi, Rax),
      CMovCc(E, Rdx, Rcx),
      JCc(E, exception_match_end),
      mov_d(Rax, 0xC000_00FD),
      LogicRR(Cmp, Rdi, Rax),
      JCc(E, exit),
      LeaRM(Rdx, Global { id: self.global_str("An exception occurred!".into()).0 }),
      Lbl(exception_match_end),
      Clear(Rcx),
      Clear(R8),
      mov_d(R9, 0x10),
      CallApi(self.import(USER32, "MessageBoxA")?),
      Lbl(exit),
      mov_q(Rcx, Rdi),
      CallApi(exit_process),
      Lbl(end),
    ])
  }
  pub(crate) fn win_handler(&mut self) -> ErrOR<[Inst; 26]> {
    let exit_process = self.import(KERNEL32, "ExitProcess")?;
    let format_message = self.import(KERNEL32, "FormatMessageW")?;
    let get_last_error = self.import(KERNEL32, "GetLastError")?;
    let message_box = self.import(USER32, "MessageBoxW")?;
    let local_free = self.import(KERNEL32, "LocalFree")?;
    let win_handler_exit = self.gen_id();
    let msg = Local { offset: 8i32 };
    Ok([
      Lbl(self.sym_table["WIN_HANDLER"]),
      SubRId(Rsp, 0x40),
      CallApi(get_last_error),
      mov_q(Rdi, Rax),
      mov_d(Rcx, 0x1300),
      Clear(Rdx),
      mov_q(R8, Rdi),
      Clear(R9),
      LeaRM(Rax, msg),
      mov_q(Args(0x20), Rax),
      Clear(Rax),
      mov_q(Args(0x28), Rax),
      mov_q(Args(0x30), Rax),
      CallApi(format_message),
      TestRdRd(Rax, Rax),
      JCc(E, win_handler_exit),
      Clear(Rcx),
      mov_q(Rdx, msg),
      Clear(R8),
      mov_d(R9, 0x10),
      CallApi(message_box),
      Lbl(win_handler_exit),
      mov_q(Rcx, msg),
      CallApi(local_free),
      mov_q(Rcx, Rdi),
      CallApi(exit_process),
    ])
  }
}
