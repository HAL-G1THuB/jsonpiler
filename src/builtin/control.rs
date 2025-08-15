use crate::{
  Arity::{AtLeast, Exactly},
  AsmFunc,
  Bind::{Lit, Var},
  ErrOR, FuncInfo,
  Inst::*,
  Json, Jsonpiler,
  OpQ::{Args, Iq, Mq, Rq},
  Reg::*,
  ScopeInfo,
  VarKind::Global,
  WithPos, built_in, err, take_arg, warn,
};
use core::mem::{replace, take};
built_in! {self, func, scope, control;
  f_if => {"if", SP_SCOPE, AtLeast(1), {
    let mut used_true = false;
    let if_end_label = self.ctx.gen_id();
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
          let next_label = self.ctx.gen_id();
          scope.push(JzJe(next_label));
          let then_result = self.eval_object(object, pos, scope)?;
          scope.drop_json(then_result)?;
          scope.push(Jmp(if_end_label));
          scope.push(Label(next_label));
        }
      }
    }
    scope.push(Label(if_end_label));
    Ok(Json::Null)
  }},
  lambda => {"lambda", SPECIAL, Exactly(2), {
    let old_scope = replace(scope, ScopeInfo::new());
    let type_annotations = take_arg!(self, func, 2, "TypeAnnotations", Json::Object(Lit(x)) => x).0;
    let mut params = vec![];
    for (idx, type_annotation) in type_annotations.into_iter().enumerate() {
      let (WithPos { value: param_name, .. }, param_type_jwp) = type_annotation;
      let Json::String(Lit(param_type)) = param_type_jwp.value else {
        return err!(
          self,
          param_type_jwp.pos,
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
      let json_type = match param_type.as_ref() {
        "String" => Json::String(Var(local)),
        "Int" => Json::Int(Var(local)),
        "Float" => Json::Float(Var(local)),
        "Null" => Json::Null,
        "Bool" => Json::Bool(Var(local)),
        "Function" | "Object" | "Array" => {
          return err!(self, param_type_jwp.pos, "Unsupported type as parameter");
        }
        _ => return err!(self, param_type_jwp.pos, "Unknown type"),
      };
      scope.innermost_scope()?
        .insert(param_name, json_type.clone());
      params.push(json_type);
    }
    let reg_align = scope.reg_align()?;
    let (object, object_pos) = take_arg!(self, func, 2, "Sequence", Json::Object(Lit(x)) => x);
    let ret = WithPos{value: self.eval_object(object, object_pos, scope)?, pos: object_pos};
    let id = self.ctx.gen_id();
    self.insts.push(Label(id));
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
    self.mov_from_args(&ret)?;
    self.insts.push(MovQQ(Rq(Rsp), Rq(Rbp)));
    self.insts.push(Pop(Rbp));
    for reg in regs.iter().rev() {
      self.insts.push(Pop(*reg));
    }
    self.insts.push(Ret);
    Ok(Json::Function(AsmFunc { id, params, ret: Box::new(ret.value) }))
  }}
}
impl Jsonpiler {
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
        Json::Array(_) | Json::Object(_) | Json::Function(_) => {
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
