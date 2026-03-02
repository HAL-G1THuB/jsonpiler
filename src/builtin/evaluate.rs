use crate::prelude::*;
built_in! {self, func, _scope, evaluate;
  f_eval => {"eval", COMMON, Exactly(1), {self.eval(func.arg()?, _scope)}},
  list => {"list", COMMON, Any, {Ok(Array(Lit(take(&mut func.args).collect())))}},
  quote => {"'", SPECIAL, Exactly(1), {Ok(func.arg()?.val)}},
  value => {"value", COMMON, Exactly(1), {Ok(func.arg()?.val)}}
}
