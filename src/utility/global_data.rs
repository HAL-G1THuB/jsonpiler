use crate::prelude::*;
pub(crate) type DyLib = (String, Vec<String>);
pub(crate) type Api = (u32, u32);
#[derive(Debug, Clone)]
pub(crate) enum GlobalData {
  Byte(u8),
  Chars(String),
  Quad(u64),
  WChars(String),
  Zeros(u32, u32),
}
impl Jsonpiler {
  pub(crate) fn api(&mut self, lib: &str, func: &str) -> Api {
    let idx = self.apis.iter().position(|(dll2, _)| *dll2 == lib).unwrap_or_else(|| {
      self.apis.push((lib.into(), vec![]));
      self.apis.len() - 1
    });
    let idx2 = self.apis[idx].1.iter().position(|func2| *func2 == func).unwrap_or_else(|| {
      self.apis[idx].1.push(func.into());
      self.apis[idx].1.len() - 1
    });
    (idx as u32, idx2 as u32)
  }
}
impl Jsonpiler {
  pub(crate) fn bss(&mut self, size: u32, align: u32) -> u32 {
    self.global_data(Zeros(size, align))
  }
  pub(crate) fn bss_symbol(&mut self, symbol: &'static str, size: u32, align: u32) -> u32 {
    let id = self.bss(size, align);
    self.symbols.insert(symbol, id);
    id
  }
  pub(crate) fn cache_string(&mut self, string: String, wide: bool) -> Memory {
    if let Some(&id) = self.str_cache.get(&string) {
      return Global(id).ptr();
    }
    let id = self.global_data(if wide { WChars } else { Chars }(string.clone()));
    self.str_cache.insert(string, id);
    Global(id).ptr()
  }
  pub(crate) fn global_b(&mut self, boolean: bool) -> Memory {
    let id = if boolean { self.global_data(Byte(bool2byte(boolean))) } else { self.bss(1, 1) };
    Global(id).v_rb()
  }
  pub(crate) fn global_data(&mut self, data: GlobalData) -> u32 {
    let id = self.id();
    self.data.push((id, data));
    id
  }
  pub(crate) fn global_q(&mut self, value: u64) -> Memory {
    let id = if value != 0 { self.global_data(Quad(value)) } else { self.bss(8, 8) };
    Global(id).v_rq()
  }
  pub(crate) fn global_str<T: Into<String>>(&mut self, value: T) -> Memory {
    self.cache_string(value.into(), false)
  }
  pub(crate) fn global_w_chars<T: Into<String>>(&mut self, value: T) -> Memory {
    self.cache_string(value.into(), true)
  }
  // Overflow is unlikely
  pub(crate) fn id(&mut self) -> u32 {
    self.id_seed += 1;
    self.id_seed
  }
}
