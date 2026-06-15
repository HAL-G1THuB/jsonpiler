use super::load_cmd::*;
use crate::prelude::*;
const SUPER_BLOB_SIZE: u32 = 0x14;
const IDENT_OFF: u32 = 0x58;
const HASH_OFF: u32 = IDENT_OFF + 2;
#[expect(clippy::arbitrary_source_item_ordering)]
pub(crate) struct LinkEdit {
  pub rebase: Vec<u8>,
  pub bind: Vec<u8>,
  pub lazy: Vec<u8>,
  pub indirect: Vec<u32>,
  pub sym_tab: Vec<u8>,
  pub sym_str: Vec<u8>,
  pub sym_type_idx: [(u32, u32); 3],
}
pub(crate) struct LinkEditOutput {
  pub bytes: Vec<u8>,
  pub code_limit: u32,
  pub lc_code_sign: LoadCmd,
  pub lc_dy_symtab: LoadCmd,
  pub lc_dyld_info_only: LoadCmd,
  pub lc_symtab: LoadCmd,
}
impl LinkEdit {
  pub(crate) fn build(&self, l_e_off: u32, text_file_size: u64) -> ErrOR<LinkEditOutput> {
    fn add_link_edit(vec: &mut Vec<u8>, data: &[u8], align: usize) -> ErrOR<u32> {
      let len = len_u32(vec)?;
      vec.extend_from_slice(data);
      vec.resize(align_up(len as usize + data.len(), align)?, 0);
      Ok(len)
    }
    let mut bytes = vec![];
    let rebase_off = add_link_edit(&mut bytes, &self.rebase, 8)?;
    let bind_off = add_link_edit(&mut bytes, &self.bind, 8)?;
    let lazy_off = add_link_edit(&mut bytes, &self.lazy, 8)?;
    let indirect: Vec<u8> = self.indirect.iter().flat_map(|idx| idx.to_le_bytes()).collect();
    let indirect_off = add_link_edit(&mut bytes, &indirect, 8)?;
    let n_indirect = len_u32(&self.indirect)?;
    let sym_tab_off = add_link_edit(&mut bytes, &self.sym_tab, 8)?;
    let symtab_len = len_u32(&self.sym_tab)?;
    let n_syms = symtab_len / 0x10;
    let str_off = add_link_edit(&mut bytes, &self.sym_str, 16)?;
    let code_sign_off = len_u32(&bytes)?;
    let code_limit = l_e_off + code_sign_off;
    let code_slot = code_limit.div_ceil(A64_PAGE_SIZE);
    let code_sign_size = SUPER_BLOB_SIZE + HASH_OFF + code_slot * 0x20;
    let code_sign = cat_vec!(
      code_sign_size as usize,
      0xFADE0CC0u32.to_be_bytes(),
      code_sign_size.to_be_bytes(),
      1u32.to_be_bytes(),
      [0; 4],
      SUPER_BLOB_SIZE.to_be_bytes(),
      0xFADE0C02u32.to_be_bytes(),
      (code_sign_size - SUPER_BLOB_SIZE).to_be_bytes(),
      0x20400u32.to_be_bytes(),
      2u32.to_be_bytes(),
      HASH_OFF.to_be_bytes(),
      IDENT_OFF.to_be_bytes(),
      [0; 4],
      code_slot.to_be_bytes(),
      code_limit.to_be_bytes(),
      [0x20, 2, 0, A64_PAGE_SHIFT],
      [0; 0x18],
      0u64.to_be_bytes(),
      text_file_size.to_be_bytes(),
      1u64.to_be_bytes(),
      [0x20, 0],
    );
    add_link_edit(&mut bytes, &code_sign, 1)?;
    let dyld_info = [
      Some((l_e_off + rebase_off, len_u32(&self.rebase)?)),
      Some((l_e_off + bind_off, len_u32(&self.bind)?)),
      None,
      Some((l_e_off + lazy_off, len_u32(&self.lazy)?)),
      None,
    ];
    Ok(LinkEditOutput {
      bytes,
      code_limit,
      lc_code_sign: CodeSign(l_e_off + code_sign_off, code_sign_size),
      lc_dyld_info_only: DyldInfoOnly(dyld_info),
      lc_symtab: Symtab(l_e_off + sym_tab_off, n_syms, l_e_off + str_off, len_u32(&self.sym_str)?),
      lc_dy_symtab: DySymtab(self.sym_type_idx, (l_e_off + indirect_off, n_indirect)),
    })
  }
  pub(crate) fn sizeof(&self, l_e_off: u32) -> ErrOR<u32> {
    let mut code_limit = l_e_off;
    code_limit += align_up_u32(len_u32(&self.rebase)?, 8)?;
    code_limit += align_up_u32(len_u32(&self.bind)?, 8)?;
    code_limit += align_up_u32(len_u32(&self.lazy)?, 8)?;
    code_limit += align_up_u32(len_u32(&self.indirect)? * 4, 8)?;
    code_limit += len_u32(&self.sym_tab)?;
    code_limit = align_up_u32(code_limit + len_u32(&self.sym_str)?, 16)?;
    let code_slot = code_limit.div_ceil(A64_PAGE_SIZE);
    let code_sign_size = SUPER_BLOB_SIZE + HASH_OFF + code_slot * 0x20;
    Ok(code_limit + code_sign_size - l_e_off)
  }
}
