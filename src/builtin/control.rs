use crate::{
  Arity::{AtLeast, Exactly},
  AsmFunc,
  Bind::{Lit, Var},
  ErrOR, FuncInfo, Json, Jsonpiler, ScopeInfo, WithPos, built_in, err, mn, take_arg,
  utility::get_argument_mem,
  warn,
};
use core::mem::{replace, take};
use std::collections::HashMap;
built_in! {self, func, scope, control;
  f_if => {"if", SP_SCOPE, AtLeast(1), {
    let mut used_true = false;
    let if_end_label = self.ctx.global(8)?;
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
      let mut then_jwp = cond_then_pair.remove(0);
      let Json::Bool(cond_bool) = self.eval(take(&mut cond_jwp), scope)? else {
        return Err(self.parser.type_err(idx, &func.name, "Bool", &cond_jwp).into());
      };
      let Json::Object(Lit(then_object)) = then_jwp.value else {
        return Err(self.parser.type_err(idx, &func.name, "Sequence", &then_jwp).into());
      };
      match cond_bool {
        Lit(l_bool) => {
          if l_bool {
            then_jwp.value = self.eval_object(then_object, then_jwp.pos, scope)?;
            used_true = true;
            scope.body.push(if_end_label.to_def());
            continue;
          }
          warn!(
            self,
            then_jwp.pos,
            "Expressions in clauses with a literal `false` condition are not evaluated at runtime, but they are still passed as arguments to the `if` function."
          );
        }
        Var(cond_label) => {
          let next_clause_label =
            if idx == func.len { &if_end_label } else { &self.ctx.global(8)? };
          func.sched_free_tmp(&cond_label);
          scope.body.push(mn!("mov", "al", cond_label));
          scope.body.push(mn!("test", "al", "al"));
          scope.body.push(mn!("jz", next_clause_label.to_ref()));
          let then_result = self.eval_object(then_object, then_jwp.pos, scope)?;
          scope.drop_json(then_result)?;
          scope.body.push(mn!("jmp", if_end_label.to_ref()));
          scope.body.push(next_clause_label.to_def());
        }
      }
    }
    Ok(Json::Null)
  }},
  lambda => {"lambda", SPECIAL, Exactly(2), {
    let mut tmp_local_scope = replace(&mut scope.locals, vec![HashMap::new()]);
    let mut inner_scope = ScopeInfo::new();
    let type_annotations = take_arg!(self, func, 2, "TypeAnnotations", Json::Object(Lit(x)) => x).0;
    let mut params = vec![];
    for (num, type_annotation) in type_annotations.into_iter().enumerate() {
      let (WithPos { value: param_name, .. }, param_type_jwp) = type_annotation;
      let Json::String(Lit(param_type)) = param_type_jwp.value else {
        return err!(
          self,
          param_type_jwp.pos,
          "Parameter types must be strings in type annotations."
        );
      };
      let local = inner_scope.local(if &param_type == "Bool" { 1 } else { 8 })?;
      inner_scope.body.push(mn!("mov", local, get_argument_mem(num, local.size)?));
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
      inner_scope
        .locals
        .last_mut()
        .ok_or("InternalError: Invalid scope.")?
        .insert(param_name, json_type.clone());
      params.push(json_type);
    }
    let (object, object_pos) = take_arg!(self, func, 2, "Sequence", Json::Object(Lit(x)) => x);
    let ret = WithPos{value: self.eval_object(object, object_pos, &mut inner_scope)?, pos: object_pos};
    let label = self.ctx.global(8)?;
    self.text.push(mn!(".seh_proc", label.to_ref()));
    self.text.push(label.to_def());
    let size = format!("{:#x}", inner_scope.calc_alloc((inner_scope.reg_used.len() & 1) << 3)?);
    for reg in &inner_scope.reg_used {
      self.text.push(mn!("push", reg));
      self.text.push(mn!(".seh_pushreg", reg));
    }
    self.text.push(mn!("push", "rbp"));
    self.text.push(mn!(".seh_pushreg", "rbp"));
    self.text.push(format!(include_str!("../asm/common/prologue.s"), size = size));
    for body in inner_scope.body {
      self.text.push(body);
    }
    let ret_str = self.get_argument(&ret)?;
    self.text.push(mn!("mov", "rax", ret_str));
    self.text.push(mn!("mov", "rsp", "rbp"));
    self.text.push(mn!("pop", "rbp"));
    for reg in inner_scope.reg_used.iter().rev() {
      self.text.push(mn!("pop", reg));
    }
    self.text.push(mn!("ret"));
    self.text.push(mn!(".seh_endproc"));
    scope.locals = take(&mut tmp_local_scope);
    Ok(Json::Function(AsmFunc { label, params, ret: Box::new(ret.value) }))
  }}
}
