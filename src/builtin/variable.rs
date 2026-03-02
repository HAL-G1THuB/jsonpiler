use crate::prelude::*;
const ENTER: &str = "EnterCriticalSection";
const LEAVE: &str = "LeaveCriticalSection";
impl Jsonpiler {
  pub(crate) fn assign(
    &mut self,
    func: &mut Function,
    scope: &mut Scope,
    is_global: bool,
  ) -> ErrOR<Json> {
    let WithPos { val: variable, pos: var_pos } = arg!(self, func, (Str(Lit(x))) => x);
    let WithPos { val, pos } = func.arg()?;
    let ref_json = if is_global {
      if scope.get_var_local(&variable).is_some() {
        return err!(var_pos, ExistentVar(variable));
      }
      self.globals.get(&variable).cloned()
    } else {
      scope.get_var_local(&variable)
    };
    if let Some(json) = &ref_json
      && discriminant(json) != discriminant(&val)
    {
      return Err(type_err(format!("Variable `{variable}`"), json.describe(), &pos.with(val)));
    }
    let reassign = ref_json.and_then(|mut json| json.label().copied());
    let data_sect_allowed = is_global && reassign.is_none() && scope.epilogue.is_none();
    let value = match &val {
      Null => Null,
      Bool(Lit(lit)) if data_sect_allowed => Bool(Var(self.global_b(*lit))),
      Int(Lit(int)) if data_sect_allowed => Int(Var(self.global_q(*int as u64))),
      Float(Lit(lit)) if data_sect_allowed => Float(Var(self.global_q(lit.to_bits()))),
      Array(_) | Bool(_) | Float(_) | Int(_) | Object(_) | Str(_) => {
        if is_global {
          self.critical_sect(scope, ENTER)?;
        }
        let size = if matches!(&val, Bool(_)) { 1 } else { 8 };
        let label = if let Some(label) = &reassign {
          *label
        } else if is_global {
          Label(Global(self.bss(u32::try_from(size)?, u32::try_from(size)?)), Size(size))
        } else {
          Label(Local(Long, scope.alloc(size, size)?), Size(size))
        };
        let value = match &val {
          Null => Null,
          Str(_) => Str(Var(Label(label.0, Heap))),
          Float(_) => Float(Var(label)),
          Int(_) => Int(Var(label)),
          Bool(_) => Bool(Var(label)),
          Array(_) | Object(_) => return err!(pos, UnsupportedType(val.describe())),
        };
        scope.extend(&self.mov_deep_json(Rax, pos.with(val))?);
        scope.extend(&ret_label(label, Rcx, Rax, size, matches!(&value, Str(_))));
        if is_global {
          self.critical_sect(scope, LEAVE)?;
        }
        value
      }
    };
    if reassign.is_none() {
      if is_global {
        &mut self.globals
      } else {
        scope.locals.last_mut().unwrap_or(&mut scope.local_top)
      }
      .insert(variable, value);
    }
    Ok(Null)
  }
  fn critical_sect(&mut self, scope: &mut Scope, action: &'static str) -> ErrOR<()> {
    let critical_section = Global(self.get_critical_section()?);
    let action_cs = self.import(KERNEL32, action)?;
    scope.extend(&[LeaRM(Rcx, critical_section), CallApi(action_cs)]);
    Ok(())
  }
}
built_in! {self, func, scope, variable;
  assign_global => {"global", COMMON, Exactly(2), { self.assign(func, scope, true) }},
  assign_local => {"=", COMMON, Exactly(2), { self.assign(func, scope, false) }},
  reference => {"$", COMMON, Exactly(1), {
    let WithPos { val: var_name, pos } = arg!(self, func, (Str(Lit(x))) => x);
    self.get_var(&var_name, pos, scope)
  }},
  scope => {"scope", SP_SCOPE, Exactly(1), {
    self.eval_object(arg_custom!(self, func, "Block", (Object(Lit(x))) => x).val, scope)
  }}
}
