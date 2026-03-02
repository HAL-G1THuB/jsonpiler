use crate::prelude::*;
built_in! {self, func, scope, define;
    f_define => {"define", SPECIAL, Exactly(4), {
    let old_scope = take(scope);
    let WithPos { val: name, pos } = arg!(self, func, (Str(Lit(x))) => x);
    if self.builtin.contains_key(&name) {
      return err!(pos, ExistentFunc(Builtin, name));
    }
    if self.user_defined.contains_key(&name) {
      return err!(pos, ExistentFunc(UserDefined, name));
    }
    let type_annotations = arg_custom!(self, func, "TypeAnnotations", (Object(Lit(x))) => x);
    let mut params = vec![];
    let mut args = vec![];
    for (var_name, param_jwp) in type_annotations.val {
      let param_type = unwrap_arg!(self, param_jwp, func, "TypeAnnotation", (Str(Lit(x))) => x);
      let size = match param_type.val.as_ref() { "Bool" => 1, _ => 8 };
      let arg = Local(Long, scope.alloc(size, size)?);
      let json_type = json_from_string(param_type, arg)?;
      let label_size = if matches!(json_type, Str(_)) {
        Heap
      } else {
        Size(size)
      };
      scope.locals.last_mut().unwrap_or(&mut scope.local_top).insert(var_name.val, json_type.clone());
      args.push(Label(arg, label_size));
      params.push(json_type);
    }
    scope.update_args_count(u32::try_from(params.len())?);
    let local = Local(Tmp, scope.alloc(8, 8)?);
    let ret = json_from_string(arg!(self, func, (Str(Lit(x))) => x), local)?;
    let id = self.id();
    let epilogue = self.id();
    let end = self.id();
    scope.epilogue = Some((epilogue, ret.clone()));
    self.user_defined.insert(name.clone(), func.pos.with(AsmFunc { id, params, ret: ret.clone() }));
    let object = arg_custom!(self, func, "Block", (Object(Lit(x))) => x);
    let ret_jwp = object.pos.with(self.eval_object(object.val, scope)?);
    if discriminant(&ret) != discriminant(&ret_jwp.val) {
      return Err(type_err(format!("Return value of `{name}`"), ret.describe(), &ret_jwp));
    }
    scope.extend(&self.mov_deep_json(Rax, ret_jwp)?);
    let stack_size = scope.resolve_stack_size()?;
    self.insts.extend_from_slice(&[Lbl(id), Push(Rbp), mov_q(Rbp, Rsp), SubRId(Rsp, stack_size)]);
    for (idx, Label(addr, size)) in args.into_iter().enumerate() {
      let tmp_reg = *REGS.get(idx).unwrap_or(&Rax);
      if tmp_reg == Rax {
        self.insts.push(mov_q(Rax, Local(Tmp, i32::try_from(idx * 8 + 16)?)));
      }
      self.insts.push(
        if matches!(size,  Size(1)) {
          mov_b(addr, tmp_reg)
        } else {
          mov_q(addr, tmp_reg)
        }
      );
    }
    extend!(
      self.insts,
      replace(scope, old_scope).body,
      [Lbl(epilogue), mov_q(Rsp, Rbp), Pop(Rbp), Custom(RET), Lbl(end)]
    );
    self.data_insts.push(Seh(id, end, stack_size));
    Ok(Null)
  }},
  ret => {"ret", COMMON, Exactly(1), {
    let ret = func.arg()?;
    let Some((epilogue, json)) = scope.epilogue.as_ref() else {
      return err!(ret.pos, OutSideError { kind: func.name.clone(), place: "function" });
    };
    let epi = *epilogue;
    if discriminant(json) != discriminant(&ret.val){
      return Err(type_err(format!("Return value of `{}`", func.name), json.describe(), &ret));
    }
    scope.extend(&self.mov_deep_json(Rax, ret)?);
    scope.push(Jmp(epi));
    Ok(Null)
  }},
}
fn json_from_string(name: WithPos<String>, local: Address) -> ErrOR<Json> {
  Ok(match name.val.as_ref() {
    "Str" => Str(Var(Label(local, Heap))),
    "Int" => Int(Var(Label(local, Size(8)))),
    "Float" => Float(Var(Label(local, Size(8)))),
    "Null" => Null,
    "Bool" => Bool(Var(Label(local, Size(1)))),
    "Object" | "Array" => return err!(name.pos, UnsupportedType(name.val)),
    unknown => return err!(name.pos, UnknownType(unknown.into())),
  })
}
