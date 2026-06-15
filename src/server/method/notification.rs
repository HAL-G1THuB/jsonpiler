use super::super::utility::*;
use crate::prelude::*;
impl Server {
  #[expect(clippy::print_stderr)]
  pub(crate) fn log(&mut self, log: LogMsg) {
    eprintln!("{}", log.msg);
    if let Some(verbose) = log.verbose.as_ref() {
      eprintln!("{verbose}");
    }
    if self.trace == Trace::Off {
      return;
    }
    self.n_log_message(log.msg_type, log.msg);
    if let Some(verbose) = log.verbose
      && self.trace == Trace::Verbose
    {
      self.n_log_message(MsgType::Log, verbose);
    }
  }
  pub(crate) fn n_log_message(&mut self, msg_type: MsgType, msg: String) {
    self.notify(
      "window/logMessage",
      ObjectN(vec![("type".into(), IntN(msg_type as i64)), ("message".into(), StrN(msg))]),
    );
  }
  pub(crate) fn n_publish_diagnostics(&mut self, uri: &str, diags: Vec<JsonNoPos>) {
    self.notify(
      "textDocument/publishDiagnostics",
      ObjectN(vec![("uri".into(), StrN(uri.into())), ("diagnostics".into(), ArrayN(diags))]),
    );
  }
  pub(crate) fn n_show_message(&mut self, msg_type: MsgType, msg: String) {
    self.notify(
      "window/showMessage",
      ObjectN(vec![("type".into(), IntN(msg_type as i64)), ("message".into(), StrN(msg))]),
    );
  }
}
