use crate::prelude::*;
use std::path;
pub(crate) struct LogMsg {
  pub msg: String,
  pub msg_type: MsgType,
  pub verbose: Option<String>,
}
impl LogMsg {
  pub fn new(msg_type: MsgType, msg: String, verbose: Option<String>) -> Self {
    Self { msg_type, msg, verbose }
  }
}
#[derive(Debug, Clone, Default, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub(crate) enum IdKind {
  Int(i64),
  #[default]
  Null,
  Str(String),
}
impl From<IdKind> for JsonNoPos {
  fn from(id: IdKind) -> Self {
    match id {
      IdKind::Null => NullN,
      IdKind::Int(int) => IntN(int),
      IdKind::Str(string) => StrN(string),
    }
  }
}
impl TryFrom<JsonNoPos> for IdKind {
  type Error = ();
  fn try_from(json: JsonNoPos) -> Result<Self, Self::Error> {
    match json {
      NullN => Ok(IdKind::Null),
      IntN(int) => Ok(IdKind::Int(int)),
      StrN(string) => Ok(IdKind::Str(string)),
      ArrayN(_) | BoolN(_) | FloatN(_) | ObjectN(_) => Err(()),
    }
  }
}
impl fmt::Display for IdKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let json_no_pos: JsonNoPos = self.clone().into();
    json_no_pos.fmt(f)
  }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
