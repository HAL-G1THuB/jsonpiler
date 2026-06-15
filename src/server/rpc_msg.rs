use super::*;
use crate::prelude::*;
impl Server {
  pub(crate) fn error(&mut self, id: IdKind, code: i64, log: LogMsg) {
    let (method, start) =
      self.requests.remove(&id).unwrap_or_else(|| (String::new(), Instant::now()));
    let stamp = time_stamp();
    let time = format_micros(start);
    let msg = format!("{stamp} [error({id})] <-- {method} {code}: {} in {time}", log.msg);
    self.log(LogMsg::new(log.msg_type, msg, log.verbose));
    let err = ObjectN(vec![("code".into(), IntN(code)), ("message".into(), StrN(log.msg))]);
    self.write(
      &ObjectN(vec![
        ("jsonrpc".into(), StrN("2.0".into())),
        ("id".into(), id.into()),
        ("error".into(), err),
      ])
      .to_string(),
    )
  }
  pub(crate) fn log_only_error(&mut self, start: Instant, log: LogMsg) {
    let stamp = time_stamp();
    let time = format_micros(start);
    let msg = format!("{stamp} [error] <-- error: {} in {time}", log.msg);
    self.log(LogMsg::new(MsgType::Error, msg, log.verbose));
  }
  pub(crate) fn notify(&mut self, method: &str, args: JsonNoPos) {
    if !matches!(method, "window/logMessage") {
      let stamp = time_stamp();
      let msg = format!("{stamp} [notify] <-- {method}");
      self.log(LogMsg::new(MsgType::Info, msg, Some(args.to_string())));
    }
    self.write(
      &ObjectN(vec![
        ("jsonrpc".into(), StrN("2.0".into())),
        ("method".into(), StrN(method.into())),
        ("params".into(), args),
      ])
      .to_string(),
    )
  }
  pub(crate) fn response(&mut self, id: IdKind, result: JsonNoPos) {
    let (method, start) =
      self.requests.remove(&id).unwrap_or_else(|| (String::new(), Instant::now()));
    let stamp = time_stamp();
    let time = format_micros(start);
    let msg = format!("{stamp} [response({id})] <-- {method} in {time}");
    self.log(LogMsg::new(MsgType::Info, msg, Some(result.to_string())));
    self.write(
      &ObjectN(vec![
        ("jsonrpc".into(), StrN("2.0".into())),
        ("id".into(), id.into()),
        ("result".into(), result),
      ])
      .to_string(),
    )
  }
}
