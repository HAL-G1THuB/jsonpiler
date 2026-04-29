mod assembler;
mod command_line;
mod compiler;
mod dependency;
mod internal;
mod parser;
mod prelude;
mod server;
mod utility;
use prelude::*;
pub struct Jsonpiler {
  analysis: Option<Analysis>,
  builtin: HashMap<&'static str, BuiltInInfo>,
  data: Vec<DataLbl>,
  dlls: Vec<Dll>,
  functions: BTreeMap<LabelId, CompiledFunc>,
  globals: BTreeMap<String, Pos<Variable>>,
  handlers: Handlers,
  id_seed: LabelId,
  parsers: Vec<Pos<Parser>>,
  release: bool,
  startup: Vec<Inst>,
  str_cache: HashMap<String, LabelId>,
  symbols: HashMap<&'static str, LabelId>,
  user_defined: BTreeMap<String, Pos<UserDefinedInfo>>,
}
