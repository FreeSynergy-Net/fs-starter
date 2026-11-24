//! The held connection of a window at level 2: writes go as `invokeIf` (with a step, so a repeat acts once), a step back or forward is
//! `revert`, both over the held connection (`Hold::call`), and the changes of the window are kept as its own history (the units it
//! made, newest last). A conflict (someone else changed the record) is a refusal with its code and a line on stderr naming who
//! changed it, never an overwrite. `expect` is empty: the page has no revision to compare (compare-and-set of the module still holds
//! for `revert`).
//!
//! Pattern: Command (jobs for the held connection) + Memento (the history holds references, the module the data) + Facade.

use std::io;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use fs_forms_run::{ChangeCounter, Coded, UndoState};
use fs_interfaces::formsource_capnp::form_source;
use fs_rpc::{Hold, Job};

use super::invoke::Written;
use super::refusal::{kind_code, with_reason};
use super::reply::{Reply, read_reply};
use super::service::{UNREACHABLE, cause};
use super::watch::Identity;

/// The two stacks of the window: what it made (undoable) and what it took back (redoable).
#[derive(Debug, Default)]
struct History {
    done: Vec<u128>,
    undone: Vec<u128>,
}

/// The held module of one window, at level 2.
pub(super) struct Session {
    hold: Hold,
    module: String,
    me: Identity,
    limit: Duration,
    seq: AtomicU64,
    history: Mutex<History>,
    counter: ChangeCounter,
}

impl Session {
    pub(super) fn new(
        hold: Hold,
        module: &str,
        me: Identity,
        limit: Duration,
        counter: ChangeCounter,
    ) -> Self {
        Self {
            hold,
            module: module.to_owned(),
            me,
            limit,
            seq: AtomicU64::new(0),
            history: Mutex::new(History::default()),
            counter,
        }
    }

    pub(super) fn counter(&self) -> u64 {
        self.counter.get()
    }

    /// How many steps back and forward the window has.
    pub(super) fn state(&self) -> UndoState {
        let h = self
            .history
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        UndoState {
            undoable: h.done.len(),
            redoable: h.undone.len(),
        }
    }

    /// The step key of this call: the run is the instance, the step counts up, so a repeat of ONE call acts once.
    fn step(&self) -> (String, String) {
        let n = self.seq.fetch_add(1, Ordering::SeqCst) + 1;
        (self.me.instance.clone(), n.to_string())
    }

    fn call(&self, job: Job<Reply>) -> Result<Reply, Coded> {
        self.hold.call(job, self.limit).map_err(|e: io::Error| {
            cause(UNREACHABLE, &self.module, "hold").with("io", e.kind().to_string())
        })
    }

    fn refusal(&self, r: &Reply, conflict_code: &str) -> Coded {
        let kind = kind_code(&r.kind);
        let clash = kind.as_deref() == Some("form-source-conflict");
        let code = if clash {
            conflict_code.to_owned()
        } else {
            kind.unwrap_or_else(|| "host-notice-invoke-failed".to_owned())
        };
        let mut c = with_reason(
            cause(&code, &self.module, "service"),
            &r.title,
            &r.detail,
            r.status,
        );
        if let Some(by) = r.by.as_ref().filter(|_| clash) {
            c = c
                .with("by-program", &by.program)
                .with("by-person", &by.person);
        }
        c
    }

