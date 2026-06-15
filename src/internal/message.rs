use crate::prelude::*;
impl Jsonpiler {
  pub(crate) fn add_button_a(
    &mut self,
    ns_str: Memory,
    alert: Memory,
    mem: Memory,
    title: &'static str,
  ) -> ErrOR<Vec<A64Inst>> {
    let string = self.global_str(title);
    Ok(
      [
        self.send_msg(ns_str, "stringWithUTF8String:", load_a(X2, string)?)?,
        store_a(mem, X9, X0)?,
        self.send_msg(alert, "addButtonWithTitle:", load_a(X2, mem)?)?,
      ]
      .concat(),
    )
  }
  pub(crate) fn get_msg_box_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x50;
    let id = symbol!(self, caller, MSG_BOX);
    let ns_app = self.app_kit_init()?;
    let (ns_pool, get_ns_pool) = self.get_class("NSAutoreleasePool")?;
    let (ns_str, get_ns_str) = self.get_class("NSString")?;
    let (ns_alert, get_ns_alert) = self.get_class("NSAlert")?;
    let title = Local(Tmp, -8).h_ptr();
    let msg = Local(Tmp, -0x10).h_ptr();
    let ns_title = Local(Tmp, -0x18).v_rq();
    let ns_msg = Local(Tmp, -0x20).v_rq();
    let ns_ok = Local(Tmp, -0x28).v_rq();
    let alert = Local(Tmp, -0x30).v_rq();
    let pool = Local(Tmp, -0x38).v_rq();
    let response = Local(Tmp, -0x40).v_rq();
    let ns_no = Local(Tmp, -0x48).v_rq();
    let is_confirm = Local(Tmp, -0x49).v_rb();
    let case_msg = self.id();
    let msg_end = self.id();
    let insts = vec![
      store_a(title, X9, X0)?,
      store_a(msg, X9, X1)?,
      store_a(is_confirm, X9, X2)?,
      self.send_msg(ns_app, "finishLaunching", vec![])?,
      get_ns_pool,
      get_ns_str,
      get_ns_alert,
      self.send_msg(ns_pool, "new", vec![])?,
      store_a(pool, X9, X0)?,
      self.send_msg(ns_app, "setActivationPolicy:", load_imm_a(X2, 1))?,
      self.send_msg(ns_app, "activateIgnoringOtherApps:", load_imm_a(X2, 1))?,
      self.send_msg(ns_str, "stringWithUTF8String:", load_a(X2, title)?)?,
      store_a(ns_title, X9, X0)?,
      self.send_msg(ns_str, "stringWithUTF8String:", load_a(X2, msg)?)?,
      store_a(ns_msg, X9, X0)?,
      self.send_msg(ns_alert, "new", vec![])?,
      store_a(alert, X9, X0)?,
      load_a(X0, is_confirm)?,
      vec![TstRb(X0), BCc(E.into(), case_msg)],
      self.add_button_a(ns_str, alert, ns_ok, "Yes")?,
      self.add_button_a(ns_str, alert, ns_no, "No")?,
      vec![B_(msg_end), LblA(case_msg)],
      self.add_button_a(ns_str, alert, ns_ok, "Ok")?,
      vec![LblA(msg_end)],
      self.send_msg(alert, "setMessageText:", load_a(X2, ns_title)?)?,
      self.send_msg(alert, "setInformativeText:", load_a(X2, ns_msg)?)?,
      self.send_msg(alert, "runModal", vec![])?,
      store_a(response, X9, X0)?,
      self.send_msg(alert, "release", vec![])?,
      self.send_msg(pool, "release", vec![])?,
      self.send_msg(ns_app, "setActivationPolicy:", load_imm_a(X2, 0))?,
      load_a(X0, response)?,
    ];
    self.use_func(caller, id);
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn get_msg_box_x(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x40;
    let id = symbol!(self, caller, MSG_BOX);
    let u8_to_16 = self.get_u8_to_16(id)?;
    let title = Local(Tmp, -0x08);
    let text = Local(Tmp, -0x10);
    let type_and_ret = Local(Tmp, -0x18);
    let insts = vec![
      vec![
        store(S8, text, Rdx),
        store(S8, type_and_ret, R8),
        MovMId(Reg(Rdx), CP_UTF8),
        Call(u8_to_16),
        store(S8, title, Rax),
        load(S8, Rcx, text),
        MovMId(Reg(Rdx), CP_UTF8),
        Call(u8_to_16),
        store(S8, text, Rax),
        Clear(Rcx),
        load(S8, Rdx, text),
        load(S8, R8, title),
        load(S8, R9, type_and_ret),
        CallApiCheck(self.api(USER32, "MessageBoxW")),
        store(S8, type_and_ret, Rax),
        load(S8, R8, title),
      ],
      self.call_free_r8_x()?,
      vec![load(S8, R8, text)],
      self.call_free_r8_x()?,
      vec![load(S8, Rax, type_and_ret)],
    ];
    self.link_func_x(id, insts, SIZE);
    Ok(id)
  }
}
