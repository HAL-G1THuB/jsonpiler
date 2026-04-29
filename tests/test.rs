#[cfg(test)]
#[expect(clippy::expect_used, clippy::panic)]
mod tests {
  use std::{
    env, fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
  };
  struct TempDirGuard(PathBuf);
  impl Drop for TempDirGuard {
    #[expect(clippy::let_underscore_must_use)]
    fn drop(&mut self) {
      let _: io::Result<()> = fs::remove_dir_all(&self.0);
    }
  }
  #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
  fn copied_examples_dir() -> (PathBuf, TempDirGuard) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/jspl");
    let stamp = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("current time must be after UNIX_EPOCH")
      .as_nanos();
    let dst = env::temp_dir().join(format!("jsonpiler_examples_{stamp}"));
    let dir = TempDirGuard(dst.clone());
    fs::create_dir_all(&dst).expect("failed to create temp examples dir");
    let entries = fs::read_dir(&src).expect("failed to read examples/jspl");
    for entry in entries {
      let path = entry.expect("failed to read entry in examples/jspl").path();
      if path.extension().and_then(|ext| ext.to_str()) != Some("jspl") {
        continue;
      }

      let name = path.file_name().expect("example file must have a file name");
      fs::copy(&path, dst.join(name)).expect("failed to copy example file");
    }
    (dst, dir)
  }
  #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
  fn run_example(examples_dir: &Path, file: &str) -> i32 {
    use jsonpiler::Jsonpiler;
    let example_path = examples_dir.join(file);
    let args = vec!["jsonpiler.exe".into(), example_path.to_string_lossy().to_string()];
    let mut jsonpiler = Jsonpiler::new(false);
    match jsonpiler.main(args) {
      Ok(code) => code,
      Err(err) => panic!("Failed to run example: {}\n{err}", file),
    }
  }
  #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
  #[test]
  fn run_jspl_examples_and_check_exit_codes() {
    let (examples_dir, _dir) = copied_examples_dir();
    let cases = [
      ("arithmetic.jspl", 9),
      ("counter.jspl", 0),
      ("global_and_local.jspl", 1000),
      ("hello.jspl", 0),
      ("import_and_assert.jspl", 11),
      ("is_prime.jspl", 0),
      ("lcm.jspl", 36),
      ("or_nand_xor.jspl", 0),
    ];

    for (file, expected) in cases {
      let code = run_example(&examples_dir, file);
      assert_eq!(code, expected, "unexpected exit code for {file}");
    }
  }

  #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
  #[test]
  fn run_random_example_and_check_exit_code_range() {
    let (examples_dir, _dir) = copied_examples_dir();
    let code = run_example(&examples_dir, "random.jspl");
    assert!(
      (0..100).contains(&code),
      "random.jspl exit code is out of expected range [0, 99]: {code}"
    );
  }

  #[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
  #[test]
  fn skip_on_unsupported_platform() {
    eprintln!("These tests require Windows x64 and are skipped on this target.");
  }
}
