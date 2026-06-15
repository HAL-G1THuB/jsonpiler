use super::{lc_segment::A64LcSegment, len_u64, link_edit::*, sha256::*};
use crate::prelude::*;
#[derive(Debug, Clone, Copy, Default)]
#[expect(clippy::arbitrary_source_item_ordering, clippy::min_ident_chars)]
pub(crate) struct Protection {
  pub r: bool,
  pub w: bool,
  pub e: bool,
}
impl Protection {
  pub(crate) fn to_le_bytes(self) -> [u8; 4] {
    u32::from((u8::from(self.e) << 2) | (u8::from(self.w) << 1) | u8::from(self.r)).to_le_bytes()
  }
}
pub(crate) struct A64Assembler {
  pub labels: BTreeMap<LabelId, (A64Sect, u32)>,
  pub rva: BTreeMap<A64Sect, u32>,
}
pub(crate) struct A64AsmOutput {
  pub bss: u64,
  pub c_str: Vec<u8>,
  pub code: Vec<u8>,
  pub data: Vec<u8>,
  pub entry_off: u32,
  pub la_sym: Vec<u64>,
  pub link_edit: LinkEdit,
  pub stub_helper: Vec<u8>,
  pub stubs: Vec<u8>,
}
impl A64Assembler {
  pub(crate) fn assemble(
    &mut self,
    insts: &[Vec<Vec<A64Inst>>],
    data_lbl_s: &[(LabelId, GlobalData)],
    main_lbl: LabelId,
    ids: [LabelId; 4],
    apis: &[(String, Vec<(LabelId, String)>)],
    text_off: u32,
  ) -> ErrOR<A64AsmOutput> {
    let apis_len = u32::try_from(apis.iter().flat_map(|(_, funcs)| funcs).count())?;
    let la_sym_lbl = ids[0];
    let got_lbl = ids[1];
    let private_lbl = ids[2];
    let stub_helper_lbl = ids[3];
    self.rva.insert(TextA, text_off);
    let mut c_str: Vec<u8> = vec![];
    let mut data = vec![];
    let mut bss: u64 = 0;
    self.encode_global_data(private_lbl, &Quad(0), &mut c_str, &mut bss, &mut data)?;
    for (id, data_lbl) in data_lbl_s {
      self.encode_global_data(*id, data_lbl, &mut c_str, &mut bss, &mut data)?;
    }
    let mut idx: u32 = 0;
    for inst in insts.iter().flatten().flatten() {
      if let LblA(lbl) = inst {
        self.insert_lbl(*lbl, TextA, idx * 4)?;
      } else {
        idx += 1;
      }
    }
    self.rva.insert(Stubs, text_off + idx * 4);
    let mut sym_str = vec![0x20, 0x00];
    let mut sym_tab = vec![];
    let mut indirect = vec![];
    for (api_idx, (api_lbl, name)) in apis.iter().flat_map(|(_, funcs)| funcs).enumerate() {
      push_sym_tab(api_idx as u32, &mut indirect, &mut sym_tab, &mut sym_str, name.as_bytes())?;
      self.insert_lbl(*api_lbl, TextA, idx * 4)?;
      idx += STUB_SIZE / 4;
    }
    push_sym_tab(apis_len, &mut indirect, &mut sym_tab, &mut sym_str, &STUB_BINDER)?;
    self.rva.insert(StubHelper, text_off + idx * 4);
    self.insert_lbl(stub_helper_lbl, TextA, idx * 4)?;
    idx += 8 + apis_len * 2;
    self.rva.insert(CString, self.rva[&TextA] + idx * 4);
    self.rva.insert(Got, align_up_u32(self.rva[&CString] + len_u32(&c_str)?, A64_SEG_ALIGN)?);
    self.insert_lbl(got_lbl, Got, 0)?;
    self.rva.insert(LaSym, self.rva[&Got] + A64_SEG_ALIGN);
    self.insert_lbl(la_sym_lbl, LaSym, 0)?;
    self.rva.insert(DataA, self.rva[&LaSym] + apis_len * 8);
    self.rva.insert(BssA, self.rva[&DataA] + align_up_u32(len_u32(&data)?, 8)?);
    idx = 0;
    let mut code = vec![];
    for inst in insts.iter().flatten().flatten() {
      if let Some(encoded) = self.encode_a64_inst(*inst, idx, text_off, apis)? {
        code.extend_from_slice(&encoded.to_le_bytes());
      }
      if !matches!(inst, LblA(_)) {
        idx += 1;
      }
    }
    let mut la_sym = vec![];
    let mut stubs = vec![];
    let mut stub_helper_insts = vec![
      vec![LblA(stub_helper_lbl)],
      load_a(X17, Global(private_lbl).ptr())?,
      vec![SubRI12(SP, SP, 0x10), Stp(X16, X17, SP, 0)],
      load_a(X16, Global(got_lbl).v_rq())?,
      vec![Br(X16)],
    ];
    if stub_helper_insts.iter().flatten().count() - 1 != usize::try_from(STUB_HELPER_SIZE / 4)? {
      return Err(InternalOverFlow.into());
    }
    let (rebase, bind, lazy) = encode_info_only(apis, stub_helper_lbl, &mut stub_helper_insts)?;
    for (api_idx, (api_lbl, _)) in apis.iter().flat_map(|(_, funcs)| funcs).enumerate() {
      indirect.push(u32::try_from(api_idx)?);
      for inst in [
        LblA(*api_lbl),
        Adrp(X16, la_sym_lbl),
        AddRI12(X16, X16, u16::try_from(api_idx * 8)?),
        LdR(S8, X16, X16, 0),
        Br(X16),
      ] {
        if let Some(encoded) = self.encode_a64_inst(inst, idx, text_off, apis)? {
          stubs.extend(encoded.to_le_bytes());
        }
        if !matches!(inst, LblA(_)) {
          idx += 1;
        }
      }
      let entry_off = STUB_HELPER_SIZE + api_idx as u64 * 8;
      la_sym.push(A64_TEXT_ADDR + self.rva[&StubHelper] as u64 + entry_off);
    }
    let mut stub_helper = vec![];
    for inst in stub_helper_insts.into_iter().flatten() {
      if let Some(encoded) = self.encode_a64_inst(inst, idx, text_off, apis)? {
        stub_helper.extend(encoded.to_le_bytes());
      }
      if !matches!(inst, LblA(_)) {
        idx += 1;
      }
    }
    let (_, entry_off) = self.get_lbl(main_lbl)?;
    let sym_type_idx = [(0, 0), (0, 0), (0, apis_len + 1)]; //got + apis
    let link_edit = LinkEdit { rebase, bind, lazy, indirect, sym_tab, sym_str, sym_type_idx };
    Ok(A64AsmOutput { code, entry_off, c_str, bss, la_sym, data, stubs, stub_helper, link_edit })
  }
  pub(crate) fn get_lbl(&self, lbl: LabelId) -> ErrOR<(A64Sect, u32)> {
    let Some((sect, offset)) = self.labels.get(&lbl).copied() else {
      return Err(UnknownLabel.into());
    };
    Ok((sect, offset))
  }
  pub(crate) fn get_lbl_addr(&self, lbl: LabelId) -> ErrOR<u64> {
    let (sect, offset) = self.get_lbl(lbl)?;
    let sect_addr = u64::from(self.rva[&sect]);
    sect_addr.checked_add(u64::from(offset)).ok_or(InternalOverFlow.into())
  }
  pub(crate) fn get_text_idx(&self, lbl: LabelId) -> ErrOR<u32> {
    let (sect, offset) = self.get_lbl(lbl)?;
    if sect != TextA {
      return Err(UnknownLabel.into());
    }
    Ok(offset / 4)
  }
  pub(crate) fn new() -> Self {
    Self { labels: BTreeMap::new(), rva: BTreeMap::new() }
  }
}
fn push_sym_tab(
  api_idx: u32,
  indirect: &mut Vec<u32>,
  sym_tab: &mut Vec<u8>,
  sym_str: &mut Vec<u8>,
  name: &[u8],
) -> ErrOR<()> {
  indirect.push(api_idx);
  extend!(sym_tab, len_u32(sym_str)?.to_le_bytes(), [1u8, 0u8], 0x0100u16.to_le_bytes(), [0; 8]);
  sym_str.extend_from_slice(name);
  sym_str.push(0);
  Ok(())
}
pub(crate) fn build_mach_o(
  insts: Vec<Vec<Vec<A64Inst>>>,
  data_lbl_s: Vec<(LabelId, GlobalData)>,
  main_lbl: LabelId,
  ids: [LabelId; 4],
  mut apis: Vec<(String, Vec<(LabelId, String)>)>,
) -> ErrOR<Vec<u8>> {
  if apis.iter().find(|(dll, _)| dll == SYS_B).is_none() {
    apis.push((SYS_B.into(), vec![]));
  }
  let apis_len = apis.iter().map(|(_, funcs)| len_u32(funcs)).sum::<ErrOR<u32>>()?;
  let mut lc_load_dylib_vec = vec![];
  for (dylib, _) in &apis {
    let lc_load_dylib = LoadDylib(dylib.clone().into());
    lc_load_dylib_vec.push(lc_load_dylib);
  }
  let (n_cmds, sizeof_cmds) = sizeof_cmd(&lc_load_dylib_vec)?;
  let mut vec = mach_o_header(n_cmds, sizeof_cmds).to_vec();
  let sizeof_header = align_up_u32(len_u32(&vec)? + sizeof_cmds, A64_PAGE_SIZE)?;
  let mut assembler = A64Assembler::new();
  let A64AsmOutput { bss, c_str, code, entry_off, stubs, data, la_sym, link_edit, stub_helper } =
    assembler.assemble(&insts, &data_lbl_s, main_lbl, ids, &apis, sizeof_header)?;
  let indirect_idx = [0, apis_len, apis_len + 1];
  const R__: Protection = Protection { r: true, w: false, e: false };
  const R_E: Protection = Protection { r: true, w: false, e: true };
  const RW_: Protection = Protection { r: true, w: true, e: false };
  let s_res = [indirect_idx[0], STUB_SIZE, 0];
  let got_res = [indirect_idx[1], 0, 0];
  let la_res = [indirect_idx[2], 0, 0];
  let lc_page_zero = A64LcSegment::page_zero();
  let mut lc_text = lc_page_zero.next(b"TEXT", None, R_E, 0)?;
  lc_text.file_size = u64::from(sizeof_header);
  lc_text.vm_size = u64::from(sizeof_header);
  lc_text.add(b"text", len_u64(&code), 2, 0x8000_0400, [0; 3], false)?;
  lc_text.add(b"stubs", len_u64(&stubs), 2, 0x8000_0408, s_res, false)?;
  lc_text.add(b"stub_helper", len_u64(&stub_helper), 2, 0x8000_0400, [0; 3], false)?;
  lc_text.add(b"cstring", len_u64(&c_str), 0, 2, [0; 3], false)?;
  let mut lc_data_const = lc_text.next(b"DATA_CONST", None, RW_, 0x10)?;
  lc_data_const.add(b"got", 8, 3, 6, got_res, false)?;
  let mut lc_data = lc_data_const.next(b"DATA", None, RW_, 0)?;
  lc_data.add(b"la_symbol_ptr", len_u64(&la_sym) * 8, 3, 7, la_res, false)?;
  lc_data.add(b"data", len_u64(&data), 3, 0, [0; 3], false)?;
  lc_data.add(b"bss", bss, 3, 1, [0; 3], true)?;
  let l_e_off = u32::try_from(
    [&lc_text, &lc_data_const, &lc_data].iter().map(|lc| lc.sizeof_file()).sum::<ErrOR<u64>>()?,
  )?;
  let l_e_size = link_edit.sizeof(l_e_off)?;
  let lc_link_edit = lc_data.next(b"LINKEDIT", Some(l_e_size as u64), R__, 0)?;
  let l_e = link_edit.build(l_e_off, lc_text.file_size)?;
  let mut lc_vec = [lc_page_zero, lc_text, lc_data_const, lc_data, lc_link_edit]
    .into_iter()
    .map(Segment)
    .collect::<Vec<_>>();
  lc_vec.extend(lc_load_dylib_vec);
  lc_vec.extend([
    BuildVersion,
    LoadDyLinker,
    Main(sizeof_header, entry_off),
    l_e.lc_dyld_info_only,
    l_e.lc_symtab,
    l_e.lc_dy_symtab,
    l_e.lc_code_sign,
  ]);
  let sizeof_cmds_actual =
    push_lc(&mut vec, &map_slice(&lc_vec.iter().map(LoadCmd::lc_cmd).collect::<ErrOR<Vec<_>>>()?))?;
  if sizeof_cmds != sizeof_cmds_actual {
    return Err(InternalOverFlow.into());
  }
  fn align_page_size(vec: &mut Vec<u8>) -> ErrOR<()> {
    vec.resize(align_up(vec.len(), A64_SEG_ALIGN as usize)?, 0);
    Ok(())
  }
  vec.resize(sizeof_header as usize, 0);
  extend!(vec, code, stubs, stub_helper, c_str);
  align_page_size(&mut vec)?;
  vec.extend_from_slice(&[0; 8]);
  align_page_size(&mut vec)?;
  for ptr in &la_sym {
    vec.extend_from_slice(&ptr.to_le_bytes());
  }
  vec.extend_from_slice(&data);
  align_page_size(&mut vec)?;
  vec.extend_from_slice(&l_e.bytes);
  let hashes =
    vec[..l_e.code_limit as usize].chunks(A64_PAGE_SIZE as usize).map(sha256).collect::<Vec<_>>();
  for hash in hashes {
    vec.extend_from_slice(&hash);
  }
  align_page_size(&mut vec)?;
  Ok(vec)
}
fn encode_info_only(
  apis: &[(String, Vec<(LabelId, String)>)],
  stub_helper_lbl: LabelId,
  stub_helper_insts: &mut Vec<Vec<A64Inst>>,
) -> ErrOR<(Vec<u8>, Vec<u8>, Vec<u8>)> {
  const DO_BIND_DONE: [u8; 2] = [0x90, 0x00];
  fn set_symbol(name: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0x40];
    bytes.extend_from_slice(name);
    bytes.push(0);
    bytes
  }
  fn set_dylib(dylib_idx: usize) -> ErrOR<Vec<u8>> {
    let ord = dylib_idx.checked_add(1).ok_or(InternalOverFlow)?;
    if let Ok(u8_ord) = u8::try_from(ord)
      && u8_ord < 0x10
    {
      Ok(vec![0x10 | u8_ord])
    } else {
      Ok([vec![0x20], uleb128(u64::try_from(ord)?)].concat())
    }
  }
  fn set_seg(opcode: u8, idx: u8, offset: u64) -> ErrOR<Vec<u8>> {
    let ord = idx.checked_add(1).ok_or(InternalOverFlow)?;
    if ord >= 0x10 {
      return Err(InternalOverFlow.into());
    }
    Ok([vec![opcode | ord], uleb128(offset)].concat())
  }
  fn bind_set_seg(idx: u8, offset: u64) -> ErrOR<Vec<u8>> {
    set_seg(0x70, idx, offset)
  }
  fn rebase_set_seg(idx: u8, offset: u64) -> ErrOR<Vec<u8>> {
    set_seg(0x20, idx, offset)
  }
  fn do_rebase_done(time: u64) -> Vec<u8> {
    [vec![0x60], uleb128(time), vec![0x00]].concat()
  }
  let func_len = apis.iter().map(|(_, funcs)| len_u64(funcs)).sum::<u64>();
  let rebase = cat_vec!(8, [0x11], rebase_set_seg(2, 0)?, do_rebase_done(func_len));
  let mut binder_idx_opt = None;
  let mut lazy = Vec::with_capacity(func_len as usize * 10);
  let mut api_idx: u64 = 0;
  for (dylib_idx, (dylib_name, funcs)) in apis.iter().enumerate() {
    if dylib_name == SYS_B {
      binder_idx_opt = Some(dylib_idx);
    }
    for (_, name) in funcs {
      stub_helper_insts
        .push(vec![MovZ(X16, u16::try_from(lazy.len())?, Lsl0), B_(stub_helper_lbl)]);
      extend!(
        lazy,
        bind_set_seg(2, api_idx * 8)?,
        set_dylib(dylib_idx)?,
        set_symbol(name.as_bytes()),
        DO_BIND_DONE
      );
      api_idx += 1;
    }
  }
  let Some(binder_idx) = binder_idx_opt else {
    return Err(InvalidInst("missing SYS_B dylib".into()).into());
  };
  let bind = cat_vec!(
    8,
    [0x51],
    bind_set_seg(1, 0)?,
    set_dylib(binder_idx)?,
    set_symbol(&STUB_BINDER),
    DO_BIND_DONE
  );
  Ok((rebase, bind, lazy))
}
pub(crate) fn mach_o_header(n_cmds: u32, sizeof_cmds: u32) -> [u8; 32] {
  const MAGIC: u32 = 0xFEED_FACF;
  const CPU_TYPE_ARM64: u32 = 0x100000C;
  const FLAGS: u32 = 0x200085;
  let mut bytes = le_bytes!(MAGIC, CPU_TYPE_ARM64, 0u32, 2u32, 0u32, 0u32, FLAGS, 0u32);
  bytes[0x10..0x14].copy_from_slice(&n_cmds.to_le_bytes());
  bytes[0x14..0x18].copy_from_slice(&sizeof_cmds.to_le_bytes());
  bytes
}
pub(crate) fn push_lc(vec: &mut Vec<u8>, lcs: &[&[u8]]) -> ErrOR<u32> {
  let mut sizeof_cmds = 0;
  for &lc in lcs {
    vec.extend_from_slice(lc);
    sizeof_cmds += len_u32(lc)?;
  }
  Ok(sizeof_cmds)
}
pub(crate) fn uleb128(mut value: u64) -> Vec<u8> {
  let mut out = vec![];
  while value >= 0x80 {
    out.push((value | 0x80) as u8);
    value >>= 7;
  }
  out.push(value as u8);
  out
}
fn sizeof_cmd(lc_load_dylib_vec: &[LoadCmd]) -> ErrOR<(u32, u32)> {
  let mut lc_vec =
    [0, 4, 1, 3, 0].into_iter().map(|n| Segment(A64LcSegment::dummy(n))).collect::<Vec<_>>();
  lc_vec.extend(lc_load_dylib_vec.iter().cloned());
  lc_vec.extend([
    BuildVersion,
    LoadDyLinker,
    Main(0, 0),
    DyldInfoOnly([None; 5]),
    Symtab(0, 0, 0, 0),
    DySymtab([(0, 0); 3], (0, 0)),
    CodeSign(0, 0),
  ]);
  Ok((len_u32(&lc_vec)?, lc_vec.iter().map(|lc| lc.sizeof_cmd()).sum::<ErrOR<u32>>()?))
}
