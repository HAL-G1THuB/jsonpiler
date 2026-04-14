mod assembler;
mod compiler;
mod internal;
mod parser;
mod prelude;
mod server;
mod utility;
use prelude::*;
pub struct Jsonpiler {
  builtin: HashMap<String, BuiltInInfo>,
  data: Vec<DataLbl>,
  dlls: Vec<Dll>,
  functions: BTreeMap<LabelId, CompiledFunc>,
  globals: BTreeMap<String, WithPos<Variable>>,
  handlers: Handlers,
  id_seed: LabelId,
  parsers: Vec<Parser>,
  release: bool,
  startup: Vec<Inst>,
  str_cache: HashMap<String, LabelId>,
  symbols: HashMap<&'static str, LabelId>,
  user_defined: BTreeMap<String, WithPos<UserDefinedInfo>>,
}
