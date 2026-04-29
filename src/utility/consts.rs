pub mod version {
  pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");
  pub const VERSION: &str = env!("CARGO_PKG_VERSION");
  pub const VER_MAJOR_MINOR: [u8; 2] = split_version();
  const fn parse_ver_number(bytes: &[u8], i: &mut usize) -> u8 {
    let mut value = 0;
    while *i < bytes.len() {
      let byte = bytes[*i];
      if byte < b'0' || byte > b'9' {
        break;
      }
      value = value * 10 + (byte - b'0');
      *i += 1;
    }
    value
  }
  const fn split_version() -> [u8; 2] {
    let bytes = VERSION.as_bytes();
    let mut i = 0;
    let major = parse_ver_number(bytes, &mut i);
    if i < bytes.len() && bytes[i] == b'.' {
      i += 1;
    }
    let minor = parse_ver_number(bytes, &mut i);
    [major, minor]
  }
}
pub mod dll {
  pub const GDI32: &str = "gdi32.dll";
  pub const KERNEL32: &str = "kernel32.dll";
  pub const USER32: &str = "user32.dll";
}
pub mod builtin_flags {
  macro_rules! def_flag {
    ($(($a:ident, $b:ident, $c:ident, $d:ident) $(,)?)+) => {
      $(
        pub const $a: (bool, bool) = (false, false);
        pub const $b: (bool, bool) = (false, true);
        pub const $c: (bool, bool) = (true, false);
        pub const $d: (bool, bool) = (true, true);
      )+
    };
  }
  def_flag!(
    (COMMON, SPECIAL, _SCOPE, SP_SCOPE),
    (INFO_NONE, INFO_KEY_VAL, INFO_FUNC, INFO_OP),
    (LABEL_NOT_RETURN, _UNREACHABLE, FN_NOT_RETURN, FN_RETURN)
  );
}
pub mod custom_insts {
  pub const CQO: &[u8] = &[0x48, 0x99];
  pub const RET: &[u8] = &[0xC3];
  pub const CLD_REPNE_SCASB: &[u8] = &[0xFC, 0xF2, 0xAE];
  pub const CLD_REP_MOVSB: &[u8] = &[0xFC, 0xF3, 0xA4];
  pub const BTR_RAX_63: &[u8] = &[0x48, 0x0F, 0xBA, 0xF0, 0x3F];
  pub const BTC_RAX_63: &[u8] = &[0x48, 0x0F, 0xBA, 0xF8, 0x3F];
}
pub mod gui_config {
  pub const GUI_H: u32 = 0x200;
  pub const GUI_W: u32 = 0x200;
  pub const GUI_PIXELS_SIZE: u64 = (GUI_W * GUI_H * 4) as u64;
  pub const TITLE: &str = "Jsonpiler GUI";
  pub const TIMER_INTERVAL_MS: u32 = 100;
}
pub mod format_config {
  pub const GB: u32 = 1 << 30;
  pub const LINE_MAX: u32 = 100;
  pub const ASSIGN_OP: &[&str] = &["=", "+=", "-=", "*=", "/="];
  pub const OP_PRECEDENCE: &[&[&str]] = &[
    ASSIGN_OP,
    &["or"],
    &["xor"],
    &["and"],
    &["<", "<=", ">", ">=", "==", "!="],
    &["<<", ">>"],
    &["+", "-"],
    &["*", "/", "%"],
  ];
}
pub mod assembly_consts {
  use crate::Register::{self, R8, R9, Rcx, Rdx};
  pub const ARG_REGS: [Register; 4] = [Rcx, Rdx, R8, R9];
  pub const IMAGE_BASE: u64 = 0x1_4000_0000;
  pub const FILE_ALIGNMENT: u32 = 0x200;
  pub const SECTION_ALIGNMENT: u32 = 0x1000;
  pub const PE_HEADER_OFFSET: u32 = 0x40;
  pub const NUMBER_OF_SECTIONS: u16 = 7;
  pub const OPTIONAL_HEADER_SIZE: u16 = 0xF0;
  pub const HEADERS_SIZE: u32 =
    PE_HEADER_OFFSET + 0x18 + OPTIONAL_HEADER_SIZE as u32 + 0x28 * NUMBER_OF_SECTIONS as u32;
}
pub mod symbols {
  macro_rules! def_sym {
    ($($name:ident,)+) => {
      $( pub const $name: &str = stringify!($name); )+
    }
  }
  def_sym!(
    COPY2HEAP,
    STD_I,
    STD_O,
    STD_E,
    RANDOM,
    U8TO16,
    U16TO8,
    MSG_BOX,
    FLAG_GUI,
    HEAP,
    LEAK_CNT,
    CRITICAL_SECTION,
    INPUT,
    PRINT,
    PRINT_N,
    PRINT_E,
    STR_LEN,
    STR_CHARS_LEN,
    STR_EQ,
    INT2STR,
    UTF8_SLICE,
  );
}
pub mod runtime_err {
  pub const ZERO_DIVISION: &str = "Division by zero";
  pub const TOO_LARGE_SHIFT: &str = "Shift amount exceeds 63 bits";
  pub const ACCESS_VIOLATION: &str = "AccessViolation";
  pub const STACK_OVERFLOW: &str = "StackOverflow";
  pub const EXCEPTION_OCCURRED: &str = "ExceptionOccurred";
  pub const WARNING: &str = "\n\u{256d}- Warning -------------------";
  pub const INTERNAL_ERR: &str = "InternalError";
  pub const SYSTEM_EXIT: &str = "\n\u{256d}- Exit ----------------------";
  pub const RUNTIME_ERR: &str = "\n\u{256d}- RuntimeError --------------";
  pub const WIN_API_ERR: &str = "\n| WinApiError:\n|   ";
  pub const ERR_END: &str = "\n\u{2570}-----------------------------\n";
  pub const ERR_SEPARATE: &str = "\n|-----------------------------\n| ";
  pub const HIDDEN_ERROR: &str = "
\u{256d}- ???Error ------------------
| An unexpected error occurred.
\u{2570}-----------------------------

Detailed information is hidden in this release build.
Use a debug build to see full error details.
";
  pub const ISSUE: &str = concat!(
    "
Internal Jsonpiler error.
This is a compiler bug.

Report:
https://github.com/HAL-G1THuB/jsonpiler/issues/new

Include:
- source
- version: ",
    env!("CARGO_PKG_VERSION"),
    "
- error code: `"
  );
  pub const COMMAND: &str = "
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
    Build an executable

build release <input.jspl | input.json>
release build <input.jspl | input.json>
    Build a release version

format input.jspl
    Format the source code

server
    Start a LSP server for the VS Code extension
";
}
