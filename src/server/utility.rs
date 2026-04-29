use crate::prelude::*;
pub(crate) fn uri2path(uri: &str) -> String {
  let raw = if let Some(path) = uri.strip_prefix("file:///") {
    path.to_owned()
  } else if let Some(path) = uri.strip_prefix("file://localhost/") {
    path.to_owned()
  } else if let Some(path) = uri.strip_prefix("file://") {
    format!(r#"\\{path}"#)
  } else {
    uri.to_owned()
  };
  let mut file = percent_decode(&raw).replace('/', r"\");
  if file.len() >= 2 && file.as_bytes()[1] == b':' {
    let mut chars = file.chars();
    if let Some(first) = chars.next() {
      let rest = chars.as_str().to_owned();
      file = first.to_uppercase().collect::<String>();
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
  String::from_utf8(out).unwrap_or_else(|_| input.to_owned())
}
pub(crate) fn path2uri(path: &str) -> String {
  let mut string = path.replace('\\', "/");
  string = if let Some(path_wo_prefix) = string.strip_prefix("//?/UNC/") {
    format!("//{path_wo_prefix}")
  } else if let Some(path_wo_prefix) = string.strip_prefix("//?/") {
    path_wo_prefix.to_owned()
  } else {
    string
  };
  if string.as_bytes().get(1) == Some(&b':')
    && let Some(first) = string.get_mut(0..1)
  {
    first.make_ascii_lowercase();
  }
  let encoded = percent_encode(&string);
  if encoded.starts_with("//") { format!("file:{encoded}") } else { format!("file:///{encoded}") }
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
#[expect(clippy::cast_possible_truncation)]
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
#[expect(clippy::cast_possible_truncation)]
pub(crate) fn offset2range(text: &str, offset: usize) -> (u32, usize) {
  let mut current_offset = 0;
  for (line, raw_line) in text.split_inclusive('\n').enumerate() {
    let line_len = raw_line.len();
    if offset < current_offset + line_len {
      let line_str = raw_line.trim_end_matches(['\n', '\r']);
      let mut utf16_count = 0;
      for (idx, ch) in line_str.char_indices() {
        if current_offset + idx == offset {
          return (line as u32, utf16_count);
        }
        utf16_count += ch.len_utf16();
      }
      return (line as u32, utf16_count);
    }
    current_offset += line_len;
  }
  let last_line = text.lines().count().saturating_sub(1);
  let last_line_str = text.lines().last().unwrap_or("");
  let utf16_count = last_line_str.chars().map(|char| char.len_utf16()).sum();
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
  (s_line, s_char): (u32, usize),
  (e_line, e_char): (u32, usize),
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
// fn find_json(json: &Pos<Json>, offset: u32) -> Option<(Json, Position)> {
//   if !json.pos.in_range(offset) {
//     return None;
//   }
//   match &json.val {
//     Object(Lit(object)) => object.iter().find_map(|(key, value)| {
//       if key.pos.in_range(offset) {
//         Some((Str(Lit(key.val.clone())), key.pos))
//       } else {
//         find_json(value, offset)
//       }
//     }),
//     Array(Lit(array)) => array.iter().find_map(|item| find_json(item, offset)),
//     Array(_) | Bool(_) | Float(_) | Int(_) | Null(_) | Object(_) | Str(_) => {
//       Some((json.val.clone(), json.pos))
//     }
//   }
// }
