use crate::prelude::*;
built_in! {self, func, _scope, evaluate;
  list => {"list", COMMON, AtLeast(0), { Ok(Array(Lit(take(&mut func.args).collect()))) }},
  name_is_main => {"main", SPECIAL, Exact(1), {
    if self.parsers[0].file == self.parsers[func.pos.file as usize].file {
      Ok(self.eval(func.arg()?, _scope)?.val)
    } else {
      Ok(Null(Lit(())))
    }
  }},
  value => {"value", COMMON, Exact(1), { Ok(func.arg()?.val) }},
}
