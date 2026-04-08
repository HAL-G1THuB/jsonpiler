use jsonpiler::Jsonpiler;
use std::process::exit;
fn main() {
  #[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
  #[deprecated(note = "This program is supported on Windows x64 only.")]
  #[allow(dead_code)]
  const _: () = ();
  #[expect(clippy::print_stderr)]
  match Jsonpiler::default().main() {
    Err(err) => {
      eprintln!("{err}");
      exit(1)
    }
    Ok(exit_code) => exit(exit_code),
  }
}
