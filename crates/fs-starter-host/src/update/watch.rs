//! The watcher: looks at the watched files, and when one was replaced and everything has settled, acts (through [`Act`]) and says it.
//!
//! Pattern: Observer (the files) + Strategy (`Act`: what ending a module and restarting the own file mean; `Transaction` and `Calm`
//! from `quiet.rs`) + Active object (the thread that polls). `step` is the whole decision and takes the time as an argument: the
//! thread is a loop around it, the tests call it with a made-up clock.

use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use fs_rpc::Saved;

use super::quiet::{Calm, Settle, Transaction};
use super::stamp::{Change, Role, Watched};

/// Where the lines go.
pub type Say = Arc<dyn Fn(&str) + Send + Sync>;

/// How often the files are looked at.
pub const POLL: Duration = Duration::from_secs(5);

/// Pattern: Strategy – what ending a module and restarting the own file mean (the world the watcher acts on).
///
/// What the watcher does to the world.
pub trait Act: Send {
    /// Ask the module service to save (heartbeat `save`), wait for it, then end it; the next call starts the new one (`ensure_single`).
    /// `Ok(None)`: no service was running. A failed save is an `Err`: the service is NOT ended then.
    ///
    /// # Errors
    /// A word that says why.
    fn end_module(&self, module: &str) -> Result<Option<Saved>, String>;

    /// End the own process and start the new file (with the same words). On success it does not come back.
    ///
    /// # Errors
    /// A word that says why it was not done.
    fn restart_own(&self) -> Result<(), String>;
}

struct Item {
    watched: Watched,
    settle: Settle,
    told_waiting: bool,
}

/// The watched files and the strategies.
pub struct Watcher {
    items: Vec<Item>,
    transaction: Box<dyn Transaction>,
    calm: Box<dyn Calm>,
    act: Box<dyn Act>,
    settle: Duration,
}

impl Watcher {
    /// A watcher over `files`.
    #[must_use]
    pub fn new(
        files: Vec<Watched>,
        transaction: Box<dyn Transaction>,
        calm: Box<dyn Calm>,
        act: Box<dyn Act>,
        settle: Duration,
    ) -> Self {
        let items = files
            .into_iter()
            .map(|watched| Item {
                watched,
                settle: Settle::default(),
                told_waiting: false,
            })
            .collect();
        Self {
            items,
            transaction,
            calm,
            act,
            settle,
        }
    }

    /// How many files are watched.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Nothing watched.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// One look at every file; the lines to say (code first, `key=value` after).
    pub fn step(&mut self, at: Instant) -> Vec<String> {
        let busy = self.transaction.busy();
        let mut lines = Vec::new();
        for item in &mut self.items {
            let Change::Replaced(now) = item.watched.check() else {
                // the same file, or gone (a transaction is half way): nothing is settled
                item.settle.reset();
                continue;
            };
            if busy {
                item.settle.reset();
                continue;
            }
            if !item.settle.settled(now, at, self.settle) {
                continue;
            }
            match item.watched.role.clone() {
                Role::Module(name) => {
                    lines.push(match self.act.end_module(&name) {
                        Ok(Some(saved)) => format!(
                            "update-module-ended module={name} saved={} note=\"the next call starts the new service\"",
                            saved.token()
                        ),
                        Ok(None) => format!("update-module-replaced module={name} note=\"no service was running\""),
                        Err(why) => format!("update-module-kept module={name} why={why} note=\"the old service keeps running\""),
                    });
                    item.watched.accept(now);
                    item.settle.reset();
                }
                Role::Own => {
                    if !self.calm.calm() {
                        if !item.told_waiting {
                            item.told_waiting = true;
                            lines.push("update-restart-waits note=\"a new program file is there; the window is not ended without a calm signal (not there yet), the new file runs at the next start\"".to_owned());
                        }
                        continue;
                    }
                    match self.act.restart_own() {
                        Ok(()) => lines.push("update-restarted note=\"the session is not restored (fs-session is not there)\"".to_owned()),
                        Err(why) => lines.push(format!("update-restart-failed why={why}")),
                    }
                    item.watched.accept(now);
                    item.settle.reset();
                }
            }
        }
        lines
    }
}

/// The running thread; dropping or [`Running::stop`] ends it.
pub struct Running {
    stop: mpsc::Sender<()>,
    done: mpsc::Receiver<()>,
    handle: JoinHandle<()>,
}

impl Running {
    /// Poll every `every` and give the lines to `say`.
    #[must_use]
    pub fn spawn(mut watcher: Watcher, every: Duration, say: Say) -> Self {
        let (stop, wake) = mpsc::channel::<()>();
        let (finished, done) = mpsc::channel::<()>();
        let handle = std::thread::spawn(move || {
            while wake.recv_timeout(every) == Err(mpsc::RecvTimeoutError::Timeout) {
                for line in watcher.step(Instant::now()) {
                    say(&line);
                }
            }
            let _ = finished.send(());
        });
        Self { stop, done, handle }
    }

    #[must_use = "false: the thread hangs and was left behind"]
    /// End the thread and wait for it, at most `wait` (a thread that hangs in a call is left behind, not waited for). `true`: it ended.
    pub fn stop(self, wait: Duration) -> bool {
        let _ = self.stop.send(());
        let ended = self.done.recv_timeout(wait).is_ok() || self.handle.is_finished();
        if ended {
            let _ = self.handle.join();
        }
        ended
    }
}

#[cfg(test)]
#[path = "watch_tests.rs"]
mod tests;
