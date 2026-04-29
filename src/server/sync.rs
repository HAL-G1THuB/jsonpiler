use crate::prelude::*;
use std::{
  sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
  thread,
  time::{Duration, Instant},
};
type UriRx = Receiver<String>;
type UriTx = Sender<String>;
pub(crate) struct Channel {
  pub rx: UriRx,
  pub tx: UriTx,
}
impl Channel {
  pub(crate) fn new() -> Self {
    let (tx, rx) = mpsc::channel();
    Channel { rx, tx }
  }
}
enum SchedulerCommand {
  Cancel(String),
  Schedule { delay: Duration, uri: String },
}
pub(crate) struct Scheduler {
  tx: Sender<SchedulerCommand>,
}
impl Scheduler {
  #[expect(clippy::let_underscore_must_use)]
  pub(crate) fn cancel(&self, uri: &str) {
    let _: Result<_, _> = self.tx.send(SchedulerCommand::Cancel(uri.to_owned()));
  }
  pub(crate) fn new(task_tx: UriTx) -> Self {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || Scheduler::run(rx, task_tx));
    Self { tx }
  }
  #[expect(clippy::let_underscore_must_use)]
  pub(crate) fn schedule(&self, uri: String, delay: Duration) {
    let _: Result<_, _> = self.tx.send(SchedulerCommand::Schedule { delay, uri });
  }
}
impl Scheduler {
  fn run(rx: Receiver<SchedulerCommand>, task_tx: UriTx) {
    let mut pending = HashMap::<String, Instant>::new();
    loop {
      let next_deadline = pending.values().copied().min();
      let command = match next_deadline {
        Some(deadline) => match deadline.saturating_duration_since(Instant::now()) {
          wait if wait.is_zero() => Err(RecvTimeoutError::Timeout),
          wait => rx.recv_timeout(wait),
        },
        None => match rx.recv() {
          Ok(command) => Ok(command),
          Err(_) => return,
        },
      };
      match command {
        Ok(SchedulerCommand::Cancel(uri)) => {
          pending.remove(&uri);
        }
        Ok(SchedulerCommand::Schedule { delay, uri }) => {
          pending.insert(uri, Instant::now() + delay);
        }
        Err(RecvTimeoutError::Timeout) => {
          let now = Instant::now();
          let ready = pending
            .iter()
            .filter(|(_, deadline)| **deadline <= now)
            .map(|(uri, _)| uri.clone())
            .collect::<Vec<_>>();
          for uri in ready {
            pending.remove(&uri);
            if task_tx.send(uri).is_err() {
              return;
            }
          }
        }
        Err(RecvTimeoutError::Disconnected) => return,
      }
    }
  }
}
impl Server {
  pub(crate) fn cancel_timer(&mut self, uri: &str) {
    self.scheduler.cancel(uri);
  }
}
