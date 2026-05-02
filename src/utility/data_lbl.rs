use crate::prelude::*;
pub(crate) type Api = (u32, u32);
#[derive(Clone, Debug)]
pub(crate) enum DataLbl {
  BssLbl(LabelId, u32, u32),
  Byte(LabelId, u8),
  Quad(LabelId, u64),
  StrLbl(LabelId, String),
  WStrLbl(LabelId, String),
}
impl Jsonpiler {
  #[expect(clippy::cast_possible_truncation)]
  pub(crate) fn api(&mut self, dll: &str, func: &str) -> Api {
    let idx = self.dlls.iter().position(|(dll2, _)| *dll2 == dll).unwrap_or_else(|| {
      self.dlls.push((dll.to_owned(), vec![]));
      self.dlls.len() - 1
    });
    let idx2 = self.dlls[idx].1.iter().position(|func2| *func2 == func).unwrap_or_else(|| {
      self.dlls[idx].1.push(func.to_owned());
      self.dlls[idx].1.len() - 1
    });
    (idx as u32, idx2 as u32)
  }
  pub(crate) fn bss(&mut self, size: u32, align: u32) -> u32 {
    let id = self.id();
    self.data.push(BssLbl(id, size, align));
    id
  }
  pub(crate) fn bss_symbol(&mut self, symbol: &'static str, size: u32) -> u32 {
    let id = self.bss(size, size);
    self.symbols.insert(symbol, id);
    id
  }
  pub(crate) fn cache_string(&mut self, string: String, wide: bool) -> u32 {
    if let Some(&id) = self.str_cache.get(&string) {
      return id;
    }
    let id = self.id();
    self.str_cache.insert(string.clone(), id);
    self.data.push(if wide { WStrLbl(id, string) } else { StrLbl(id, string) });
    id
  }
  pub(crate) fn global_b(&mut self, boolean: bool) -> Memory {
    let id = self.id();
    self.data.push(if boolean { Byte(id, bool2byte(boolean)) } else { BssLbl(id, 1, 1) });
    Memory(Global(id), MemoryType { heap: Value, size: Small(RB) })
  }
  pub(crate) fn global_q(&mut self, value: u64) -> Memory {
    let id = self.id();
    self.data.push(if value != 0 { Quad(id, value) } else { BssLbl(id, 8, 8) });
    Memory(Global(id), MemoryType { heap: Value, size: Small(RQ) })
  }
  pub(crate) fn global_str<T: Into<String>>(&mut self, value: T) -> u32 {
    self.cache_string(value.into(), false)
  }
  pub(crate) fn global_w_chars<T: Into<String>>(&mut self, value: T) -> u32 {
    self.cache_string(value.into(), true)
  }
  // Overflow is unlikely
  pub(crate) fn id(&mut self) -> u32 {
    self.id_seed += 1;
    self.id_seed
  }
}
