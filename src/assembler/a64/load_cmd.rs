pub(crate) use super::lc_segment::*;
use crate::prelude::*;
#[derive(Debug, Clone)]
pub(crate) enum LoadCmd {
  BuildVersion,
  CodeSign(u32, u32),
  DySymtab([(u32, u32); 3], (u32, u32)),
  DyldInfoOnly([Option<(u32, u32)>; 5]),
  LoadDyLinker,
  LoadDylib(Vec<u8>),
  Main(u32, u32),
  Segment(A64LcSegment),
  Symtab(u32, u32, u32, u32),
}
impl LoadCmd {
  pub(crate) fn lc_cmd(&self) -> ErrOR<Vec<u8>> {
    match self {
      BuildVersion => Ok(lc_build_version().to_vec()),
      CodeSign(offset, size) => Ok(lc_code_sign(*offset, *size).to_vec()),
      DySymtab(sym_idx_range, indirect) => Ok(lc_dy_symtab(*sym_idx_range, *indirect).to_vec()),
      DyldInfoOnly(dyld_info) => Ok(lc_dyld_info_only(*dyld_info).to_vec()),
      LoadDyLinker => Ok(lc_load_dy_linker().to_vec()),
      LoadDylib(lib_name) => lc_load_dylib(lib_name),
      Main(sizeof_header, entry_off) => Ok(lc_main(*sizeof_header, *entry_off).to_vec()),
      Segment(seg) => seg.lc_segment(),
      Symtab(sym_off, n_syms, str_off, str_size) => {
        Ok(lc_symtab(*sym_off, *n_syms, *str_off, *str_size).to_vec())
      }
    }
  }
  pub(crate) fn sizeof_cmd(&self) -> ErrOR<u32> {
    Ok(match self {
      BuildVersion => 0x18,
      CodeSign(_, _) => 0x10,
      DySymtab(_, _) => 0x50,
      DyldInfoOnly(_) => 0x30,
      LoadDyLinker => 0x20,
      LoadDylib(lib_name) => 0x18 + align_up_u32((lib_name.len() + 1) as u32, 8)?,
      Main(_, _) => 0x18,
      Segment(seg) => seg.sizeof_cmd()?,
      Symtab(_, _, _, _) => 0x18,
    })
  }
}
pub(crate) const fn lc_build_version() -> [u8; 0x18] {
  const LC_BUILD_VERSION: u32 = 0x32;
  const PLATFORM_MACOS: u32 = 1;
  const N_TOOLS: u32 = 0;
  const TOTAL: u32 = 0x18 + N_TOOLS * 8;
  const MIN_OS: u32 = 12 << 16;
  const SDK: u32 = MIN_OS;
  const _TOOL_VERSIONS: [u8; N_TOOLS as usize * 8] = [];
  le_bytes!(LC_BUILD_VERSION, TOTAL, PLATFORM_MACOS, MIN_OS, SDK, N_TOOLS)
}
pub(crate) const fn lc_load_dy_linker() -> [u8; 0x20] {
  const LC_LOAD_DY_LINKER: u32 = 0xE;
  const DYLD_OFF: u32 = 0xC;
  const TOTAL: u32 = 0x20;
  const NAME: [u8; 14] = *b"/usr/lib/dyld\0";
  let mut arr = le_bytes!(LC_LOAD_DY_LINKER, TOTAL, DYLD_OFF, 0u32, 0u128);
  let mut idx = 0;
  while idx < NAME.len() {
    arr[DYLD_OFF as usize + idx] = NAME[idx];
    idx += 1;
  }
  arr
}
pub(crate) fn dylib_version(lib_name: &[u8]) -> (u32, u32) {
  if lib_name == SYS_B.as_bytes() {
    (0x54Cu32 << 16, 1u32 << 16)
  } else if lib_name == OBJC_A.as_bytes() {
    (0xE4u32 << 16, 1u32 << 16)
  } else if lib_name == APP_KIT.as_bytes() {
    (0xA7D3C68u32, 0x2Du32 << 16)
  } else if lib_name == CORE_GRAPHICS.as_bytes() {
    (0x7AD0501u32, 0x40u32 << 16)
  } else {
    (0u32, 0u32)
  }
}
pub(crate) fn lc_load_dylib(lib_name: &[u8]) -> ErrOR<Vec<u8>> {
  const LC_LOAD_DYLIB: u32 = 0xC;
  let (dylib_ver, compat_ver) = dylib_version(lib_name);
  let mut arr = le_bytes!(LC_LOAD_DYLIB, 0u32, 0x18u32, 2u32, 0u32, 0u32).to_vec();
  let name_len = align_up(lib_name.len() + 1, 8)?;
  let total = arr.len() + name_len;
  arr.extend_from_slice(lib_name);
  arr.push(0);
  arr.resize(total, 0);
  arr[4..8].copy_from_slice(&u32::try_from(total)?.to_le_bytes());
  arr[16..20].copy_from_slice(&dylib_ver.to_le_bytes());
  arr[20..24].copy_from_slice(&compat_ver.to_le_bytes());
  Ok(arr)
}
pub(crate) fn lc_main(sizeof_header: u32, entry_off: u32) -> [u8; 0x18] {
  const LC_MAIN: u32 = 0x8000_0028;
  const TOTAL: u32 = 0x18;
  let mut bytes = le_bytes!(LC_MAIN, TOTAL, 0u64, 0u64);
  bytes[8..16].copy_from_slice(&((sizeof_header + entry_off) as u64).to_le_bytes());
  bytes
}
pub(crate) fn lc_dyld_info_only(dyld_info: [Option<(u32, u32)>; 5]) -> [u8; 0x30] {
  const LC_DYLD_INFO_ONLY: u32 = 0x8000_0022;
  const TOTAL: u32 = 0x30;
  let mut bytes = [0u8; 0x30];
  bytes[0..4].copy_from_slice(&LC_DYLD_INFO_ONLY.to_le_bytes());
  bytes[4..8].copy_from_slice(&TOTAL.to_le_bytes());
  for (i, info) in dyld_info.into_iter().enumerate() {
    if let Some((offset, size)) = info {
      bytes[8 + i * 8..0xC + i * 8].copy_from_slice(&offset.to_le_bytes());
      bytes[0xC + i * 8..0x10 + i * 8].copy_from_slice(&size.to_le_bytes());
    }
  }
  bytes
}
pub(crate) fn lc_symtab(sym_off: u32, n_syms: u32, str_off: u32, str_size: u32) -> [u8; 0x18] {
  const LC_SYMTAB: u32 = 0x2;
  const TOTAL: u32 = 0x18;
  let mut bytes = [0u8; 0x18];
  bytes[0..4].copy_from_slice(&LC_SYMTAB.to_le_bytes());
  bytes[4..8].copy_from_slice(&TOTAL.to_le_bytes());
  for (i, val) in [sym_off, n_syms, str_off, str_size].iter().enumerate() {
    bytes[8 + i * 4..0xC + i * 4].copy_from_slice(&val.to_le_bytes());
  }
  bytes
}
pub(crate) fn lc_dy_symtab(
  sym_idx_range: [(u32, u32); 3],
  (indirect_off, n_indirect): (u32, u32),
) -> [u8; 0x50] {
  const LC_DY_SYMTAB: u32 = 0xB;
  const TOTAL: u32 = 0x50;
  let mut bytes = [0u8; 0x50];
  bytes[0..4].copy_from_slice(&LC_DY_SYMTAB.to_le_bytes());
  bytes[4..8].copy_from_slice(&TOTAL.to_le_bytes());
  for (i, (idx, range)) in sym_idx_range.iter().enumerate() {
    bytes[8 + i * 8..0xC + i * 8].copy_from_slice(&idx.to_le_bytes());
    bytes[0xC + i * 8..0x10 + i * 8].copy_from_slice(&range.to_le_bytes());
  }
  bytes[0x38..0x3C].copy_from_slice(&indirect_off.to_le_bytes());
  bytes[0x3C..0x40].copy_from_slice(&n_indirect.to_le_bytes());
  bytes
}
pub(crate) fn lc_code_sign(offset: u32, size: u32) -> [u8; 0x10] {
  const LC_CODE_SIGN: u32 = 0x1D;
  const TOTAL: u32 = 0x10;
  let mut bytes = [0u8; 0x10];
  bytes[0..4].copy_from_slice(&LC_CODE_SIGN.to_le_bytes());
  bytes[4..8].copy_from_slice(&TOTAL.to_le_bytes());
  bytes[8..0xC].copy_from_slice(&offset.to_le_bytes());
  bytes[0xC..0x10].copy_from_slice(&size.to_le_bytes());
  bytes
}
