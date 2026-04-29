use crate::prelude::*;
const ENTER: &str = "EnterCriticalSection";
const LEAVE: &str = "LeaveCriticalSection";
impl Jsonpiler {
  pub(crate) fn assign(
    &mut self,
    is_global_opt: Option<bool>,
    (var, val): KeyVal,
    scope: &mut Scope,
  ) -> ErrOR<bool> {
    let reassign = if let Some(is_g) = is_global_opt {
      self.check_defined(&var, var.pos, scope)?;
      Err(is_g)
    } else {
      let ref_val = self.get_var(&var, scope)?;
      if ref_val.as_type() != val.val.as_type() {
        return Err(type_err(
          format!("Variable `{}`", var.val),
          vec![ref_val.as_type()],
          val.map_ref(Json::as_type),
        ));
      }
      Ok(
        ref_val
          .memory()
          .ok_or_else(|| Compilation(UndefinedVar(var.val.clone()), vec![var.pos]))?,
      )
    };
    let is_global = reassign.is_err_and(|is_g| is_g);
    let call_once = scope.loop_labels.is_empty() && scope.epilogue.is_none();
    let data_sect = is_global && call_once;
    let value = match &val.val {
      Bool(Lit(lit)) if data_sect => Bool(Var(self.global_b(*lit))),
      Int(Lit(int)) if data_sect => Int(Var(self.global_q(int.cast_unsigned()))),
      Float(Lit(lit)) if data_sect => Float(Var(self.global_q(lit.to_bits()))),
      Null(_) | Array(_) | Bool(_) | Float(_) | Int(_) | Object(_) | Str(_) => {
        if is_global {
          self.critical_sect(scope, ENTER);
        }
        let val_type = val.val.as_type();
        let size = val_type.mem_type(val.pos)?.size();
        let memory = match reassign {
          Ok(memory) => {
            self.heap_free(memory, scope);
            memory
          }
          Err(is_g) => Memory(
            if is_g {
              Global(self.bss(u32::try_from(size)?, u32::try_from(size)?))
            } else {
              Local(Long, scope.alloc(size, size)?)
            },
            MemoryType {
              heap: Value,
              size: Small(match size {
                1 => RB,
                4 => RD,
                8 => RQ,
                _ => return err!(val.pos, UnsupportedType(val_type.name())),
              }),
            },
          ),
        };
        let value = val_type.to_json(val.pos, memory.0)?;
        scope.extend(&self.mov_json(Rax, val.clone(), Some(scope.id))?);
        scope.extend(&ret_memory(memory, Rcx, Rax)?);
        self.drop_json(val.val, false, scope);
        if is_global {
          self.critical_sect(scope, LEAVE);
        }
        value
      }
    };
    if let Err(is_g) = reassign {
      let target = if is_g { &mut self.globals } else { scope.innermost() };
      target.insert(var.val.clone(), var.pos.with(Variable::new(value)));
    }
    Ok(reassign.is_ok())
  }
  fn critical_sect(&mut self, scope: &mut Scope, action: &'static str) {
    let critical_section = Global(self.get_critical_section());
    let action_cs = self.import(KERNEL32, action);
    scope.extend(&[LeaRM(Rcx, critical_section), CallApi(action_cs)]);
  }
  pub(crate) fn declare(
    &mut self,
    is_global: bool,
    func: &mut Pos<BuiltIn>,
    scope: &mut Scope,
  ) -> ErrOR<Json> {
    let mut assign_expr = arg!(func, (Object(Lit(x))) => x);
    if assign_expr.val.len() == 1
      && let (name, Pos { val: Array(Lit(mut expr)), .. }) = take(&mut assign_expr.val[0])
      && name.val == "="
      && expr.len() == 2
    {
      let var = take(&mut expr[0]).into_ident("Variable name")?;
      let val = self.eval(take(&mut expr[1]), scope)?;
      self.assign(Some(is_global), (var, val), scope)?;
      Ok(Null(Lit(())))
    } else {
      Err(type_err(
        func.val.name.clone(),
        vec![CustomT("Assign expression".into())],
        assign_expr.pos.with(ObjectT),
      ))
    }
  }
}
built_in! {self, func, scope, variable;
  assign_global => {"global", SPECIAL, Exact(1), { self.declare(true, func, scope) }},
  assign_local => {"let", SPECIAL, Exact(1), { self.declare(false, func, scope) }},
  reassign => {"=", SPECIAL, Exact(2), {
    let var = func.arg()?.into_ident("Variable name")?;
    let val = self.eval(func.arg()?, scope)?;
    if self.assign(None, (var.clone(), val), scope)? {
      Ok(Null(Lit(())))
    } else {
      err!(var.pos, UndefinedVar(var.val))
    }
  }},
  reference => {"$", COMMON, Exact(1), {
    self.get_var(&arg!(func, (Str(Lit(x))) => x), scope)
  }},
  scope => {"scope", SP_SCOPE, Exact(1), {
    Ok(self.eval(func.arg()?, scope)?.val)
  }}
}
