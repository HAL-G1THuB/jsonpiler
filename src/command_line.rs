use crate::assembler::a64::mach_o::build_mach_o;
use crate::prelude::*;
#[cfg(target_os = "macos")]
use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;
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
  exe_args: I,
  file_name: String,
  flags: JsonpilerFlags,
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
    let Some(CmdLineInfo { file_name, flags, exe_args }) = parse_command_line(args)? else {
      return Ok(0);
    };
    let a64 = self.flags.a64 || flags.a64;
    self.flags = flags;
    self.flags.a64 = a64;
    if fs::metadata(&file_name)?.len() > u64::from(GB) {
      return Err(JsonpilerErr { kind: Compilation(TooLargeFile), refs: vec![] });
    }
    let source = fs::read_to_string(&file_name)?;
    let exe_path = Path::new(&file_name).with_extension(if self.flags.a64 { "" } else { "exe" });
    let exe = exe_path.to_string_lossy().to_string();
    let full = full_path(&file_name)?;
    let file = self.push_file(source, full.clone())?;
    let parsed = match Path::new(&file_name).extension().map(|ext| ext.to_string_lossy()) {
      Some(jspl) if jspl == "jspl" => file.parse_jspl(),
      Some(json) if json == "json" => file.parse_json(),
      _ => return Err(JsonpilerErr { kind: Compilation(UnsupportedFile), refs: vec![] }),
    }
    .map_err(Into::<JsonpilerErr>::into)?;
    self.compile(parsed)?;
    let root_id = self.first_file()?.dep.id;
    if self.flags.a64 {
      let apis = take(&mut self.apis)
        .into_iter()
        .map(|(dll, funcs)| (dll, funcs.into_iter().map(|api| (self.id(), api)).collect()))
        .collect();
      let insts = self.build_functions_a()?;
      let ids = [self.id(), self.id(), self.id(), self.id()];
      fs::write(&exe, build_mach_o(insts, take(&mut self.data), root_id, ids, apis)?)?;
      #[cfg(target_os = "macos")]
      {
        let mut perms = fs::metadata(&exe)?.permissions();
        perms.set_mode(perms.mode() | 0o111);
        fs::set_permissions(&exe, perms)?;
      }
    } else {
      let (insts, seh) = self.build_functions_x()?;
      let assembler = X64Assembler::new(take(&mut self.apis), root_id, self.handlers);
      assembler.assemble(&insts, take(&mut self.data), &full, seh)?;
    }
    if self.flags.build_only {
      return Ok(0);
    }
    // check_platform()?;
    let exe_full = env::current_dir()?.join(exe);
    let status = Command::new(exe_full).args(exe_args).status()?;
    Ok(status.code().unwrap_or(0))
  }
}
#[expect(clippy::print_stdout)]
fn help_message(program_name: &str) {
  println!("Usage: {program_name} <input.jspl | input.json> [args for .exe]{COMMAND}");
}
// fn check_platform() -> ErrOR<()> {
//   if !cfg!(target_os = "windows") {
//     return Err(platform_err("Windows x64"));
//   }
//   if !cfg!(target_arch = "x86_64") {
//     return Err(platform_err("x86_64 architecture"));
//   }
//   if !is_x86_feature_detected!("sse2") {
//     return Err(platform_err("a CPU with SSE2 support"));
//   }
//   Ok(())
// }
fn platform_err(requirement: &'static str) -> JsonpilerErr {
  JsonpilerErr { kind: Platform(format!("The executable requires {requirement}")), refs: vec![] }
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
  let mut file_name = next_file!(args_iter, program_name);
  let mut build_only = false;
  let mut debug = true;
  match file_name.as_ref() {
    "server" => {
      let mut server = Server::new();
      server.main();
    }
    "help" => help_message(&program_name),
    "version" => println!("{PKG_NAME} version {VERSION}"),
    "format" => {
      file_name = next_file!(args_iter, program_name);
      let source = fs::read_to_string(&file_name)?;
      let mut parser = <Pos<Parser>>::new(source, 0);
      if let Some(out) = parser.format(&file_name) {
        fs::write(file_name, out)?;
      }
    }
    _ => {
      match file_name.as_ref() {
        "build" => {
          build_only = true;
          file_name = next_file!(args_iter, program_name);
          if file_name == "release" {
            debug = false;
            file_name = next_file!(args_iter, program_name);
          }
        }
        "release" => {
          debug = false;
          file_name = next_file!(args_iter, program_name);
          if file_name == "build" {
            build_only = true;
            file_name = next_file!(args_iter, program_name);
          }
        }
        _ => (),
      }
      let a64 = cfg!(target_arch = "aarch64") && cfg!(target_os = "macos");
      let x64 = cfg!(target_arch = "x86_64") && cfg!(target_os = "windows");
      if !a64 && !x64 {
        return Err(platform_err("Windows x64 or macOS a64"));
      }
      let flags = JsonpilerFlags { debug, build_only, a64 };
      return Ok(Some(CmdLineInfo { file_name, flags, exe_args: args_iter }));
    }
  }
  Ok(None)
}
