use crate::prelude::*;
built_in! {self, func, scope, gui;
  {"GUI", SPECIAL, Exact(1),
    init_gui => { self.gui_window(func, scope) },
    init_gui_a => { self.gui_window(func, scope) }
  }
}
impl Jsonpiler {
  fn gui_window(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    let name = func.arg()?.into_ident("render")?;
    let render_id = {
      let Some(u_d) = self.user_defined.get_mut(&name.val) else {
        return err!(name.pos, UndefinedFunc(name.val));
      };
      let render = u_d.val.clone();
      self.use_func(scope.id, render.dep.id);
      self.use_u_d(scope.id, render.dep.id, name.pos)?;
      if render.sig.params.len() != 5 {
        return err!(
          name.pos,
          ArityErr {
            name: "render".into(),
            expected: Exact(5),
            actual: len_u32(&render.sig.params)?
          }
        );
      }
      for (i, (_, param_type)) in render.sig.params.iter().enumerate() {
        if param_type != &IntT {
          let actual = name.pos.with(param_type.clone());
          return Err(type_err(fmt_args((i + 1) as u32, "render"), vec![IntT], actual));
        }
      }
      if render.sig.ret_type != IntT {
        let actual = name.pos.with(render.sig.ret_type.clone());
        return Err(type_err(fmt_ret_val("render"), vec![IntT], actual));
      }
      render.dep.id
    };
    if self.flags.a64 {
      scope.p_a(Bl(self.get_gui_a(scope.id, func.pos, &name.val, render_id)?))?;
    } else {
      scope.p_x(Call(self.get_gui_x(scope.id, func.pos, &name.val, render_id)?))?;
    }
    Ok(Null(Lit(())))
  }
}
