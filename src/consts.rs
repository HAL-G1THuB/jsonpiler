pub(crate) mod version {
  #[macro_export]
  #[doc(hidden)]
  macro_rules! version {
    () => {
      "0.8.0"
    };
  }
  pub const VER_MAJOR: u8 = 0;
  pub const VER_MINOR: u8 = 8;
}
pub(crate) mod dll {
  pub const GDI32: &str = "gdi32.dll";
  pub const KERNEL32: &str = "kernel32.dll";
  pub const USER32: &str = "user32.dll";
}
pub(crate) mod builtin_flags {
  pub const COMMON: (bool, bool) = (false, false);
  pub const SCOPE: (bool, bool) = (true, false);
  pub const SPECIAL: (bool, bool) = (false, true);
  pub const SP_SCOPE: (bool, bool) = (true, true);
}
pub(crate) mod custom_insts {
  pub const CQO: &[u8] = &[0x48, 0x99];
  pub const RET: &[u8] = &[0xC3];
  pub const CLD_REPNE_SCASB: &[u8] = &[0xFC, 0xF2, 0xAE];
  pub const CLD_REP_MOVSB: &[u8] = &[0xFC, 0xF3, 0xA4];
  pub const BTR_RAX_63: &[u8] = &[0x48, 0x0F, 0xBA, 0xF0, 0x3F];
  pub const BTC_RAX_63: &[u8] = &[0x48, 0x0F, 0xBA, 0xF8, 0x3F];
}
pub(crate) mod gui_config {
  pub const GUI_H: u32 = 0x200;
  pub const GUI_W: u32 = 0x200;
  pub const GUI_PIXELS_SIZE: u64 = GUI_W as u64 * GUI_H as u64 * 4;
  pub const TITLE: &str = "Jsonpiler GUI";
  pub const TIMER_INTERVAL_MS: u32 = 100;
}
pub(crate) mod assembly_consts {
  use crate::Register::{self, R8, R9, Rcx, Rdx};
  pub const REGS: [Register; 4] = [Rcx, Rdx, R8, R9];
  pub const IMAGE_BASE: u64 = 0x1_4000_0000;
  pub const FILE_ALIGNMENT: u32 = 0x200;
  pub const SECTION_ALIGNMENT: u32 = 0x1000;
  pub const PE_HEADER_OFFSET: u32 = 0x40;
  pub const NUMBER_OF_SECTIONS: u16 = 7;
  pub const OPTIONAL_HEADER_SIZE: u16 = 0xF0;
  pub const PE_HEADERS_TOTAL: u32 =
    24 + OPTIONAL_HEADER_SIZE as u32 + 40 * NUMBER_OF_SECTIONS as u32;
  pub const HEADERS_V_SIZE: u32 = PE_HEADER_OFFSET + PE_HEADERS_TOTAL;
  pub const TEXT_C: u32 = 0x6000_0020;
  pub const DATA_C: u32 = 0xC000_0040;
  pub const BSS_C: u32 = 0xC000_0080;
  pub const N_DATA_C: u32 = 0x4000_0040;
}
pub(crate) mod symbols {
  pub const COPY2HEAP: &str = "COPY2HEAP";
  pub const STD_I: &str = "STD_I";
  pub const STD_O: &str = "STD_O";
  pub const STD_E: &str = "STD_E";
  pub const RANDOM: &str = "RANDOM";
  pub const U8TO16: &str = "U8TO16";
  pub const MSG_BOX: &str = "MSG_BOX";
  pub const FLAG_GUI: &str = "FLAG_GUI";
  pub const HEAP: &str = "HEAP";
  pub const CRITICAL_SECTION: &str = "CRITICAL_SECTION";
  pub const SEH_HANDLER: &str = "SEH_HANDLER";
  pub const WIN_HANDLER: &str = "WIN_HANDLER";
  pub const ERR_HANDLER: &str = "ERR_HANDLER";
  pub const CTRL_C_HANDLER: &str = "CTRL_C_HANDLER";
  // pub const IGNORE_HANDLER: &str = "IGNORE_HANDLER";
  pub const INPUT: &str = "INPUT";
  pub const PRINT: &str = "PRINT";
  pub const PRINT_N: &str = "PRINT_N";
  pub const PRINT_E: &str = "PRINT_E";
  pub const INT2STR: &str = "INT2STR";
}
pub(crate) mod runtime_err {
  use crate::version;
  pub const ZERO_DIVISION: &str = "Division by zero";
  pub const ACCESS_VIOLATION: &str = "AccessViolation";
  pub const STACK_OVERFLOW: &str = "StackOverflow";
  pub const EXCEPTION_OCCURRED: &str = "ExceptionOccurred";
  pub const WARNING: &str = "\n\u{256d}- Warning --------------\n| ";
  pub const IO_ERROR: &str = "\n\u{256d}- IOError --------------\n| ";
  pub const COMPILATION_ERROR: &str = "\n\u{256d}- CompilationError -----\n| ";
  pub const INTERNAL_ERROR: &str = "\n\u{256d}- InternalError --------\n| ";
  pub const ABORTED_ERROR: &str = "\n\u{256d}- AbortedError ---------\n| ";
  pub const RUNTIME_ERROR: &str = "\n\u{256d}- RunTimeError ---------\n| ";
  pub const WIN_API_ERROR: &str = "WinApiError:\n|   ";
  pub const SECONDARY_GUI_ERROR: &str = "SecondaryGUIError";
  pub const ASSERTION_ERROR: &str = "AssertionError:\n|   ";
  pub const ERR_END: &str = "\n\u{2570}------------------------\n";
  pub const ERR_SEPARATE: &str = "\n|------------------------\n| ";
  pub const HIDDEN_ERROR: &str = "
\u{256d}- ???Error -------------
| An unexpected error occurred.
\u{2570}------------------------

Detailed information is hidden in this release build.
Use a debug build to see full error details.
";
  pub const REPORT_MSG: &str = concat!(
    "
Internal Jsonpiler error.
This is a compiler bug.

Report:
https://github.com/HAL-G1THuB/jsonpiler/issues/new

Include:
- source
- version: ",
    version!(),
    "
- error code: `"
  );
  pub const COMMAND: &str = "\
  Commands:

  version
      Print program version

  help
      Print this help message

  <input.jspl | input.json> [args for .exe]
      Build an executable and run

  release <input.jspl | input.json> [args for .exe]
      Build a release version and run

  build <input.jspl | input.json>
      Build an executable file

  build release <input.jspl | input.json>
      Build a release version
";
}
