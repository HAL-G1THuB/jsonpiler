mod assembler;
mod command_line;
mod compiler;
mod dependency;
mod evaluator;
mod internal;
mod parser;
mod prelude;
mod server;
mod utility;
use prelude::*;
pub struct Jsonpiler {
  analysis: Option<Analysis>,
  apis: Vec<DyLib>,
  builtin: BTreeMap<String, BuiltInInfo>,
  data: Vec<(LabelId, GlobalData)>,
  files: Vec<File>,
  flags: JsonpilerFlags,
  functions: BTreeMap<LabelId, CompiledFunc>,
  globals: BTreeMap<String, Pos<Variable>>,
  handlers: Handlers,
  id_seed: LabelId,
  startup: AorX,
  str_cache: HashMap<String, LabelId>,
  symbols: HashMap<&'static str, LabelId>,
  user_defined: BTreeMap<String, Pos<UserDefinedInfo>>,
}
