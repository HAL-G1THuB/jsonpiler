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
impl Jsonpiler {
  #[expect(clippy::print_stdout)]
  fn command_line(&mut self) -> Result<Option<(String, bool, env::Args)>, String> {
    let mut args = env::args();
    let program_name = args.next().unwrap_or(PKG_NAME.into());
    let mut file = next_file!(args, program_name);
    let mut build_only = false;
    match file.as_ref() {
      "server" => {
        let mut server = Server::new();
        server.main();
      }
      "help" => help_message(&program_name),
      "version" => println!("{PKG_NAME} version {VERSION}"),
      "format" => {
        file = next_file!(args, program_name);
        let source = fs::read(&file).map_err(|err| self.io_err(err))?;
        let full_path = self.full_path(&file)?;
        self.parsers.push(<Pos<Parser>>::new(source, 0, full_path, 0));
        if let Some(out) = self.parsers[0].format() {
          fs::write(file, out).map_err(|err| self.io_err(err))?;
        }
      }
      _ => {
        match file.as_ref() {
          "build" => {
            build_only = true;
            file = next_file!(args, program_name);
            if file == "release" {
              self.release = true;
              file = next_file!(args, program_name);
            }
          }
          "release" => {
            self.release = true;
            file = next_file!(args, program_name);
            if file == "build" {
              build_only = true;
              file = next_file!(args, program_name);
            }
          }
          _ => (),
        }
        return Ok(Some((file, build_only, args)));
      }
    }
    Ok(None)
  }
  pub(crate) fn full_path(&self, file: &str) -> Result<String, String> {
    env::current_dir()
      .and_then(|dir| dir.join(Path::new(file)).canonicalize())
      .map(|path| path.to_string_lossy().to_string())
      .map_err(|err| self.io_err(err))
  }
  #[inline]
  pub fn main(&mut self) -> Result<i32, String> {
    let Some((file, build_only, args)) = self.command_line()? else {
      return Ok(0);
    };
    if fs::metadata(&file).map_err(|err| self.io_err(err))?.len() > u64::from(GB) {
      return Err(self.format_err(&Compilation(TooLargeFile, vec![])));
    }
    let source = fs::read(&file).map_err(|err| self.io_err(err))?;
    let exe_path = Path::new(&file).with_extension("exe");
    let exe = exe_path.to_string_lossy().to_string();
    let full = self.full_path(&file)?;
    let first_parser = <Pos<Parser>>::new(source, 0, full, self.id());
    self.parsers.push(first_parser);
    let parsed = match Path::new(&file).extension().map(|ext| ext.to_string_lossy()) {
      Some(jspl) if jspl == "jspl" => self.parsers[0].parse_jspl(),
      Some(json) if json == "json" => self.parsers[0].parse_json(),
      _ => return Err(self.format_err(&Compilation(UnsupportedFile, vec![]))),
    }
    .map_err(|err| self.format_err(&err.into()))?;
    self.compile(parsed).map_err(|err| self.format_err(&err))?;
    let (insts, seh) = self.resolve_calls();
    Assembler::new(take(&mut self.dlls), self.parsers[0].val.dep.id, self.handlers)
      .assemble(&insts, take(&mut self.data), &self.parsers[0].val.file, seh)
      .map_err(|err| self.format_err(&err))?;
    if build_only {
      return Ok(0);
    }
    check_platform()?;
    let exe_full = env::current_dir().map_err(|err| self.io_err(err))?.join(exe);
    let status = Command::new(exe_full).args(args).status().map_err(|err| self.io_err(err))?;
    Ok(status.code().unwrap_or(0))
  }
}
#[expect(clippy::print_stdout)]
fn help_message(program_name: &str) {
  println!("Usage: {program_name} <input.jspl | input.json> [args for .exe]{COMMAND}");
}
fn check_platform() -> Result<(), String> {
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
fn platform_err(requirement: &'static str) -> String {
  format!(
    "{}\n| The generated executable requires {requirement}{ERR_END}",
    make_header("PlatformError")
  )
}
