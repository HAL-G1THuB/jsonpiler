use crate::{
  built_in, err, take_arg, warn, Arity::{AtLeast, Exactly}, AsmFunc, Bind::{Lit, Var}, ErrOR, FuncInfo, Inst::*, Json, Jsonpiler, OpQ::{Args, Iq, Mq, Rq}, Position, Reg::*, ScopeInfo, VarKind::Global, WithPos, Label
};
use core::mem::{replace, take, discriminant};
built_in! {self, func, scope, control;
  define => {"define", SPECIAL, Exactly(4), {
    let old_scope = replace(scope, ScopeInfo::new());
    let (name, name_pos) = take_arg!(self, func, 2, "String (Literal)", Json::String(Lit(x)) => x);
    if self.builtin.contains_key(&name) {
      return err!(self, name_pos, "Name conflict with a built-in function.");
    }
    if self.user_defined.contains_key(&name) {
      return err!(self, name_pos, "Redefinition of user-defined function is not allowed.");
    }
    let type_annotations = take_arg!(self, func, 2, "TypeAnnotations", Json::Object(Lit(x)) => x).0;
    let mut params = vec![];
    for (idx, type_annotation) in type_annotations.into_iter().enumerate() {
      let (WithPos { value: param_name, .. }, param_jwp) = type_annotation;
      let Json::String(Lit(param_type)) = param_jwp.value else {
        return err!(
          self,
          param_jwp.pos,
          "Parameter types must be strings in type annotations."
        );
      };
      let local = scope.local(if &param_type == "Bool" { 1 } else { 8 })?;
      if let Some(&reg) = Jsonpiler::REGS.get(idx) {
        scope.push(if &param_type == "Bool" { MovMbRb(local.kind, Rax) }else {MovQQ(Mq(local.kind), Rq(reg))});
      } else {
        scope.push(MovQQ(Rq(Rax), Args(8 * (idx - 4))));
        scope.push(MovQQ(Mq(local.kind), Rq(Rax)));
      }
      let json_type = self.json_from_string(&param_type, param_jwp.pos, local)?;
      scope.innermost_scope()?
      .insert(param_name, json_type.clone());
    params.push(json_type);
  }
    let (ret_str, ret_pos) = take_arg!(self, func, 2, "String (Literal)", Json::String(Lit(x)) => x);
    let ret = self.json_from_string(&ret_str, ret_pos, scope.local(8)?)?;
    let id = self.gen_id();
    self.user_defined.insert(name.clone(), AsmFunc { id, params, ret: ret.clone() });
    let reg_align = scope.reg_align()?;
    let (object, object_pos) = take_arg!(self, func, 2, "Sequence", Json::Object(Lit(x)) => x);
    let ret_val = WithPos{value: self.eval_object(object, object_pos, scope)?, pos: object_pos};
    if discriminant(&ret) != discriminant(&ret_val.value){
      return Err(self.parser.type_err(
            2,
            &format!("Return value of function `{name}`"),
            &ret.type_name(),
            &ret_val,
          )
          .into()
          );
    }
    self.insts.push(Lbl(id));
    let size = scope.resolve_stack_size(reg_align)?;
    let regs = scope.take_regs();
    for reg in &regs {
      self.insts.push(Push(*reg));
    }
    self.insts.push(Push(Rbp));
    self.insts.push(MovQQ(Rq(Rbp), Rq(Rsp)));
    self.insts.push(SubRId(Rsp, size));
    let new_scope =  replace(scope, old_scope);
    for body in new_scope.into_iter_code() {
      self.insts.push(body);
    }
    self.mov_from_args(&ret_val)?;
    self.insts.push(MovQQ(Rq(Rsp), Rq(Rbp)));
    self.insts.push(Pop(Rbp));
    for reg in regs.iter().rev() {
      self.insts.push(Pop(*reg));
    }
    self.insts.push(Ret);
    Ok(Json::Null)
  }},
  f_if => {"if", SP_SCOPE, AtLeast(1), {
    let mut used_true = false;
    let if_end_label = self.gen_id();
    for idx in 1..=func.len {
      let (mut cond_then_pair, pos) =
        take_arg!(self, func, idx, "Array[Bool, Any] (Literal)", Json::Array(Lit(x)) => x);
      if used_true {
        warn!(
          self,
          pos,
          concat!(
            "Expressions in clauses following a clause ",
            "with a literal `true` condition are not evaluated at runtime, ",
            "but they are still present and parsed."
          )
        );
        break;
      }
      if cond_then_pair.len() != 2 {
        return err!(
          self,
          pos,
          "Each 'if' clause must have exactly two elements: a condition and a then expression."
        );
      }
      let mut cond_jwp = cond_then_pair.remove(0);
      let then_jwp = cond_then_pair.remove(0);
      let Json::Bool(cond_bool) = self.eval(take(&mut cond_jwp), scope)? else {
        return Err(self.parser.type_err(idx, &func.name, "Bool", &cond_jwp).into());
      };
      let Json::Object(Lit(object)) = then_jwp.value else {
        return Err(self.parser.type_err(idx, &func.name, "Sequence", &then_jwp).into());
      };
      match cond_bool {
        Lit(l_bool) => {
          if l_bool {
            self.eval_object(object, pos, scope)?;
            used_true = true;
          } else {
            warn!(
              self,
              then_jwp.pos,
              "Expressions in clauses with a literal `false` condition are not evaluated at runtime, but they are still passed as arguments to the `if` function."
            );
          }
        }
        Var(cond_label) => {
          func.sched_free_tmp(&cond_label);
          scope.push(MovRbMb(Rax, cond_label.kind));
          scope.push(TestRbRb(Rax, Rax));
          let next_label = self.gen_id();
          scope.push(Jze(next_label));
          let then_result = self.eval_object(object, pos, scope)?;
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
    let (body_arg, body_pos) =
      take_arg!(self, func, 2, "Sequence", Json::Object(Lit(x)) => x);
    let while_start = self.gen_id();
    let while_end   = self.gen_id();
    scope.push(Lbl(while_start));
    let Json::Bool(cond_bool) = self.eval(take(&mut cond_jwp), scope)? else {
      return Err(self.parser.type_err(1, &func.name, "Bool", &cond_jwp).into());
    };
    match cond_bool {
      Lit(l_bool) => {
        if l_bool {
          warn!(self, cond_jwp.pos,
            concat!(
              "This while loop will never terminate ",
              "because the condition is a literal true ",
              "and break is not implemented."
            )
          );
          let body_result = self.eval_object(body_arg, body_pos, scope)?;
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
        scope.push(MovRbMb(Rax, cond_label.kind));
        scope.push(TestRbRb(Rax, Rax));
        scope.push(Jze(while_end));
        let body_result = self.eval_object(body_arg, body_pos, scope)?;
        scope.drop_json(body_result)?;
        scope.push(Jmp(while_start));
      }
    }
    scope.push(Lbl(while_end));
    Ok(Json::Null)
  }},
}
impl Jsonpiler {
  fn json_from_string(&self, name: &str, pos: Position, local: Label) -> ErrOR<Json>{
    match name {
        "String" => Ok(Json::String(Var(local))),
        "Int" => Ok(Json::Int(Var(local))),
        "Float" => Ok(Json::Float(Var(local))),
        "Null" => Ok(Json::Null),
        "Bool" => Ok(Json::Bool(Var(local))),
        "Object" | "Array" =>
          err!(self, pos, "Unsupported type as parameter or return value"),
        _ => err!(self, pos, "Unknown type"),
      }
  }
  fn mov_from_args(&mut self, jwp: &WithPos<Json>) -> ErrOR<()> {
    {
      match &jwp.value {
        Json::String(Lit(l_str)) => {
          let mem = Global { id: self.global_str(l_str.to_owned()) };
          self.insts.push(LeaRM(Rax, mem));
        }
        Json::String(Var(label))
        | Json::Float(Var(label))
        | Json::Bool(Var(label))
        | Json::Int(Var(label)) => self.insts.push(MovQQ(Rq(Rax), Mq(label.kind))),
        Json::Null => self.insts.push(Clear(Rax)),
        #[expect(clippy::cast_sign_loss)]
        Json::Int(Lit(l_int)) => self.insts.push(MovQQ(Rq(Rax), Iq(*l_int as u64))),
        Json::Bool(Lit(l_bool)) => self.insts.push(MovRbIb(Rax, if *l_bool { 0xFF } else { 0 })),
        Json::Float(Lit(l_float)) => self.insts.push(MovQQ(Rq(Rax), Iq(l_float.to_bits()))),
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
