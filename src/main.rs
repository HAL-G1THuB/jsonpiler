use jsonpiler::Jsonpiler;
use std::{env, process::exit};
fn main() {
  #[expect(clippy::print_stderr)]
  exit(Jsonpiler::new(false).main(env::args()).unwrap_or_else(|err| {
    eprintln!("{err}");
    1
  }))
}
