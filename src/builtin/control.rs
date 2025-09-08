use crate::{
  Arity::{AtLeast, Exactly},
  AsmFunc,
  Bind::{Lit, Var},
  ConditionCode::*,
  ErrOR, FuncInfo,
  Inst::*,
  Json, Jsonpiler, Label,
  LogicByteOpcode::*,
  Memory::Global,
  Operand::Args,
  Position,
  Register::*,
  ScopeInfo, WithPos, built_in, err, take_arg, unwrap_arg,
  utility::{mov_b, mov_q},
  warn,
};
use core::mem::{discriminant, replace, take};
built_in! {self, func, scope, control;
  define => {"define", SPECIAL, Exactly(4), {
    let old_scope = replace(scope, ScopeInfo::new());
    let name = take_arg!(self, func, "String (Literal)", Json::String(Lit(x)) => x);
    if self.builtin.contains_key(&name.value) {
      return err!(self, name.pos, "DefineError: `{}` exists as a built-in function", name.value);
    }
    if self.user_defined.contains_key(&name.value) {
      return err!(self, name.pos, "DefineError: `{}` exists as a user-defined function", name.value);
    }
    let type_annotations = take_arg!(self, func, "TypeAnnotations", Json::Object(Lit(x)) => x);
    if type_annotations.value.len() >= 16 {
      return err!(self, type_annotations.pos, "ArityError: Up to 16 arguments are allowed.");
    }
    let mut params = vec![];
    let mut args = vec![];
    for type_annotation in type_annotations.value {
      let (WithPos { value: param_name, .. }, param_jwp) = type_annotation;
      let Json::String(Lit(param_type)) = param_jwp.value else {
        return err!(
          self,
          param_jwp.pos,
          "Parameter types must be strings in type annotations."
        );
      };
      let size = match param_type.as_ref() { "Bool" => 1, _ => 8};
      let local = scope.local(size, size)?;
      let json_type = self.json_from_string(&param_type, param_jwp.pos, local)?;
      scope.innermost_scope()?
      .insert(param_name, json_type.clone());
      args.push(local);
      params.push(json_type);
    }
    let ret_str = take_arg!(self, func, "String (Literal)", Json::String(Lit(x)) => x);
    let ret_val = self.json_from_string(&ret_str.value, ret_str.pos, scope.local(8, 8)?)?;
    let id = self.gen_id();
    let epilogue = self.gen_id();
    scope.set_epilogue(Some((epilogue, ret_val.clone())));
    self.user_defined.insert(name.value.clone(), AsmFunc { id, params, ret: ret_val.clone(), file: func.pos.file });
    let object = take_arg!(self, func, "Block", Json::Object(Lit(x)) => x);
    let ret_jwp = WithPos{value: self.eval_object(object.value, object.pos, scope)?, pos: object.pos};
    if discriminant(&ret_val) != discriminant(&ret_jwp.value){
      return Err(self.parser[ret_jwp.pos.file].type_error(
            &format!("Return value of function `{}`", name.value),
            &ret_val.type_name(),
            &ret_jwp,
          ).into()
        );
      }
      self.return_value(&ret_jwp, scope)?;
    self.insts.push(Lbl(id));
    let size = scope.resolve_stack_size()?;
    self.insts.push(Push(Rbp));
    self.insts.push(mov_q(Rbp, Rsp));
    self.insts.push(SubRId(Rsp, size));
    for (idx, local) in args.into_iter().enumerate() {
      let reg = *Jsonpiler::REGS.get(idx).unwrap_or(&Rax);
      if reg == Rax {
        self.insts.push(mov_q(Rax, Args(8 * idx + usize::try_from(size)? + 16)));
      }
      if local.size == 1 {
        self.insts.push(mov_b(local.mem, reg));
      } else {
        self.insts.push(mov_q(local.mem, reg));
      }
    }
    let new_scope =  replace(scope, old_scope);
    for body in new_scope.into_iter_code() {
      self.insts.push(body);
    }
    self.insts.push(Lbl(epilogue));
    self.insts.push(mov_q(Rsp, Rbp));
    self.insts.push(Pop(Rbp));
    self.insts.push(Custom(&Jsonpiler::RET));
    Ok(Json::Null)
  }},
  f_break => {"break", COMMON, Exactly(0), {
    let Some(&(_, end_label)) = scope.loop_label() else {
      return err!(self, func.pos, "ScopeError: `break` outside of loop.");
    };
    scope.push(Jmp(end_label));
    Ok(Json::Null)
  }},
  f_continue => {"continue", COMMON, Exactly(0), {
    let Some(&(start_label, _)) = scope.loop_label() else {
      return err!(self, func.pos, "ScopeError: `continue` outside of loop.");
    };
    scope.push(Jmp(start_label));
    Ok(Json::Null)
  }},
  f_if => {"if", SP_SCOPE, AtLeast(1), {
    let mut used_true = false;
    let if_end_label = self.gen_id();
    for _ in 0..func.len {
      let mut cond_then =
        take_arg!(self, func, "Array[Bool, Any] (Literal)", Json::Array(Lit(x)) => x);
      if used_true {
        warn!(self, cond_then.pos, "Blocks in subsequent clauses are not evaluated");
        break;
      }
      if cond_then.value.len() != 2 {
        return Err(self.parser[cond_then.pos.file].type_error(
          "Each `if` clause",
          "Array[Bool, Any] (Literal)",
          &WithPos{ pos: cond_then.pos, value:Json::Array(Lit(cond_then.value)) }
        ).into());
      }
      let mut cond_jwp = cond_then.value.remove(0);
      let then_jwp = cond_then.value.remove(0);
      let cond = WithPos{pos: cond_jwp.pos, value: self.eval(take(&mut cond_jwp), scope)?};
      let cond_bool = unwrap_arg!(self, cond, func, "Bool", Json::Bool(x) => x).value;
      let object = unwrap_arg!(self, then_jwp, func, "Block", Json::Object(Lit(x)) => x).value;
      match cond_bool {
        Lit(l_bool) => {
          if l_bool {
            self.eval_object(object, cond_then.pos, scope)?;
            used_true = true;
          } else {
            warn!(self, then_jwp.pos, "This block is passed to `if` but not evaluated");
          }
        }
        Var(cond_label) => {
          func.sched_free_tmp(&cond_label);
          scope.push(mov_b(Rax, cond_label.mem));
          scope.push(LogicRbRb(Test, Rax, Rax));
          let next_label = self.gen_id();
          scope.push(JCc(E, next_label));
          let then_result = self.eval_object(object, cond_then.pos, scope)?;
          scope.drop_json(then_result)?;
          scope.push(Jmp(if_end_label));
          scope.push(Lbl(next_label));
        }
      }
    }
    scope.push(Lbl(if_end_label));
    Ok(Json::Null)
  }},
  f_while => {"while", SP_SCOPE, Exactly(2), {
    let mut cond_jwp = func.arg()?;
    let body =
    take_arg!(self, func, "Block", Json::Object(Lit(x)) => x);
    let while_start = self.gen_id();
    let while_end   = self.gen_id();
    scope.loop_enter(while_start, while_end);
    scope.push(Lbl(while_start));
    let cond = WithPos { pos: cond_jwp.pos, value: self.eval(take(&mut cond_jwp), scope)? };
    let cond_bool = unwrap_arg!(self, cond, func, "Bool", Json::Bool(x) => x).value;
    match cond_bool {
      Lit(l_bool) => {
        if l_bool {
          let body_result = self.eval_object(body.value, body.pos, scope)?;
          scope.drop_json(body_result)?;
          scope.push(Jmp(while_start));
        } else {
          warn!(self, cond_jwp.pos,
            "While condition is a literal `false`, loop body is unreachable."
          );
        }
      }
      Var(cond_label) => {
        func.sched_free_tmp(&cond_label);
        scope.push(mov_b(Rax, cond_label.mem));
        scope.push(LogicRbRb(Test, Rax, Rax));
        scope.push(JCc(E, while_end));
        let body_result = self.eval_object(body.value, body.pos, scope)?;
        scope.drop_json(body_result)?;
        scope.push(Jmp(while_start));
      }
    }
    scope.push(Lbl(while_end));
    scope.loop_exit();
    Ok(Json::Null)
  }},
  ret => {"ret", COMMON, Exactly(1), {
    let ret_jwp = func.arg()?;
    self.return_value(&ret_jwp, scope)?;
    let Some((epilogue, json)) = scope.get_epilogue() else {
      return err!(self, ret_jwp.pos, "ScopeError: `ret` outside of function.");
    };
    if discriminant(json) != discriminant(&ret_jwp.value){
      return Err(self.parser[ret_jwp.pos.file].type_error(
            &format!("Return value of function `{}`", func.name),
            &json.type_name(),
            &ret_jwp,
          ).into()
          );
    }
    scope.push(Jmp(*epilogue));
    Ok(Json::Null)
  }},
}
impl Jsonpiler {
  fn json_from_string(&self, name: &str, pos: Position, local: Label) -> ErrOR<Json> {
    match name {
      "String" => Ok(Json::String(Var(local))),
      "Int" => Ok(Json::Int(Var(local))),
      "Float" => Ok(Json::Float(Var(local))),
      "Null" => Ok(Json::Null),
      "Bool" => Ok(Json::Bool(Var(local))),
      "Object" | "Array" => err!(self, pos, "Unsupported type as parameter or return value"),
      _ => err!(self, pos, "Unknown type"),
    }
  }
  fn return_value(&mut self, jwp: &WithPos<Json>, scope: &mut ScopeInfo) -> ErrOR<()> {
    {
      match &jwp.value {
        Json::String(string) => {
          let inst = match string {
            Lit(l_str) => {
              let id = self.global_str(l_str.clone()).0;
              LeaRM(Rax, Global { id, disp: 0i32 })
            }
            Var(str_label) => mov_q(Rax, str_label.mem),
          };
          scope.push(inst);
        }
        Json::Float(Var(label)) | Json::Bool(Var(label)) | Json::Int(Var(label)) => {
          scope.push(mov_q(Rax, label.mem));
        }
        Json::Null => scope.push(Clear(Rax)),
        #[expect(clippy::cast_sign_loss)]
        Json::Int(Lit(l_int)) => scope.push(mov_q(Rax, *l_int as u64)),
        Json::Bool(Lit(l_bool)) => {
          scope.push(mov_b(Rax, if *l_bool { 0xFF } else { 0 }));
        }
        Json::Float(Lit(l_float)) => scope.push(mov_q(Rax, l_float.to_bits())),
        Json::Array(_) | Json::Object(_) => {
          return err!(
            self,
            jwp.pos,
            "This type cannot be accepted as a return value of an user-defined function."
          );
        }
      }
    }
    Ok(())
  }
}