#[expect(clippy::arbitrary_source_item_ordering)]
pub(crate) enum Trace {
  #[default]
  Off = 0,
  Messages = 1,
  Verbose = 2,
}
impl From<&str> for Trace {
  fn from(str: &str) -> Self {
    match str {
      "off" => Trace::Off,
      "messages" => Trace::Messages,
      "verbose" => Trace::Verbose,
      _ => Trace::Off,
    }
  }
}
#[derive(Debug, Clone, Copy)]
#[expect(clippy::arbitrary_source_item_ordering)]
pub(crate) enum MsgType {
  Error = 1,
  Warning = 2,
  Info = 3,
  Log = 4,
}
pub(crate) fn uri2path(uri: &str) -> String {
  let raw = if let Some(path) = uri.strip_prefix("file://localhost/") {
    if cfg!(target_os = "windows") { path.into() } else { format!("/{path}") }
  } else if let Some(path) = uri.strip_prefix("file:///") {
    if cfg!(target_os = "windows") { path.into() } else { format!("/{path}") }
  } else if let Some(path) = uri.strip_prefix("file://") {
    if cfg!(target_os = "windows") { format!(r#"\\{path}"#) } else { format!("//{path}") }
  } else {
    uri.into()
  };
  let mut file = percent_decode(&raw).replace('/', path::MAIN_SEPARATOR_STR);
  if cfg!(target_os = "windows") && file.len() >= 2 && file.as_bytes()[1] == b':' {
    let mut chars = file.chars();
    if let Some(first) = chars.next() {
      let rest: String = chars.as_str().into();
      file = first.to_uppercase().collect();
      file.push_str(&rest);
    }
  }
  file
}
pub(crate) fn percent_encode(input: &str) -> String {
  let mut out = String::with_capacity(input.len() + 16);
  for byte in input.as_bytes() {
    match byte {
      b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' => out.push(*byte as char),
      _ if b"-_.~/:".contains(byte) => out.push(*byte as char),
      _ => out.push_str(&format!("%{byte:02X}")),
    }
  }
  out
}
pub(crate) fn percent_decode(input: &str) -> String {
  let bytes = input.as_bytes();
  let mut out = Vec::with_capacity(bytes.len());
  let mut idx = 0;
  while idx < bytes.len() {
    if bytes[idx] == b'%'
      && idx + 2 < bytes.len()
      && let (Some(p1), Some(p2)) = (ascii2hex(bytes[idx + 1]), ascii2hex(bytes[idx + 2]))
    {
      out.push((p1 << 4) | p2);
      idx += 3;
      continue;
    }
    out.push(bytes[idx]);
    idx += 1;
  }
  String::from_utf8(out).unwrap_or_else(|_| input.into())
}
pub(crate) fn path2uri(path: &str) -> String {
  let mut string = path.replace('\\', "/");
  string = if let Some(path_wo_prefix) = string.strip_prefix("//?/UNC/") {
    format!("//{path_wo_prefix}")
  } else if let Some(path_wo_prefix) = string.strip_prefix("//?/") {
    path_wo_prefix.into()
  } else {
    string
  };
  if string.as_bytes().get(1) == Some(&b':')
    && let Some(first) = string.get_mut(0..1)
  {
    first.make_ascii_lowercase();
  }
  let encoded = percent_encode(&string);
  if encoded.starts_with("//") {
    format!("file:{encoded}")
  } else if encoded.starts_with('/') {
    format!("file://{encoded}")
  } else {
    format!("file:///{encoded}")
  }
}
#[expect(clippy::string_slice)]
pub(crate) fn get_line_str(text: &str, line: usize) -> Option<(usize, &str)> {
  let bytes = text.as_bytes();
  let mut current_line = 0;
  let mut line_start = 0;
  for i in 0..=bytes.len() {
    let is_newline = i < bytes.len() && bytes[i] == b'\n';
    let is_end = i == bytes.len();
    if is_newline || is_end {
      if current_line == line {
        let mut slice = &text[line_start..i];
        if slice.ends_with('\r') {
          slice = &slice[..slice.len() - 1];
        }
        return Some((line_start, slice));
      }
      current_line += 1;
      line_start = i + 1;
    }
  }
  None
}
pub(crate) fn range2offset(text: &str, position: &JsonNoPos) -> Option<usize> {
  let line = position.get_int("line")?.cast_unsigned() as usize;
  let (line_start, line_str) = get_line_str(text, line)?;
  let character = position.get_int("character")?.cast_unsigned() as usize;
  if character == 0 {
    return Some(line_start);
  }
  let mut utf16_count = 0;
  for (idx, ch) in line_str.char_indices() {
    if utf16_count >= character {
      return Some(line_start + idx);
    }
    utf16_count += ch.len_utf16();
    if character < utf16_count {
      return Some(line_start + idx);
    }
  }
  Some(line_start + line_str.len())
}
pub(crate) fn offset2range(text: &str, offset: u32) -> (u32, u32) {
  let mut current_off = 0;
  for (line, raw_line) in text.split_inclusive('\n').enumerate() {
    let line_len = len_u32(raw_line.as_bytes()).unwrap_or(0);
    if offset < current_off + line_len {
      let line_str = raw_line.trim_end_matches(['\n', '\r']);
      let mut utf16_count = 0;
      for (idx, ch) in line_str.char_indices() {
        if current_off + idx as u32 == offset {
          return (line as u32, utf16_count);
        }
        utf16_count += ch.len_utf16() as u32;
      }
      return (line as u32, utf16_count);
    }
    current_off += line_len;
  }
  let last_line = text.lines().count().saturating_sub(1);
  let last_line_str = text.lines().last().unwrap_or("");
  let utf16_count = last_line_str.chars().map(|char| char.len_utf16() as u32).sum();
  (last_line as u32, utf16_count)
}
pub(crate) fn floor_char_boundary(source: &str, mut index: usize) -> usize {
  index = index.min(source.len());
  while index > 0 && index < source.len() && (source.as_bytes()[index] & 0b1100_0000) == 0b1000_0000
  {
    index -= 1;
  }
  index
}
pub(crate) fn format_range(
  (s_line, s_char): (u32, u32),
  (e_line, e_char): (u32, u32),
) -> JsonNoPos {
  ObjectN(vec![
    (
      "start".into(),
      ObjectN(vec![
        ("line".into(), IntN(s_line as i64)),
        ("character".into(), IntN(s_char as i64)),
      ]),
    ),
    (
      "end".into(),
      ObjectN(vec![
        ("line".into(), IntN(e_line as i64)),
        ("character".into(), IntN(e_char as i64)),
      ]),
    ),
  ])
}
// fn find_json(json: &Pos<Json>, offset: u32) -> Option<&Pos<Json>> {
//   if !json.pos.in_range(offset) {
//     return None;
//   }
//   for child in json.val.children() {
//     if let Some(result) = find_json(child, offset) {
//       return Some(result);
//     }
//   }
//   Some(json)
// }
