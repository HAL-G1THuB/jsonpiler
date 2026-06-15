use crate::prelude::*;
type Imp<'a> = &'a dyn Fn(&mut Jsonpiler, LabelId) -> ErrOR<LabelId>;
impl Jsonpiler {
  fn add_ivar(
    &mut self,
    class: Memory,
    name: &'static str,
    size: i64,
    alignment: i64,
    type_encoding: &'static str,
  ) -> ErrOR<Vec<A64Inst>> {
    let insts: &[&[A64Inst]] = &[
      &load_a(X0, class)?,
      &load_a(X1, self.global_str(name))?,
      &load_imm_a(X2, size),
      &load_imm_a(X3, alignment),
      &load_a(X4, self.global_str(type_encoding))?,
      &[BApi(self.api(OBJC_A, "_class_addIvar"))],
    ];
    Ok(insts.concat())
  }
  fn add_ivars(
    &mut self,
    class: Memory,
    ivars: &[(&'static str, i64, i64, &'static str)],
  ) -> ErrOR<Vec<Vec<A64Inst>>> {
    let mut insts = vec![];
    for (name, size, alignment, type_encoding) in ivars {
      insts.push(self.add_ivar(class, name, *size, *alignment, type_encoding)?);
    }
    Ok(insts)
  }
  fn add_method(
    &mut self,
    class: Memory,
    sel: &'static str,
    imp: LabelId,
    types: &'static str,
  ) -> ErrOR<Vec<A64Inst>> {
    let insts: &[&[A64Inst]] = &[
      &self.reg_name(sel)?,
      &[MovRR(X1, X0)],
      &load_a(X0, class)?,
      &load_a(X2, Global(imp).ptr())?,
      &load_a(X3, self.global_str(types))?,
      &[BApi(self.api(OBJC_A, "_class_addMethod"))],
    ];
    Ok(insts.concat())
  }
  fn add_methods(
    &mut self,
    class: Memory,
    methods: &[(&'static str, Imp, &'static str)],
    caller: LabelId,
  ) -> ErrOR<Vec<Vec<A64Inst>>> {
    let mut insts = vec![];
    for (sel, imp, types) in methods {
      let id = imp(self, caller)?;
      insts.push(self.add_method(class, sel, id, types)?);
    }
    Ok(insts)
  }
  pub(crate) fn app_kit_init(&mut self) -> ErrOR<Memory> {
    if let Some(id) = self.symbols.get(NS_APP) {
      return Ok(Global(*id).v_rq());
    }
    self.apis.push((APP_KIT.into(), vec![]));
    let (ns_app, get_app) = self.get_class_id(NS_APP)?;
    self.symbols.insert(NS_APP, ns_app);
    let shared_app = self.send_msg(Global(ns_app).v_rq(), "sharedApplication", vec![])?;
    self.startup.a64_mut()?.extend([get_app, shared_app, store_a(Global(ns_app).v_rq(), X1, X0)?]);
    Ok(Global(ns_app).v_rq())
  }
  fn cg_rect_make(&mut self, width: Bind<f64>, height: Bind<f64>) -> ErrOR<Vec<A64Inst>> {
    Ok(
      [
        self.load_dn(X0, X9, Lit(0.0))?,
        self.load_dn(X1, X9, Lit(0.0))?,
        self.load_dn(X2, X9, width)?,
        self.load_dn(X3, X9, height)?,
      ]
      .concat(),
    )
  }
  pub(crate) fn get_class(&mut self, class: &'static str) -> ErrOR<(Memory, Vec<A64Inst>)> {
    self.get_class_id(class).map(|(id, insts)| (Global(id).v_rq(), insts))
  }
  pub(crate) fn get_class_id(&mut self, class: &'static str) -> ErrOR<(LabelId, Vec<A64Inst>)> {
    let id = self.bss(8, 8);
    let insts: &[&[A64Inst]] = &[
      &load_a(X0, self.global_str(class))?,
      &[BApi(self.api(OBJC_A, "_objc_getClass"))],
      &store_a(Global(id).v_rq(), X1, X0)?,
    ];
    Ok((id, insts.concat()))
  }
  fn load_ivar_x0(&mut self, object: Memory, name: &'static str) -> ErrOR<Vec<A64Inst>> {
    let insts: &[&[A64Inst]] = &[
      &load_a(X0, object)?,
      &[BApi(self.api(OBJC_A, "_object_getClass"))],
      &load_a(X1, self.global_str(name))?,
      &[BApi(self.api(OBJC_A, "_class_getInstanceVariable"))],
      &[BApi(self.api(OBJC_A, "_ivar_getOffset"))],
      &load_a(X1, object)?,
      &[AddR3(X0, X0, X1)],
    ];
    Ok(insts.concat())
  }
  fn reg_name(&mut self, sel: &'static str) -> ErrOR<Vec<A64Inst>> {
    let reg_name = self.api(OBJC_A, "_sel_registerName");
    Ok([load_a(X0, self.global_str(sel))?, vec![BApi(reg_name)]].concat())
  }
  pub(crate) fn send_msg(
    &mut self,
    class: Memory,
    sel: &'static str,
    arg_insts: Vec<A64Inst>,
  ) -> ErrOR<Vec<A64Inst>> {
    let msg_send = self.api(OBJC_A, "_objc_msgSend");
    let insts = [
      self.reg_name(sel)?,
      vec![MovRR(X1, X0)],
      load_a(X0, class)?,
      arg_insts,
      vec![BApi(msg_send)],
    ];
    Ok(insts.concat())
  }
}
impl Jsonpiler {
  fn create_window_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x30;
    let id = self.id();
    let (ns_window, get_ns_window) = self.get_class("NSWindow")?;
    let (canvas_view, get_canvas_view) = self.get_class("CanvasView")?;
    let (ns_str, get_ns_str) = self.get_class("NSString")?;
    let this = Local(Tmp, -8).v_rq();
    let view = Local(Tmp, -0x10).v_rq();
    let window = Local(Tmp, -0x18).v_rq();
    let ns_title = Local(Tmp, -0x28).v_rq();
    let render = Local(Tmp, -0x30).v_rq();
    let cg_rect = self.cg_rect_make(Lit(GUI_W as f64), Lit(GUI_H as f64))?;
    let insts = vec![
      store_a(this, X9, X0)?,
      get_ns_str,
      get_ns_window,
      get_canvas_view,
      self.send_msg(canvas_view, "alloc", vec![])?,
      store_a(view, X1, X0)?,
      self.send_msg(view, "initWithFrame:", cg_rect.clone())?,
      store_a(view, X1, X0)?,
      self.load_ivar_x0(this, "render")?,
      vec![LdR(S8, X0, X0, 0)],
      store_a(render, X9, X0)?,
      self.load_ivar_x0(view, "render")?,
      load_a(X1, render)?,
      vec![StR(S8, X1, X0, 0)],
      self.send_msg(ns_window, "alloc", vec![])?,
      store_a(window, X1, X0)?,
      self.send_msg(
        window,
        "initWithContentRect:styleMask:backing:defer:",
        [cg_rect, load_imm_a(X2, 0b1111), load_imm_a(X3, 2), load_imm_a(X4, 0)].concat(),
      )?,
      store_a(window, X1, X0)?,
      self.load_ivar_x0(this, "window")?,
      load_a(X1, window)?,
      vec![StR(S8, X1, X0, 0)],
      self.send_msg(window, "setReleasedWhenClosed:", load_imm_a(X2, 0))?,
      self.load_ivar_x0(this, "title")?,
      vec![LdR(S8, X0, X0, 0)],
      store_a(ns_title, X9, X0)?,
      self.send_msg(ns_str, "stringWithUTF8String:", load_a(X2, ns_title)?)?,
      store_a(ns_title, X1, X0)?,
      self.send_msg(window, "setAcceptsMouseMovedEvents:", load_imm_a(X2, 1))?,
      self.send_msg(window, "setTitle:", load_a(X2, ns_title)?)?,
      self.send_msg(window, "setContentView:", load_a(X2, view)?)?,
      self.send_msg(view, "release", vec![])?,
      self.send_msg(window, "makeKeyAndOrderFront:", load_imm_a(X2, 0))?,
      self.send_msg(window, "setDelegate:", load_a(X2, this)?)?,
    ];
    self.use_func(caller, id);
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
  fn dealloc_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x10;
    let id = self.id();
    let this = Local(Tmp, -0x10).v_rq();
    let super_class = Local(Tmp, -8).v_rq();
    let insts = vec![
      store_a(this, X9, X0)?,
      vec![
        BApi(self.api(OBJC_A, "_object_getClass")),
        BApi(self.api(OBJC_A, "_class_getSuperclass")),
      ],
      store_a(super_class, X9, X0)?,
      self.load_ivar_x0(this, "pixels")?,
      vec![LdR(S8, X0, X0, 0)],
      self.call_free_x0_a()?,
      self.reg_name("dealloc")?,
      vec![MovRR(X1, X0)],
      load_ref_a(X0, this)?,
      vec![BApi(self.api(OBJC_A, "_objc_msgSendSuper"))],
    ];
    self.use_func(caller, id);
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
  fn draw_rect_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x80;
    let id = self.id();
    let (ns_gc, get_gc) = self.get_class("NSGraphicsContext")?;
    let x = Local(Tmp, -8).v_rq();
    let y = Local(Tmp, -0x10).v_rq();
    let this = Local(Tmp, -0x28).v_rq();
    let pixels = Local(Tmp, -0x30).v_rq();
    let color_space = Local(Tmp, -0x38).v_rq();
    let bitmap_context = Local(Tmp, -0x40).v_rq();
    let image = Local(Tmp, -0x48).v_rq();
    let graphics_context = Local(Tmp, -0x50).v_rq();
    let ctx = Local(Tmp, -0x58).v_rq();
    let mouse_x = Local(Tmp, -0x60).v_rq();
    let mouse_y = Local(Tmp, -0x68).v_rq();
    let frame = Local(Tmp, -0x70).v_rq();
    let render = Local(Tmp, -0x78).v_rq();
    let outer_loop = self.id();
    let inner_loop = self.id();
    let next_row = self.id();
    let exit = self.id();
    let insts = vec![
      store_a(this, X9, X0)?,
      self.load_ivar_x0(this, "pixels")?,
      vec![LdR(S8, X0, X0, 0)],
      store_a(pixels, X9, X0)?,
      self.load_ivar_x0(this, "mouse_x")?,
      vec![LdR(S8, X0, X0, 0)],
      store_a(mouse_x, X9, X0)?,
      self.load_ivar_x0(this, "mouse_y")?,
      vec![LdR(S8, X0, X0, 0)],
      store_a(mouse_y, X9, X0)?,
      self.load_ivar_x0(this, "frame")?,
      store_a(frame, X9, X0)?,
      self.load_ivar_x0(this, "render")?,
      vec![LdR(S8, X0, X0, 0)],
      store_a(render, X9, X0)?,
      load_imm_a(X0, 0),
      store_a(y, X9, X0)?,
      vec![LblA(outer_loop)],
      load_a(X0, y)?,
      load_imm_a(X1, i64::from(GUI_H)),
      vec![CmpRR(X0, X1), BCc(Ge.into(), exit)],
      load_imm_a(X0, 0),
      store_a(x, X9, X0)?,
      vec![LblA(inner_loop)],
      load_a(X0, x)?,
      load_imm_a(X1, i64::from(GUI_W)),
      vec![CmpRR(X0, X1), BCc(Ge.into(), next_row)],
      load_a(X0, x)?,
      vec![SubRI12(X0, X0, (GUI_W / 2) as u16)],
      load_a(X1, y)?,
      vec![SubRI12(X1, X1, (GUI_H / 2) as u16)],
      load_a(X2, frame)?,
      vec![LdR(S8, X2, X2, 0)],
      load_a(X3, mouse_x)?,
      load_a(X4, mouse_y)?,
      load_a(X5, render)?,
      vec![Blr(X5)],
      load_imm_a(X1, i64::from(GUI_H - 1)),
      load_a(X2, y)?,
      vec![SubR3(X1, X1, X2), Lsl(X1, X1, 9)],
      load_a(X2, x)?,
      vec![AddR3(X1, X1, X2), Lsl(X1, X1, 2)],
      load_a(X2, pixels)?,
      vec![AddR3(X2, X2, X1), StR(S4, X0, X2, 0)],
      inc_a(x, X0, X9)?,
      vec![B_(inner_loop), LblA(next_row)],
      inc_a(y, X0, X9)?,
      vec![
        B_(outer_loop),
        LblA(exit),
        BApi(self.api(CORE_GRAPHICS, "_CGColorSpaceCreateDeviceRGB")),
      ],
      store_a(color_space, X9, X0)?,
      load_a(X0, pixels)?,
      load_imm_a(X1, i64::from(GUI_W)),
      load_imm_a(X2, i64::from(GUI_H)),
      load_imm_a(X3, 8),
      load_imm_a(X4, i64::from(GUI_W * 4)),
      load_a(X5, color_space)?,
      load_imm_a(X6, 0x2006),
      vec![BApi(self.api(CORE_GRAPHICS, "_CGBitmapContextCreate"))],
      store_a(bitmap_context, X9, X0)?,
      load_a(X0, bitmap_context)?,
      vec![BApi(self.api(CORE_GRAPHICS, "_CGBitmapContextCreateImage"))],
      store_a(image, X9, X0)?,
      get_gc,
      self.send_msg(ns_gc, "currentContext", vec![])?,
      store_a(graphics_context, X1, X0)?,
      self.send_msg(graphics_context, "CGContext", vec![])?,
      store_a(ctx, X1, X0)?,
      load_imm_a(X1, 1),
      vec![BApi(self.api(CORE_GRAPHICS, "_CGContextSetInterpolationQuality"))],
      self.send_msg(this, "bounds", vec![])?,
      load_a(X0, ctx)?,
      load_a(X1, image)?,
      vec![BApi(self.api(CORE_GRAPHICS, "_CGContextDrawImage"))],
      load_a(X0, image)?,
      vec![BApi(self.api(CORE_GRAPHICS, "_CGImageRelease"))],
      load_a(X0, bitmap_context)?,
      vec![BApi(self.api(CORE_GRAPHICS, "_CGContextRelease"))],
      load_a(X0, color_space)?,
      vec![BApi(self.api(CORE_GRAPHICS, "_CGColorSpaceRelease"))],
      load_a(X0, frame)?,
      vec![LdR(S8, X1, X0, 0), AddRI12(X1, X1, 1), StR(S8, X1, X0, 0)],
    ];
    self.use_func(caller, id);
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
  pub(crate) fn get_app_delegate_a(
    &mut self,
    caller: LabelId,
  ) -> ErrOR<(LabelId, LabelId, Vec<A64Inst>)> {
    let (id, m_did_finish) = if let Some(id) = self
      .symbols
      .get(APP_DELEGATE)
      .and_then(|a_d| self.symbols.get(CREATE_WINDOW).map(|create_window| (*a_d, *create_window)))
    {
      id
    } else {
      let id = self.bss(8, 8);
      self.symbols.insert(APP_DELEGATE, id);
      let register_canvas = self.register_canvas_view_a(caller)?;
      let register_delegate = self.register_app_delegate_a(caller)?;
      let create_window = self.create_window_a(caller)?;
      self.symbols.insert(CREATE_WINDOW, create_window);
      self.startup.a64_mut()?.push(vec![Bl(register_canvas), Bl(register_delegate)]);
      (id, create_window)
    };
    let insts: &[&[A64Inst]] = &[
      &load_a(X0, self.global_str(APP_DELEGATE))?,
      &[BApi(self.api(OBJC_A, "_objc_getClass"))],
      &store_a(Global(id).v_rq(), X1, X0)?,
    ];
    Ok((id, m_did_finish, insts.concat()))
  }
  pub(crate) fn get_gui_a(
    &mut self,
    caller: LabelId,
    func_pos: Position,
    name: &str,
    render: LabelId,
  ) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x20;
    let id = self.id();
    let secondary_gui_err = self.runtime_err(SecondaryGUIErr, None, func_pos, caller)?;
    let flag_gui = self.get_flag_gui()?;
    let ns_app = self.app_kit_init()?;
    let (app_delegate, create_window, get_app_delegate) = self.get_app_delegate_a(caller)?;
    let (ns_pool, get_ns_pool) = self.get_class("NSAutoreleasePool")?;
    let delegate = Local(Tmp, -8).v_rq();
    let pool = Local(Tmp, -0x10).v_rq();
    let insts = vec![
      get_ns_pool,
      load_a(X0, Global(flag_gui).v_rb())?,
      vec![TstRb(X0), BCc(Ne.into(), secondary_gui_err)],
      load_imm_a(X0, i64::from(bool2byte(true))),
      store_a(Global(flag_gui).v_rb(), X1, X0)?,
      self.send_msg(ns_pool, "new", vec![])?,
      store_a(pool, X1, X0)?,
      get_app_delegate,
      self.send_msg(Global(app_delegate).v_rq(), "alloc", vec![])?,
      store_a(delegate, X1, X0)?,
      self.send_msg(delegate, "init", vec![])?,
      store_a(delegate, X1, X0)?,
      self.load_ivar_x0(delegate, "title")?,
      load_a(X1, self.global_str(name))?,
      vec![StR(S8, X1, X0, 0)],
      self.load_ivar_x0(delegate, "render")?,
      load_a(X1, Global(render).ptr())?,
      vec![StR(S8, X1, X0, 0)],
      self.send_msg(ns_app, "setDelegate:", load_a(X2, delegate)?)?,
      self.send_msg(ns_app, "setActivationPolicy:", load_imm_a(X2, 0))?,
      self.send_msg(ns_app, "activateIgnoringOtherApps:", load_imm_a(X2, 1))?,
      load_a(X0, delegate)?,
      vec![Bl(create_window)],
      self.send_msg(ns_app, "run", vec![])?,
      self.send_msg(delegate, "release", vec![])?,
      self.send_msg(pool, "release", vec![])?,
      load_imm_a(X0, i64::from(bool2byte(false))),
      store_a(Global(flag_gui).v_rb(), X1, X0)?,
    ];
    self.use_func(caller, id);
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
  fn init_with_frame_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x40;
    let id = self.id();
    let leak = Global(self.get_leak()?).v_rd();
    let (ns_timer, get_ns_timer) = self.get_class("NSTimer")?;
    // superInfo
    let super_class = Local(Tmp, -8).v_rq();
    let this = Local(Tmp, -0x10).v_rq();
    // end
    let pixels = Local(Tmp, -0x18).v_rq();
    let tick_name = Local(Tmp, -0x20).v_rq();
    let t_area = Local(Tmp, -0x28).v_rq();
    let bounds_w = Local(Tmp, -0x30).v_rq();
    let bounds_h = Local(Tmp, -0x38).v_rq();
    let timer = Local(Tmp, -0x40).v_rq();
    let (ns_t_area, get_ns_tracking_area) = self.get_class("NSTrackingArea")?;
    let insts = vec![
      store_a(this, X9, X0)?,
      vec![
        BApi(self.api(OBJC_A, "_object_getClass")),
        BApi(self.api(OBJC_A, "_class_getSuperclass")),
      ],
      store_a(super_class, X9, X0)?,
      get_ns_timer,
      get_ns_tracking_area,
      self.reg_name("initWithFrame:")?,
      vec![MovRR(X1, X0)],
      load_ref_a(X0, this)?,
      self.cg_rect_make(Lit(GUI_W as f64), Lit(GUI_H as f64))?,
      vec![BApi(self.api(OBJC_A, "_objc_msgSendSuper"))],
      store_a(this, X9, X0)?,
      load_imm_a(X0, i64::from(GUI_W * GUI_H)),
      load_imm_a(X1, 4),
      self.call_api_zero_a(SYS_B, "_calloc"),
      store_a(pixels, X9, X0)?,
      inc_a(leak, X0, X9)?,
      self.load_ivar_x0(this, "pixels")?,
      load_a(X1, pixels)?,
      vec![StR(S8, X1, X0, 0)],
      self.load_ivar_x0(this, "frame")?,
      vec![StR(S8, Xzr, X0, 0)],
      self.load_ivar_x0(this, "mouse_x")?,
      vec![StR(S8, Xzr, X0, 0)],
      self.load_ivar_x0(this, "mouse_y")?,
      vec![StR(S8, Xzr, X0, 0)],
      self.reg_name("tick:")?,
      store_a(tick_name, X9, X0)?,
      #[expect(clippy::float_arithmetic)]
      {
        let mov_interval = self.load_dn(X0, X9, Lit(TIMER_INTERVAL_MS as f64 / 1000.0))?;
        self.send_msg(
          ns_timer,
          "scheduledTimerWithTimeInterval:target:selector:userInfo:repeats:",
          [
            mov_interval,
            load_a(X2, this)?,
            load_a(X3, tick_name)?,
            load_imm_a(X4, 0),
            load_imm_a(X5, 1),
          ]
          .concat(),
        )?
      },
      store_a(timer, X9, X0)?,
      self.load_ivar_x0(this, "timer")?,
      load_a(X1, timer)?,
      vec![StR(S8, X1, X0, 0)],
      self.send_msg(ns_t_area, "alloc", vec![])?,
      store_a(t_area, X1, X0)?,
      self.send_msg(this, "bounds", vec![])?,
      store_dn(bounds_w, X9, X2)?,
      store_dn(bounds_h, X9, X3)?,
      {
        let tracking_insts = [
          self.cg_rect_make(Var(bounds_w), Var(bounds_h))?,
          load_imm_a(X2, 0x282),
          load_a(X3, this)?,
          load_imm_a(X4, 0),
        ]
        .concat();
        self.send_msg(t_area, "initWithRect:options:owner:userInfo:", tracking_insts)?
      },
      store_a(t_area, X1, X0)?,
      self.send_msg(this, "addTrackingArea:", load_a(X2, t_area)?)?,
      self.send_msg(t_area, "release", vec![])?,
      load_a(X0, this)?,
    ];
    self.use_func(caller, id);
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
  fn mouse_moved_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x40;
    let id = self.id();
    let this = Local(Tmp, -8).v_rq();
    let event = Local(Tmp, -0x10).v_rq();
    let mouse_x = Local(Tmp, -0x20).v_rq();
    let mouse_y = Local(Tmp, -0x28).v_rq();
    let mouse_xy =
      [("mouse_x", (mouse_x, X0), (GUI_W, X2)), ("mouse_y", (mouse_y, X1), (GUI_H, X3))];
    let mut cvt_point_insts = vec![];
    for (_, (mouse_mem, mouse_reg), _) in mouse_xy {
      cvt_point_insts.extend(self.load_dn(mouse_reg, X9, Var(mouse_mem))?);
    }
    cvt_point_insts.extend(load_imm_a(X2, 0));
    let mut insts = vec![
      store_a(this, X9, X0)?,
      store_a(event, X9, X2)?,
      self.send_msg(event, "locationInWindow", vec![])?,
    ];
    for (_, (mouse_mem, mouse_reg), _) in mouse_xy {
      insts.push(store_dn(mouse_mem, X9, mouse_reg)?);
    }
    insts.push(self.send_msg(this, "convertPoint:fromView:", cvt_point_insts)?);
    for (_, (mouse_mem, mouse_reg), _) in mouse_xy {
      insts.push(store_dn(mouse_mem, X9, mouse_reg)?);
    }
    insts.push(self.send_msg(this, "bounds", vec![])?);
    for (_, (mouse_mem, _), (size, size_reg)) in mouse_xy {
      insts.extend([
        self.load_dn(X4, X9, Var(mouse_mem))?,
        self.load_dn(X5, X9, Lit(size as f64))?,
        vec![FArithD(Mul, X4, X4, X5), FArithD(Div, X4, X4, size_reg), FCvtZSD(X0, X4)],
        store_a(mouse_mem, X9, X0)?,
      ]);
    }
    for (name, (mouse_mem, _), (size, _)) in mouse_xy {
      insts.extend([
        self.load_ivar_x0(this, name)?,
        load_a(X1, mouse_mem)?,
        vec![SubRI12(X1, X1, (size / 2) as u16), StR(S8, X1, X0, 0)],
      ]);
    }
    self.use_func(caller, id);
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
  fn register_app_delegate_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x10;
    const METHODS: &[(&str, Imp, &str)] =
      &[("windowWillClose:", &Jsonpiler::window_will_close_a, "v@:@")];
    const IVARS: &[(&str, i64, i64, &str)] =
      &[("window", 8, 3, "@"), ("title", 8, 3, "*"), ("render", 8, 3, "^v")];
    let id = self.id();
    let (ns_object, get_ns_object) = self.get_class("NSObject")?;
    let class = Local(Tmp, -8).v_rq();
    let mut insts = vec![
      get_ns_object,
      load_a(X0, ns_object)?,
      load_a(X1, self.global_str(APP_DELEGATE))?,
      load_imm_a(X2, 0),
      vec![BApi(self.api(OBJC_A, "_objc_allocateClassPair"))],
      store_a(class, X1, X0)?,
    ];
    insts.extend(self.add_ivars(class, IVARS)?);
    insts.extend(self.add_methods(class, METHODS, id)?);
    insts.extend([load_a(X0, class)?, vec![BApi(self.api(OBJC_A, "_objc_registerClassPair"))]]);
    self.use_func(caller, id);
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
  fn register_canvas_view_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x10;
    const METHODS: &[(&str, Imp, &str)] = &[
      ("initWithFrame:", &Jsonpiler::init_with_frame_a, "@@:{CGRect={CGPoint=dd}{CGSize=dd}}"),
      ("drawRect:", &Jsonpiler::draw_rect_a, "v@:{CGRect={CGPoint=dd}{CGSize=dd}}"),
      ("dealloc", &Jsonpiler::dealloc_a, "v@:"),
      ("mouseMoved:", &Jsonpiler::mouse_moved_a, "v@:@"),
      ("tick:", &Jsonpiler::tick_a, "v@:@"),
      ("stopTimer", &Jsonpiler::stop_timer_a, "v@:"),
    ];
    const IVARS: &[(&str, i64, i64, &str)] = &[
      ("mouse_x", 8, 3, "q"),
      ("mouse_y", 8, 3, "q"),
      ("frame", 8, 3, "q"),
      ("pixels", 8, 3, "^I"),
      ("render", 8, 3, "^v"),
      ("timer", 8, 3, "@"),
    ];
    let id = self.id();
    let (ns_view, get_ns_view) = self.get_class("NSView")?;
    let class = Local(Tmp, -8).v_rq();
    let mut insts = vec![
      get_ns_view,
      load_a(X0, ns_view)?,
      load_a(X1, self.global_str("CanvasView"))?,
      load_imm_a(X2, 0),
      vec![BApi(self.api(OBJC_A, "_objc_allocateClassPair"))],
      store_a(class, X1, X0)?,
    ];
    insts.extend(self.add_ivars(class, IVARS)?);
    insts.extend(self.add_methods(class, METHODS, caller)?);
    insts.extend([load_a(X0, class)?, vec![BApi(self.api(OBJC_A, "_objc_registerClassPair"))]]);
    self.use_func(caller, id);
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
  fn stop_timer_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x10;
    let id = self.id();
    let this = Local(Tmp, -8).v_rq();
    let timer = Local(Tmp, -0x10).v_rq();
    let insts = vec![
      store_a(this, X9, X0)?,
      self.load_ivar_x0(this, "timer")?,
      vec![LdR(S8, X1, X0, 0)],
      store_a(timer, X9, X1)?,
      vec![StR(S8, Xzr, X0, 0)],
      self.send_msg(timer, "invalidate", vec![])?,
    ];
    self.use_func(caller, id);
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
  fn tick_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x10;
    let id = self.id();
    let this = Local(Tmp, -8).v_rq();
    let insts =
      vec![store_a(this, X9, X0)?, self.send_msg(this, "setNeedsDisplay:", load_imm_a(X2, 1))?];
    self.use_func(caller, id);
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
  fn window_will_close_a(&mut self, caller: LabelId) -> ErrOR<LabelId> {
    const SIZE: i32 = 0x20;
    let id = self.id();
    let ns_app = self.app_kit_init()?;
    let this = Local(Tmp, -8).v_rq();
    let window = Local(Tmp, -0x10).v_rq();
    let view = Local(Tmp, -0x18).v_rq();
    let insts = vec![
      store_a(this, X9, X0)?,
      self.load_ivar_x0(this, "window")?,
      vec![LdR(S8, X1, X0, 0)],
      store_a(window, X9, X1)?,
      vec![StR(S8, Xzr, X0, 0)],
      self.send_msg(window, "contentView", vec![])?,
      store_a(view, X1, X0)?,
      self.send_msg(view, "stopTimer", vec![])?,
      self.send_msg(window, "setContentView:", load_imm_a(X2, 0))?,
      self.send_msg(window, "release", vec![])?,
      self.send_msg(ns_app, "stop:", load_imm_a(X2, 0))?,
    ];
    self.use_func(caller, id);
    self.link_func_a(id, insts, SIZE);
    Ok(id)
  }
}
