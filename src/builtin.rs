crate::def_mod_and_register! {arithmetic, control, evaluate, logical, output, variable}
impl Jsonpiler {
  pub(crate) fn eval(&mut self, json: Json, scope: &mut ScopeInfo) -> ErrOR<Json> {
    if let Json::Array(Lit(list)) = json {
      Ok(Json::Array(Lit(self.eval_args(list, scope)?)))
    } else if let Json::Object(Lit(mut object)) = json {
      Ok(self.eval_object(&mut object, scope)?)
    } else {
      Ok(json)
    }
  }
  fn eval_args(
    &mut self, mut args: Vec<WithPos<Json>>, scope: &mut ScopeInfo,
  ) -> ErrOR<Vec<WithPos<Json>>> {
    for arg in &mut args {
      arg.value = self.eval(take(&mut arg.value), scope)?;
    }
    Ok(args)
  }
  fn eval_func(
    &mut self, scope: &mut ScopeInfo, key: WithPos<String>, val: WithPos<Json>,
  ) -> ErrOR<Json> {
    let WithPos { value: name, pos } = key;
    if let Some(builtin) = self.builtin.get(&name) {
      let Builtin { scoped, skip_eval, func, arg_len } = *builtin;
      let mut maybe_tmp = None;
      if scoped {
        maybe_tmp = Some(scope.begin()?);
      }
      let args: VecDeque<WithPos<Json>> = if let Json::Array(Lit(arr)) = val.value {
        if skip_eval { arr } else { self.eval_args(arr, scope)? }
      } else if skip_eval {
        vec![val]
      } else {
        self.eval_args(vec![val], scope)?
      }
      .into();
      let len = args.len();
      let free_list = vec![];
      let mut func_info = FuncInfo { args, free_list, len, name, pos };
      self.parser.validate_args(&func_info, arg_len)?;
      let result = func(self, &mut func_info, scope)?;
      for label in func_info.free_list {
        scope.free(label)?;
      }
      if let Some(tmp) = maybe_tmp {
        scope.end(tmp)?;
      }
      Ok(result)
    } else if let Some(Json::Function(asm_func)) = self.get_var(&name, scope) {
      scope.body.push(mn!("call", asm_func.label.to_ref()));
      if let Json::Int(_) = *asm_func.ret {
        return Ok(Json::Int(Var(scope.mov_tmp("rax")?)));
      }
      Ok(Json::Null)
    } else {
      return err!(self, val.pos, "Undefined function: `{name}`");
    }
  }
  fn eval_object(
    &mut self, object: &mut Vec<(WithPos<String>, WithPos<Json>)>, scope: &mut ScopeInfo,
  ) -> ErrOR<Json> {
    for (key, val) in object.drain(..object.len().saturating_sub(1)) {
      let tmp_json = self.eval_func(scope, key, val)?;
      scope.drop_json(tmp_json)?;
    }
    let (key, val) = take(&mut object[0]);
    self.eval_func(scope, key, val)
  }
  pub(crate) fn register(
    &mut self, name: &str, (scoped, skip_eval): (bool, bool), func: JFunc, arg_len: ArgLen,
  ) {
    self.builtin.insert(name.to_owned(), Builtin { arg_len, func, scoped, skip_eval });
  }
  #[inline]
  pub fn run(&mut self) -> ErrOR<()> {
    let json = self.parser.parse()?;
    mn_write!(self.data, ".intel_syntax", "noprefix");
    let msg = include_str!("txt/SEH_HANDLER_MSG.txt");
    self.data.write_all(format!(include_str!("asm/once/data.s"), msg = msg).as_bytes())?;
    self.register_all();
    let mut scope = ScopeInfo::new();
    let result = self.eval(json.value, &mut scope)?;
    self.data.write_all(include_bytes!("asm/once/bss.s"))?;
    for (id, size) in &self.bss {
      writeln!(self.data, "\t.lcomm\t.L{id},\t{size}")?;
    }
    self.data.write_all(include_bytes!("asm/once/main.s"))?;
    write!(
      self.data,
      include_str!("asm/common/prologue.s"),
      size = format!("{:#x}", scope.calc_alloc(8)?)
    )?;
    self.data.write_all(include_bytes!("asm/once/startup.s"))?;
    for body in &scope.body {
      self.data.write_all(body.as_bytes())?;
    }
    if let Json::Int(int) = result {
      mn_write!(self.data, "mov", "rcx", get_int_str_without_free(&int));
    } else {
      mn_write!(self.data, "xor", "ecx", "ecx");
    }
    self.data.write_all(imp_call("ExitProcess").as_bytes())?;
    mn_write!(self.data, ".seh_endproc");
    for text in &self.text {
      self.data.write_all(text.as_bytes())?;
    }
    self.data.write_all(include_bytes!("asm/once/handler.s"))?;
    self.data.flush()?;
    Ok(())
  }
}
