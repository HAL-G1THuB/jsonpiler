use super::{inst::*, mach_o::*, ops::invert_cond};
use crate::prelude::*;
impl A64Assembler {
  pub(crate) fn encode_a64_inst(
    &mut self,
    inst: A64Inst,
    idx: u32,
    text_off: u32,
    apis: &[(String, Vec<(LabelId, String)>)],
  ) -> ErrOR<Option<u32>> {
    Ok(Some(match inst {
      Asr(rd, rn, imm6) => {
        if imm6 > 63 {
          return Err(InternalOverFlow.into());
        }
        enc_reg2(0x9340_0000, rd, rn) | (u32::from(imm6) << 16) | (63 << 10)
      }
      Lsl(rd, rn, imm6) => {
        if imm6 > 63 {
          return Err(InternalOverFlow.into());
        }
        enc_reg2(0xD340_0000, rd, rn)
          | ((64 - u32::from(imm6)) << 16)
          | ((63 - u32::from(imm6)) << 10)
      }
      AsrR3(rd, rn, rm) => enc_reg3(0x9AC0_2800, rd, rn, rm),
      LslR3(rd, rn, rm) => enc_reg3(0x9AC0_2000, rd, rn, rm),
      AddR3(rd, rn, rm) => enc_reg3(0x8B00_0000, rd, rn, rm),
      AddSR3(rd, rn, rm) => enc_reg3(0xAB00_0000, rd, rn, rm),
      AddRI12(rd, rn, imm) => enc_imm12(0x9100_0000, rd, rn, imm)?,
      AddLbl(rd, lbl) => {
        let imm = self.get_lbl_addr(lbl)?;
        match self.encode_a64_inst(
          AddRI12(rd, rd, (imm % A64_PAGE_SIZE as u64) as u16),
          idx,
          text_off,
          apis,
        )? {
          Some(code) => code,
          None => return Ok(None),
        }
      }
      SubR3(rd, rn, rm) => enc_reg3(0xCB00_0000, rd, rn, rm),
      SubSR3(rd, rn, rm) => enc_reg3(0xEB00_0000, rd, rn, rm),
      SubRI12(rd, rn, imm) => enc_imm12(0xD100_0000, rd, rn, imm)?,
      MulR3(rd, rn, rm) => enc_reg4(0x9B00_0000, rd, rn, rm, Xzr),
      SMulH(rd, rn, rm) => enc_reg3(0x9B40_7C00, rd, rn, rm),
      SDivR3(rd, rn, rm) => enc_reg3(0x9AC0_0C00, rd, rn, rm),
      AndR3(rd, rn, rm) => enc_reg3(0x8A00_0000, rd, rn, rm),
      OrrR3(rd, rn, rm) => enc_reg3(0xAA00_0000, rd, rn, rm),
      OrnR3(rd, rn, rm) => enc_reg3(0xAA20_0000, rd, rn, rm),
      EorR3(rd, rn, rm) => enc_reg3(0xCA00_0000, rd, rn, rm),
      TstRb(rn) => enc_reg2(0xF240_1C00, Xzr, rn),
      MovRR(rd, rm) => match self.encode_a64_inst(OrrR3(rd, Xzr, rm), idx, text_off, apis)? {
        Some(code) => code,
        None => return Ok(None),
      },
      MovN(rd, imm16, shift) => enc_shift_imm16(0x9280_0000, rd, imm16, shift),
      MovZ(rd, imm16, shift) => enc_shift_imm16(0xD280_0000, rd, imm16, shift),
      MovK(rd, imm16, shift) => enc_shift_imm16(0xF280_0000, rd, imm16, shift),
      LdR(reg_size, rt, rn, off) => enc_r(reg_size, true, rt, rn, off)?,
      StR(reg_size, rt, rn, off) => enc_r(reg_size, false, rt, rn, off)?,
      FLdRD(rt, rn, off) => enc_f_r(true, rt, rn, off)?,
      FStRD(rt, rn, off) => enc_f_r(false, rt, rn, off)?,
      FMovDX(rd, rn) => enc_reg2(0x9E67_0000, rd, rn),
      FMovXD(rd, rn) => enc_reg2(0x9E66_0000, rd, rn),
      FArithD(kind, rd, rn, rm) => {
        let op = 0x1E60_0800
          | match kind {
            Mul => 0x0000_0000,
            Div => 0x0000_1000,
            AddSd => 0x0000_2000,
            SubSd => 0x0000_3000,
          };
        enc_reg3(op, rd, rn, rm)
      }
      FNegD(rd, rn) => enc_reg2(0x1E61_4000, rd, rn),
      FSqrtD(rd, rn) => enc_reg2(0x1E61_C000, rd, rn),
      SCvtFD(rd, rn) => enc_reg2(0x9E62_0000, rd, rn),
      FCvtZSD(rd, rn) => enc_reg2(0x9E78_0000, rd, rn),
      FCmpD(rn, rm) => enc_reg3_ignore_rd(0x1E60_2000, rn, rm),
      FAbsD(rd, rn) => enc_reg2(0x1E60_C000, rd, rn),
      CmpRR(rn, rm) => match self.encode_a64_inst(SubSR3(Xzr, rn, rm), idx, text_off, apis)? {
        Some(code) => code,
        None => return Ok(None),
      },
      CmpRI12(rn, imm12) => enc_imm12(0xF100_0000, Xzr, rn, imm12)?,
      CSet(rd, cond) => enc_csinc(0x9A80_0400, rd, Xzr, Xzr, invert_cond(cond)),
      CSel(rd, rn, rm, cond) => enc_csinc(0x9A80_0000, rd, rn, rm, cond),
      Adrp(rd, lbl) => {
        let target_addr = self.get_lbl_addr(lbl)?;
        let Some(pc_addr) = u64::from(self.rva[&TextA]).checked_add(u64::from(idx) * 4) else {
          return Err(InternalOverFlow.into());
        };
        let target_page = target_addr & !(A64_PAGE_SIZE - 1) as u64;
        let pc_page = pc_addr & !(A64_PAGE_SIZE - 1) as u64;
        let signed_diff = (target_page as i64 - pc_page as i64) >> 12;
        if !((-0x100000..0x100000).contains(&signed_diff)) {
          return Err(InternalOverFlow.into());
        }
        let imm21_val = (signed_diff as u32) & ((1 << 21) - 1);
        let imm_lo = (imm21_val & 0x3) << 29;
        let imm_hi = ((imm21_val >> 2) & ((1 << 19) - 1)) << 5;
        0x9000_0000 | imm_lo | imm_hi | (rd as u32)
      }
      Br(rn) => 0xD61F_0000 | ((rn as u32) << 5),
      B_(lbl) => enc_b_idx(idx, self.get_text_idx(lbl)?)?,
      BCc(cond, lbl) => enc_bcc_idx(cond, idx, self.get_text_idx(lbl)?)?,
      Bl(lbl) => enc_bl_idx(idx, self.get_text_idx(lbl)?)?,
      Blr(rn) => 0xD63F_0000 | ((rn as u32) << 5),
      BApi(api) => {
        let mut api_idx = api.1;
        for (i, (_, funcs)) in apis.iter().enumerate() {
          if i == api.0 as usize {
            break;
          }
          api_idx += len_u32(funcs)?;
        }
        enc_bl_idx(idx, (self.rva[&Stubs] - text_off + api_idx * STUB_SIZE) / 4)?
      }
      RetA => 0xD65F_0000 | ((X30 as u32) << 5),
      Ldp(rt, rt2, rn, offset) => enc_p_64(true, rt, rt2, rn, offset)?,
      Stp(rt, rt2, rn, offset) => enc_p_64(false, rt, rt2, rn, offset)?,
      LblA(_) => return Ok(None),
    }))
  }
  pub(crate) fn encode_global_data(
    &mut self,
    id: LabelId,
    data_lbl: &GlobalData,
    c_str: &mut Vec<u8>,
    bss: &mut u64,
    data: &mut Vec<u8>,
  ) -> ErrOR<()> {
    match data_lbl {
      Chars(str) => {
        self.insert_lbl(id, CString, len_u32(c_str)?)?;
        c_str.extend_from_slice(str.as_bytes());
        c_str.push(0);
      }
      WChars(w_chars) => {
        c_str.resize(align_up(c_str.len(), 2)?, 0);
        self.insert_lbl(id, CString, len_u32(c_str)?)?;
        for ch in w_chars.encode_utf16() {
          c_str.extend_from_slice(&ch.to_le_bytes());
        }
        c_str.extend_from_slice(&[0; 2]);
      }
      Zeros(size, align) => {
        *bss = align_up_u64(*bss, *align as u64)?;
        self.insert_lbl(id, BssA, u32::try_from(*bss)?)?;
        *bss += u64::from(*size);
      }
      Byte(byte) => {
        self.insert_lbl(id, DataA, len_u32(data)?)?;
        data.push(*byte);
      }
      Quad(qword) => {
        data.resize(align_up(data.len(), 8)?, 0);
        self.insert_lbl(id, DataA, len_u32(data)?)?;
        data.extend_from_slice(&qword.to_le_bytes());
      }
    }
    Ok(())
  }
}
fn enc_reg3_ignore_rd(op: u32, rn: A64Reg, rm: A64Reg) -> u32 {
  op | ((rm as u32) << 16) | ((rn as u32) << 5)
}
fn enc_reg2(op: u32, rd: A64Reg, rn: A64Reg) -> u32 {
  op | ((rn as u32) << 5) | (rd as u32)
}
fn enc_reg3(op: u32, rd: A64Reg, rn: A64Reg, rm: A64Reg) -> u32 {
  op | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}
