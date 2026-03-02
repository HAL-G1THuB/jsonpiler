use jsonpiler::Jsonpiler;
use std::process::exit;
fn main() {
  #[cfg(all(not(doc), not(all(target_os = "windows", target_arch = "x86_64"))))]
  compile_error!("This program is supported on Windows x64 only.");
  #[expect(clippy::print_stderr)]
  match Jsonpiler::default().run() {
    Err(err) => {
      eprintln!("{err}");
      exit(1)
    }
    Ok(exit_code) => exit(exit_code),
  }
}
