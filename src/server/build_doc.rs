use crate::prelude::*;
use std::fs::DirEntry;
pub(crate) fn build_doc() -> HashMap<String, String> {
  let mut map = HashMap::new();
  let Some(exe_dir) =
    env::current_exe().ok().and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
  else {
    return map;
  };
  let dir = exe_dir.join("../docs/functions");
  let Ok(entries) = fs::read_dir(&dir) else {
    return map;
  };
  for entry in entries.flatten() {
    build_file_doc(entry, &mut map);
  }
  map
}
fn build_file_doc(entry: DirEntry, map: &mut HashMap<String, String>) -> Option<()> {
  let path = entry.path();
  if path.file_name()?.to_str()? == "README.md" {
    return None;
  }
  if path.extension()?.to_str()? != "md" {
    return None;
  }
  let content = fs::read_to_string(&path).ok()?;
  parse_markdown_into(&content, map);
  Some(())
}
#[expect(clippy::else_if_without_else)]
fn parse_markdown_into(content: &str, map: &mut HashMap<String, String>) {
  let mut current_name: Option<String> = None;
  let mut current_body = String::new();
  for line in content.lines() {
    if let Some(name) = line.strip_prefix("## ") {
      if let Some(prev) = current_name.take() {
        map.insert(prev, unescape_hash(current_body.trim()));
        current_body.clear();
      }
      current_name = Some(name.trim().into());
    } else if current_name.is_some() {
      current_body.push_str(line);
      current_body.push('\n');
    }
  }
  if let Some(last) = current_name {
    map.insert(last, unescape_hash(current_body.trim()));
  }
}
fn unescape_hash(string: &str) -> String {
  let mut out = String::with_capacity(string.len());
  let mut chars = string.chars();
  while let Some(char) = chars.next() {
    if char == '\\' {
      if let Some(next) = chars.next() {
        if "#\\*".contains(next) {
          out.push(next);
        } else {
          out.push('\\');
          out.push(next);
        }
      } else {
        out.push('\\');
      }
    } else {
      out.push(char);
    }
  }
  out
}
