use crate::prelude::*;
use std::{env, process::Command};
macro_rules! next_file {
  ($args:ident, $program_name:ident) => {{
    let Some(next_file) = $args.next() else {
      help_message(&$program_name);
      return Ok(None);
    };
    next_file
  }};
}
struct CmdLineInfo<I>
where
  I: Iterator<Item = String>,
{
  build_only: bool,
  exe_args: I,
  file: String,
  release: bool,
}
impl Jsonpiler {
  #[inline]
  pub fn main<I>(&mut self, args: I) -> Result<i32, String>
  where
    I: IntoIterator<Item = String>,
  {
    self.run(args).map_err(|err| self.format_err(&err))
  }
  fn run<I>(&mut self, args: I) -> ErrOR<i32>
  where
    I: IntoIterator<Item = String>,
  {
    let Some(CmdLineInfo { file, build_only, release, exe_args }) = parse_command_line(args)?
    else {
      return Ok(0);
    };
    self.release = release;
    if fs::metadata(&file)?.len() > u64::from(GB) {
      return Err(Compilation(TooLargeFile, vec![]));
    }
    let source = fs::read_to_string(&file)?;
    let exe_path = Path::new(&file).with_extension("exe");
    let exe = exe_path.to_string_lossy().to_string();
    let full = full_path(&file)?;
    let root_id = self.id();
    self.parsers.push(<Pos<Parser>>::new(source, 0, full.clone(), root_id));
    let parsed = match Path::new(&file).extension().map(|ext| ext.to_string_lossy()) {
      Some(jspl) if jspl == "jspl" => self.first_parser_mut()?.parse_jspl(),
      Some(json) if json == "json" => self.first_parser_mut()?.parse_json(),
      _ => return Err(Compilation(UnsupportedFile, vec![])),
    }
    .map_err(Into::<JsonpilerErr>::into)?;
    self.compile(parsed)?;
    let (insts, seh) = self.resolve_calls()?;
    let assembler = Assembler::new(take(&mut self.dlls), root_id, self.handlers);
    assembler.assemble(&insts, take(&mut self.data), &full, seh)?;
    if build_only {
      return Ok(0);
    }
    check_platform()?;
    let exe_full = env::current_dir()?.join(exe);
    let status = Command::new(exe_full).args(exe_args).status()?;
    Ok(status.code().unwrap_or(0))
  }
}
#[expect(clippy::print_stdout)]
fn help_message(program_name: &str) {
  println!("Usage: {program_name} <input.jspl | input.json> [args for .exe]{COMMAND}");
}
fn check_platform() -> ErrOR<()> {
  if !cfg!(target_os = "windows") {
    return Err(platform_err("Windows x64"));
  }
  if !cfg!(target_arch = "x86_64") {
    return Err(platform_err("x86_64 architecture"));
  }
  if !is_x86_feature_detected!("sse2") {
    return Err(platform_err("a CPU with SSE2 support"));
  }
  Ok(())
}
fn platform_err(requirement: &'static str) -> JsonpilerErr {
  Platform(format!("The generated executable requires {requirement}"))
}
fn full_path(file: &str) -> ErrOR<String> {
  Ok(env::current_dir()?.join(file).canonicalize()?.to_string_lossy().to_string())
}
#[expect(clippy::print_stdout)]
fn parse_command_line<I>(args: I) -> ErrOR<Option<CmdLineInfo<I::IntoIter>>>
where
  I: IntoIterator<Item = String>,
{
  let mut args_iter = args.into_iter();
  let program_name = args_iter.next().unwrap_or(PKG_NAME.into());
  let mut file = next_file!(args_iter, program_name);
  let mut build_only = false;
  let mut release = false;
  match file.as_ref() {
    "server" => {
      let mut server = Server::new();
      server.main();
    }
    "help" => help_message(&program_name),
    "version" => println!("{PKG_NAME} version {VERSION}"),
    "format" => {
      file = next_file!(args_iter, program_name);
      let source = fs::read_to_string(&file)?;
      let full_path = full_path(&file)?;
      let mut parser = <Pos<Parser>>::new(source, 0, full_path, 0);
      if let Some(out) = parser.format() {
        fs::write(file, out)?;
      }
    }
    _ => {
      match file.as_ref() {
        "build" => {
          build_only = true;
          file = next_file!(args_iter, program_name);
          if file == "release" {
            release = true;
            file = next_file!(args_iter, program_name);
          }
        }
        "release" => {
          release = true;
          file = next_file!(args_iter, program_name);
          if file == "build" {
            build_only = true;
            file = next_file!(args_iter, program_name);
          }
        }
        _ => (),
      }
      return Ok(Some(CmdLineInfo { file, build_only, release, exe_args: args_iter }));
    }
  }
  Ok(None)
}