fn enc_imm12(op: u32, rd: A64Reg, rn: A64Reg, imm12: u16) -> ErrOR<u32> {
  if imm12 >= 1 << 12 {
    Err(InternalOverFlow.into())
  } else {
    Ok(op | (u32::from(imm12) << 10) | ((rn as u32) << 5) | (rd as u32))
  }
}
fn enc_reg4(op: u32, rd: A64Reg, rn: A64Reg, rm: A64Reg, ra: A64Reg) -> u32 {
  op | ((rm as u32) << 16) | ((ra as u32) << 10) | ((rn as u32) << 5) | (rd as u32)
}
fn enc_shift_imm16(op: u32, rd: A64Reg, imm16: u16, shift: Shift16) -> u32 {
  op | ((shift as u32) << 21) | ((imm16 as u32) << 5) | (rd as u32)
}
fn enc_csinc(op: u32, rd: A64Reg, rn: A64Reg, rm: A64Reg, cond: A64Cc) -> u32 {
  op | ((rm as u32) << 16) | ((cond as u32) << 12) | ((rn as u32) << 5) | (rd as u32)
}
fn enc_b_idx(from: u32, to: u32) -> ErrOR<u32> {
  let imm26 = to as i32 - from as i32;
  if !(-1 << 25..1 << 25).contains(&imm26) {
    Err(InternalOverFlow.into())
  } else {
    Ok(0x1400_0000 | (imm26.cast_unsigned() & ((1 << 26) - 1)))
  }
}
fn enc_bl_idx(from: u32, to: u32) -> ErrOR<u32> {
  let imm26 = to as i32 - from as i32;
  if !(-1 << 25..1 << 25).contains(&imm26) {
    Err(InternalOverFlow.into())
  } else {
    Ok(0x9400_0000 | (imm26.cast_unsigned() & ((1 << 26) - 1)))
  }
}
fn enc_bcc_idx(cond: A64Cc, from: u32, to: u32) -> ErrOR<u32> {
  let imm19 = to as i32 - from as i32;
  if !(-1 << 18..1 << 18).contains(&imm19) {
    Err(InternalOverFlow.into())
  } else {
    Ok(0x5400_0000 | (((imm19 as u32) & ((1 << 19) - 1)) << 5) | (cond as u32))
  }
}
// TODO -offset(stack)
fn enc_r(reg_size: RegSize, load: bool, rt: A64Reg, rn: A64Reg, off: u16) -> ErrOR<u32> {
  if !off.is_multiple_of(reg_size as u16) {
    Err(InternalOverFlow.into())
  } else {
    let imm12 = (off / reg_size as u16) as u32;
    if imm12 >= 1 << 12 {
      Err(InternalOverFlow.into())
    } else {
      Ok(enc_reg2(
        ((match reg_size {
          S8 => 3,
          S4 => 2,
          S1 => 0,
        }) << 30)
          | 0x3900_0000
          | (if load { 1 << 22 } else { 0 })
          | (((imm12) & ((1 << 12) - 1)) << 10),
        rt,
        rn,
      ))
    }
  }
}
fn enc_f_r(load: bool, rt: A64Reg, rn: A64Reg, off: u16) -> ErrOR<u32> {
  if !off.is_multiple_of(8) {
    Err(InternalOverFlow.into())
  } else {
    let imm12 = (off / 8) as u32;
    if imm12 >= 1 << 12 {
      Err(InternalOverFlow.into())
    } else {
      Ok(enc_reg2(
        0xFD00_0000 | (if load { 1 << 22 } else { 0 }) | (((imm12) & ((1 << 12) - 1)) << 10),
        rt,
        rn,
      ))
    }
  }
}
fn enc_p_64(load: bool, rt: A64Reg, rt2: A64Reg, rn: A64Reg, offset: i8) -> ErrOR<u32> {
  if offset % 8 != 0 {
    return Err(InternalOverFlow.into());
  }
  let imm7 = offset / 8;
  if !(-1 << 6..1 << 6).contains(&imm7) {
    Err(InternalOverFlow.into())
  } else {
    Ok(
      0xA900_0000
        | (if load { 1 << 22 } else { 0 })
        | (((imm7 as u32) & ((1 << 7) - 1)) << 15)
        | ((rt2 as u32) << 10)
        | ((rn as u32) << 5)
        | rt as u32,
    )
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  #[expect(clippy::panic_in_result_fn)]
  fn encode_a64_inst() -> ErrOR<()> {
    const MAIN_LBL: LabelId = 0;
    let insts = vec![
      LblA(MAIN_LBL),
      AddR3(X0, X1, X2),
      AddRI12(X0, X1, 123),
      SubR3(X0, X1, X2),
      SubRI12(X0, X1, 123),
      MulR3(X0, X1, X2),
      SDivR3(X0, X1, X2),
      AndR3(X0, X1, X2),
      OrrR3(X0, X1, X2),
      EorR3(X0, X1, X2),
      MovRR(X0, X1),
      MovN(X0, 12345, Shift16::Lsl16),
      MovZ(X0, 12345, Shift16::Lsl16),
      MovK(X0, 12345, Shift16::Lsl16),
      B_(MAIN_LBL),
    ];
    let mut assembler = A64Assembler::new();
    let code = assembler
      .assemble(&[vec![insts]], &[], MAIN_LBL, [1, 2, 3, 4], &[(SYS_B.into(), vec![])], 0x1000)?
      .code;
    let expected: Vec<u32> = vec![
      0x8B020020, 0x9101EC20, 0xCB020020, 0xD101EC20, 0x9B027C20, 0x9AC20C20, 0x8A020020,
      0xAA020020, 0xCA020020, 0xAA0103E0, 0x92A60720, 0xD2A60720, 0xF2A60720, 0x17FFFFF3,
    ];
    let expected_bytes: Vec<u8> = expected.into_iter().flat_map(u32::to_le_bytes).collect();
    assert_eq!(code, expected_bytes);
    Ok(())
  }
}
