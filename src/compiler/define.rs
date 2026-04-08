use crate::prelude::*;
use core::iter;
built_in! {self, func, scope, define;
  f_define => {"define", SPECIAL, Exact(4), {
    let id = self.id();
    let old_scope = scope.change(id);
    let name = func.arg()?.into_ident("Function name")?;
    self.check_defined(&name.val, name.pos, scope)?;
    let type_annotations = arg_custom!(
      self, func, vec![CustomT("TypeAnnotations".into())], (Object(Lit(x))) => x
    );
    let mut params = vec![];
    let mut args = vec![];
    for (var_name, param) in type_annotations.val {
      let param_type_str = param.into_ident("Type annotation")?;
      let json_type = JsonType::from_string(&param_type_str.val);
      let mem_type = json_type.mem_type(param_type_str.pos)?;
      let arg = Local(Long, scope.alloc(mem_type.size(), mem_type.size())?);
      let json = json_type.to_json(param_type_str.pos, arg)?;
      scope.innermost().insert(
        var_name.val, var_name.pos.with(Variable::new(json.clone()))
      );
      args.push(Memory(arg, mem_type));
      params.push(json_type);
    }
    scope.update_args_count(len_u32(&params)?);
    let ret_type = JsonType::from_string(&func.arg()?.into_ident("Type annotation")?.val);
    let epilogue = self.id();
    scope.epilogue = Some((epilogue, ret_type.clone()));
    self.user_defined.insert(name.val.clone(), name.pos.with(UserDefinedInfo { id, params, ret_type: ret_type.clone(), uses: vec![] }));
    let ret = self.eval(func.arg()?, scope)?;
    if ret_type != ret.val.as_type() {
      let ret_val = format!("Function `{}`'s return value", name.val);
      return Err(type_err(ret_val, vec![ret_type], ret.map_ref(Json::as_type)));
    }
    let tmp = scope.alloc(8, 8)?;
    scope.extend(&self.mov_json(Rax, ret.clone(), Some(scope.id))?);
    scope.push(mov_q(Local(Tmp, tmp), Rax));
    self.drop_json(ret.val, scope, false);
    self.drop_all_scope(scope);
    scope.push(mov_q(Rax, Local(Tmp, tmp)));
    scope.free(tmp, Size(8));
    scope.check_free()?;
    let stack_size = scope.resolve_stack_size()?;
    let mut insts = vec![];
    for (idx, Memory(addr, size)) in args.into_iter().enumerate() {
      let tmp_reg = *REGS.get(idx).unwrap_or(&Rax);
      if tmp_reg == Rax {
        insts.push(mov_q(Rax, Local(Tmp, i32::try_from(idx * 8 + 16)?)));
      }
      insts.push(
        if size == Size(1) {
          mov_b(addr, tmp_reg)
        } else {
          mov_q(addr, tmp_reg)
        }
      );
    }
    insts.extend_from_slice(&scope.replace(old_scope));
    insts.push(Lbl(epilogue));
    self.link_function(id, &insts, stack_size);
    Ok(Null(Lit(())))
  }},
  ret => {"ret", COMMON, Exact(1), {
    let ret = func.arg()?;
    let Some((epilogue, ret_type)) = scope.epilogue.as_ref() else {
      return err!(ret.pos, OutSideError { kind: func.name.clone(), place: "function" });
    };
    let epi = *epilogue;
    if *ret_type != ret.val.as_type() {
      let ret_val = format!("Function `{}`'s return value", func.name);
      return Err(type_err(ret_val, vec![ret_type.clone()], ret.map_ref(Json::as_type)));
    }
    for locals in scope.locals.clone().into_iter().chain(iter::once(scope.local_top.clone())) {
      for local in locals.into_values() {
        if let Some(Memory(addr @ Local(..), Heap(_))) =  local.val.val.memory()
        {
          self.heap_free(addr, scope);
        }
      }
    }
    scope.extend(&self.mov_json(Rax, ret, Some(scope.id))?);
    scope.push(Jmp(epi));
    Ok(Null(Lit(())))
  }},
}
