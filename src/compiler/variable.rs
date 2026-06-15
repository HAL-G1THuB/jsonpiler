use crate::prelude::*;
const ENTER: &str = "EnterCriticalSection";
const LEAVE: &str = "LeaveCriticalSection";
impl Jsonpiler {
  pub(crate) fn assign(
    &mut self,
    is_global_opt: Option<bool>,
    (name, val): KeyVal,
    scope: &mut Scope,
  ) -> ErrOR<bool> {
    let reassign = if let Some(is_g) = is_global_opt {
      self.check_defined(&name, name.pos, GlobalVar, scope)?;
      Err(is_g)
    } else {
      let var = self.get_var(&name, scope)?.val;
      if var.val.as_type() != val.val.as_type() {
        let val_type = val.map_ref(Json::as_type);
        return Err(type_err(fmt_var(&name.val, var.kind), vec![var.val.as_type()], val_type));
      }
      let Some(mem) = var.val.mem() else {
        return err!(name.pos, UndefinedVar(name.val.clone()));
      };
      Ok(mem)
    };
    let is_global = reassign.is_err_and(|is_g| is_g);
    let call_once = scope.loop_labels.is_empty() && scope.epilogue.is_none();
    let data_sect = is_global && call_once;
    let value = match &val.val {
      Bool(Lit(lit)) if data_sect => Bool(Var(self.global_b(*lit))),
      Int(Lit(int)) if data_sect => Int(Var(self.global_q(int.cast_unsigned()))),
      Float(Lit(lit)) if data_sect => Float(Var(self.global_q(lit.to_bits()))),
      Null(_) | Array(..) | Bool(_) | Float(_) | Int(_) | Object(_) | Str(_) => {
        if is_global {
          if self.flags.a64 {
            self.os_unfair_lock(scope, true)?;
          } else {
            self.critical_sect(scope, ENTER)?;
          }
        }
        let val_type = val.val.as_type();
        let mem_type = val_type.mem_type(val.pos)?;
        let size = mem_type.size()?;
        let mem = match reassign {
          Ok(mem) => {
            self.heap_free(mem, scope)?;
            mem
          }
          Err(is_g) => Memory(
            if is_g {
              let size_u32 = u32::try_from(size)?;
              Global(self.bss(size_u32, size_u32))
            } else {
              Local(Long, scope.alloc(size, size)?)
            },
            MemType { pass_by: Value, size: Small(mem_type.reg_size()) },
          ),
        };
        let value = val_type.to_json(val.pos, mem.0)?;
        if self.flags.a64 {
          scope.e_a(self.load_json_a(X0, &val, Some(scope.id))?)?;
          scope.e_a(store_a(mem, X1, X0)?)?;
        } else {
          scope.e_x(self.load_json_x(Rax, &val, Some(scope.id))?)?;
          scope.e_x(store_x(mem, Rcx, Rax)?)?;
        }
        self.drop_json(&val.val, false, scope)?;
        if is_global {
          if self.flags.a64 {
            self.os_unfair_lock(scope, false)?;
          } else {
            self.critical_sect(scope, LEAVE)?;
          }
        }
        value
      }
    };
    if let Err(is_g) = reassign {
      let target = if is_g { &mut self.globals } else { scope.innermost() };
      let kind = if is_g { GlobalVar } else { LocalVar };
      target.insert(name.val, name.pos.with(Variable::new(value, kind)));
    }
    Ok(reassign.is_ok())
  }
  fn critical_sect(&mut self, scope: &mut Scope, action: &'static str) -> ErrOR<()> {
    let critical_section = Global(self.get_critical_section()?);
    scope.e_x(vec![LeaRM(Rcx, critical_section), CallApi(self.api(KERNEL32, action))])
  }
  fn os_unfair_lock(&mut self, scope: &mut Scope, lock: bool) -> ErrOR<()> {
    let action = if lock { "_os_unfair_lock_lock" } else { "_os_unfair_lock_unlock" };
    let os_unfair_lock = Global(self.get_os_unfair_lock()?).ptr();
    scope.e_a(load_a(X0, os_unfair_lock)?)?;
    scope.p_a(BApi(self.api(SYS_B, action)))
  }
  fn reassign(&mut self, func: &mut Pos<BuiltIn>, scope: &mut Scope) -> ErrOR<Json> {
    let mut first = func.arg()?;
    if let Object(Lit(obj)) = &mut first.val
      && obj.len() == 1
      && (obj[0].0.val == "let" || obj[0].0.val == "global")
    {
      if let Some(builtin) = self.builtin.get_mut(&obj[0].0.val) {
        builtin.refs.push(obj[0].0.pos);
      }
      let is_global = obj[0].0.val == "global";
      let var = if let Array(_, Lit(args)) = &mut obj[0].1.val
        && args.len() == 1
      {
        take(&mut args[0])
      } else {
        take(&mut obj[0].1)
      }
      .into_ident("Variable name")?;
      let val = self.eval(func.arg()?, scope)?;
      self.assign(Some(is_global), (var, val), scope)?;
      return Ok(Null(Lit(())));
    }
    let name = first.into_ident("Variable name")?;
    let val = self.eval(func.arg()?, scope)?;
    if self.assign(None, (name.clone(), val), scope)? {
      Ok(Null(Lit(())))
    } else {
      err!(name.pos, UndefinedVar(name.val))
    }
  }
}
built_in! {self, _func, _scope, variable;
  {"global", SPECIAL, Exact(1),
    assign_global => {Ok(Null(Lit(()))) },
    assign_global_a => { Ok(Null(Lit(()))) }
  },
  {"let", SPECIAL, Exact(1),
    assign_local => { Ok(Null(Lit(()))) },
    assign_local_a => { Ok(Null(Lit(()))) }
  },
  {"=", SPECIAL, Exact(2),
    f_reassign => { self.reassign(_func, _scope) },
    f_reassign_a => { self.reassign(_func, _scope) }
  },
  {"$", COMMON, Exact(1),
    reference => { Ok(self.get_var(&arg!(_func, (Str(Lit(x))) => x), _scope)?.val.val.clone()) },
    reference_f => { Ok(self.get_var(&arg!(_func, (Str(Lit(x))) => x), _scope)?.val.val.clone()) }
  },
  {"scope", SP_SCOPE, Exact(1),
    scope => { Ok(self.eval(_func.arg()?, _scope)?.val) },
    scope_a => { Ok(self.eval(_func.arg()?, _scope)?.val) }
  }
}
