use crate::{
  Arity::{AtLeast, Exactly},
  Bind::{Lit, Var},
  ConditionCode::*,
  ErrOR, FuncInfo,
  Inst::*,
  Json, Jsonpiler,
  OpQ::{Iq, Mq, Rq},
  Reg::*,
  ScopeInfo,
  VarKind::Global,
  built_in, err, take_arg,
  utility::{mov_float, mov_int},
};
use std::collections::hash_map::Entry::{Occupied, Vacant};
built_in! {self, func, scope, arithmetic;
  abs => {"abs", COMMON, Exactly(1), {
    self.take_int(Rax, func, scope)?;
    scope.push(Custom(Jsonpiler::CQO.to_vec()));
    scope.push(XorRR(Rax, Rdx));
    scope.push(SubRR(Rax, Rdx));
    Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
  }},
  div => {"/", COMMON, AtLeast(2), {
    let arg = func.arg()?;
    if let Json::Int(int) = arg.value {
      mov_int(&int, Rax, func, scope);
      for _ in 1..func.len {
        self.mov_rcx_nonzero(scope, func)?;
        scope.push(Custom(Jsonpiler::CQO.to_vec()));
        scope.push(IDivR(Rcx));
      }
      Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
    } else if let Json::Float(float) = arg.value {
      mov_float(&float, Rax, Rax, func, scope)?;
        for _ in 1..func.len {
          self.take_float(Rcx, Rax, func, scope)?;
          scope.push(DivSd(Rax, Rcx));
        }
        let tmp = scope.tmp(8)?;
        scope.push(MovSdMX(tmp.kind, Rax));
        Ok(Json::Float(Var(tmp)))
    } else {
      Err(self.parser.type_err(1, &func.name, "Int` or `Float", &arg).into())
    }
  }},
  int => {"Int", COMMON, Exactly(1), {
    self.take_float(Rax, Rax, func, scope)?;
    scope.push(CvtTSd2Si(Rax, Rax));
    Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
  }},
  minus => {"-", COMMON, AtLeast(1), {
    let arg = func.arg()?;
    if let Json::Int(int) = arg.value {
      mov_int(&int, Rax, func, scope);
      if func.len == 1 {
        scope.push(NegR(Rax));
      } else {
        for _ in 1..func.len {
          self.take_int(Rcx, func, scope)?;
          scope.push(SubRR(Rax, Rcx));
        }
      }
      Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
    } else if let Json::Float(float) = arg.value {
      const NEG: [u8; 5] = [0x48, 0x0F, 0xBA, 0xF0, 0x3F];
      mov_float(&float, Rax, Rax, func, scope)?;
      if func.len == 1 {
        scope.push(Custom(NEG.to_vec()));
        Ok(Json::Float(Var(scope.mov_tmp(Rcx)?)))
      } else {
        for _ in 1..func.len {
          self.take_float(Rcx, Rax, func, scope)?;
          scope.push(SubSd(Rax, Rcx));
        };
        let tmp = scope.tmp(8)?;
        scope.push(MovSdMX(tmp.kind, Rax));
        Ok(Json::Float(Var(tmp)))
      }
    } else {
      Err(self.parser.type_err(1, &func.name, "Int` or `Float", &arg).into())
    }}},
  mul => {"*", COMMON, AtLeast(2), {
    let arg = func.arg()?;
    if let Json::Int(int) = arg.value {
      mov_int(&int, Rax, func, scope);
      for _ in 1..func.len {
        self.take_int(Rcx, func, scope)?;
        scope.push(IMulRR(Rax, Rcx));
      }
      Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
    } else if let Json::Float(float) = arg.value {
      mov_float(&float, Rax, Rax, func, scope)?;
      for _ in 1..func.len {
        self.take_float(Rcx, Rax, func, scope)?;
        scope.push(MulSd(Rax, Rcx));
      };
      let tmp = scope.tmp(8)?;
      scope.push(MovSdMX(tmp.kind, Rax));
      Ok(Json::Float(Var(tmp)))
    } else {
      Err(self.parser.type_err(1, &func.name, "Int` or `Float", &arg).into())
  }}},
  plus => {"+", COMMON, AtLeast(2), {
    let arg = func.arg()?;
    if let Json::Int(int) = arg.value {
      mov_int(&int, Rax, func, scope);
      for _ in 1..func.len {
        self.take_int(Rcx, func, scope)?;
        scope.push(AddRR(Rax, Rcx));
      }
      Ok(Json::Int(Var(scope.mov_tmp(Rax)?)))
    } else if let Json::Float(float) = arg.value {
      mov_float(&float, Rax, Rax, func, scope)?;
      for _ in 1..func.len {
        self.take_float(Rcx, Rax, func, scope)?;
        scope.push(AddSd(Rax, Rcx));
      };
      let tmp = scope.tmp(8)?;
      scope.push(MovSdMX(tmp.kind, Rax));
      Ok(Json::Float(Var(tmp)))
    } else {
      Err(self.parser.type_err(1, &func.name, "Int` or `Float", &arg).into())
  }}},
  rem => {"%", COMMON, Exactly(2), {
    self.take_int(Rax, func, scope)?;
    self.mov_rcx_nonzero(scope, func)?;
      scope.push(Custom(Jsonpiler::CQO.to_vec()));
    scope.push(IDivR(Rcx));
    Ok(Json::Int(Var(scope.mov_tmp(Rdx)?)))
  }}
}
impl Jsonpiler {
  fn mov_rcx_nonzero(&mut self, scope: &mut ScopeInfo, func: &mut FuncInfo) -> ErrOR<()> {
    let (int, pos) = take_arg!(self, func, "Int", Json::Int(x) => x);
    match int {
      Lit(l_int) => {
        if l_int == 0 {
          return err!(self, pos, "ZeroDivisionError");
        }
        #[expect(clippy::cast_sign_loss)]
        scope.push(MovQQ(Rq(Rcx), Iq(l_int as u64)));
      }
      Var(label) => {
        func.sched_free_tmp(&label);
        scope.push(MovQQ(Rq(Rcx), Mq(label.kind)));
        scope.push(CmpRIb(Rcx, 0));
        let zero_division_msg = self.global_str("ZeroDivisionError".to_owned());
        let message_box = self.import(Jsonpiler::USER32, "MessageBoxA", 0x285);
        let mb_a = self.call_api_check_null(message_box);
        let exit_process = self.import(Jsonpiler::KERNEL32, "ExitProcess", 0x167);
        let zero_division_err = match self.sym_table.entry("ZERO_DIVISION_ERR") {
          Occupied(entry) => *entry.get(),
          Vacant(entry) => {
            let id = self.label_id;
            self.label_id += 1;
            self.insts.extend_from_slice(&[
              Lbl(id),
              Clear(Rcx),
              LeaRM(Rdx, Global { id: zero_division_msg }),
              Clear(R8),
              MovRId(R9, 0x10),
            ]);
            self.insts.extend_from_slice(&mb_a);
            self.insts.extend_from_slice(&[MovRId(Rcx, u32::MAX), CallApi(exit_process)]);
            entry.insert(id);
            id
          }
        };
        scope.push(Jcc(E, zero_division_err));
      }
    }
    Ok(())
  }
}