    /// `invokeIf`: the command `key` on `record`, `expect` empty. What it made goes to the history.
    ///
    /// # Errors
    /// The refusal as a code; `form-source-conflict` (with `by-program`, `by-person`) when someone else changed the record.
    pub(super) fn invoke_if(
        &self,
        key: &str,
        record: &str,
        args: &[(String, String)],
    ) -> Result<Written, Coded> {
        let (run, step) = self.step();
        let (key, record, args) = (key.to_owned(), record.to_owned(), args.to_vec());
        let job: Job<Reply> = Box::new(move |client| {
            Box::pin(async move {
                let source: form_source::Client =
                    capnp::capability::FromClientHook::new(client.hook);
                let mut request = source.invoke_if_request();
                {
                    let mut p = request.get();
                    p.set_command(key.as_str());
                    p.set_record(record.as_str());
                    p.set_expect("");
                    let mut s = p.reborrow().init_step();
                    s.set_run(run.as_str());
                    s.set_step(step.as_str());
                    let mut list = p.init_args(u32::try_from(args.len()).unwrap_or(u32::MAX));
                    for (i, (path, value)) in (0..).zip(&args) {
                        let mut v = list.reborrow().get(i);
                        v.set_path(path.as_str());
                        v.set_text(value.as_str());
                    }
                }
                let answer = request.send().promise.await.map_err(io::Error::other)?;
                let a = answer.get().map_err(io::Error::other)?;
                read_reply(
                    a.get_failed(),
                    a.get_problem(),
                    a.get_result(),
                    a.get_conflict(),
                )
                .map_err(io::Error::other)
            })
        });
        let reply = self.call(job)?;
        if reply.failed {
            return Err(self.refusal(&reply, "form-source-conflict"));
        }
        if let Some(c) = &reply.change {
            let mut h = self
                .history
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            h.done.push(c.activity);
            h.undone.clear();
        }
        Ok(Written {
            record: reply.record,
            text: reply.text,
        })
    }

    fn revert(&self, id: u128) -> Result<Reply, Coded> {
        let (run, step) = self.step();
        let job: Job<Reply> = Box::new(move |client| {
            Box::pin(async move {
                let source: form_source::Client =
                    capnp::capability::FromClientHook::new(client.hook);
                let mut request = source.revert_request();
                {
                    let mut p = request.get();
                    let mut u = p.reborrow().init_change();
                    u.set_hi(u64::try_from(id >> 64).unwrap_or(0));
                    u.set_lo(u64::try_from(id & u128::from(u64::MAX)).unwrap_or(0));
                    let mut s = p.init_step();
                    s.set_run(run.as_str());
                    s.set_step(step.as_str());
                }
                let answer = request.send().promise.await.map_err(io::Error::other)?;
                let a = answer.get().map_err(io::Error::other)?;
                read_reply(
                    a.get_failed(),
                    a.get_problem(),
                    a.get_result(),
                    a.get_conflict(),
                )
                .map_err(io::Error::other)
            })
        });
        let reply = self.call(job)?;
        if reply.failed {
            let c = self.refusal(&reply, "host-notice-undo-conflict");
            eprintln!(
                "fs-starter: undo module={} {}",
                self.module,
                super::starter::line(&c)
            );
            return Err(c);
        }
        Ok(reply)
    }

    /// One step back (`redo = false`) or forward: reverts the last unit of the window (a redo reverts the undo). The unit stays on its
    /// stack when the module refuses (a conflict is shown, nothing is overwritten).
    ///
    /// # Errors
    /// `host-notice-undo-conflict` (with `by-program`, `by-person`), other refusals as their code, `host-notice-undo-empty` when there is no step.
    pub(super) fn step_back(&self, redo: bool) -> Result<(), Coded> {
        let id = {
            let mut h = self
                .history
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if redo { h.undone.pop() } else { h.done.pop() }
        }
        .ok_or_else(|| cause("host-notice-undo-empty", &self.module, "history"))?;
        let put_back = |id: u128| {
            let mut h = self
                .history
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if redo {
                h.undone.push(id);
            } else {
                h.done.push(id);
            }
        };
        match self.revert(id) {
            Ok(reply) => {
                if let Some(c) = reply.change {
                    let mut h = self
                        .history
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    if redo {
                        h.done.push(c.activity);
                    } else {
                        h.undone.push(c.activity);
                    }
                }
                Ok(())
            }
            Err(e) => {
                put_back(id);
                Err(e)
            }
        }
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("module", &self.module)
            .field("me", &self.me)
            .finish_non_exhaustive()
    }
}
