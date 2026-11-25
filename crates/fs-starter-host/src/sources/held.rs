//! The records of a window that HOLDS its module: while the window lives, one connection to the service stands (`fs_rpc::hold`), so the
//! module does not end after its idle time; the window closing drops the records, the connection is closed, and the module ends by
//! itself after the idle time. Every attach, re-attach, give-up and the end is said on stderr (`fs-starter: hold module=M attached`,
//! `... attached resync`, `... gave-up`, `... detached`), never silent. A command still goes over its own connection per call.
//!
//! Pattern: Decorator (the records of the read, plus the handle) + RAII (`Hold`: the handle is the registration) + Observer (`HoldEvent`).

use std::sync::{Arc, PoisonError};

use fs_components_kanban::{Answer, Reason};
use fs_forms_bind::ModuleForm;
use fs_forms_run::{ChangeCounter, Coded, FormData, FormRecords, Records, SideItem, UndoState};
use fs_rpc::{Hold, HoldEvent, HoldSpec, ReconnectTimes, hold, hold_calling};

use super::answer::Limits;
use super::end::EndOnDrop;
use super::known::Known;
use super::level::HISTORY_FROM;
use super::range::Version;
use super::service::{UNREACHABLE, cause, parts, socket_of};
use super::session::Session;
use super::watch::{Attach, Identity};

/// What `connect` gives the window: the records of the read and, as long as they live, the hold of the module.
///
/// A one-time command (`--picture`) holds too (with history at level 2); the service it started itself ends when these records are
/// dropped, one that was up before stays.
pub struct HeldRecords {
    records: FormRecords,
    held: Held,
    /// Last field: dropped after the hold, so the service ends when nobody holds it any more.
    _end: Option<EndOnDrop>,
}

/// How the module is held: level 1 only keeps the connection; level 2 is a [`Session`] (history, watch).
pub(super) enum Held {
    /// RAII: the handle is the registration, only dropping it matters.
    Plain(#[allow(dead_code)] Hold),
    Session(Arc<Session>),
}

impl HeldRecords {
    /// The records of a read, held by `hold` (if any).
    #[must_use]
    pub(super) fn new(records: FormRecords, held: Held, end: Option<EndOnDrop>) -> Self {
        Self {
            records,
            held,
            _end: end,
        }
    }

    fn session(&self) -> Option<&Arc<Session>> {
        match &self.held {
            Held::Session(s) => Some(s),
            Held::Plain(_) => None,
        }
    }

    /// The connection that takes a write to the module (the port `Writer`); `None` = nothing can be written.
    #[must_use]
    pub fn writer(&self) -> Option<Arc<dyn fs_forms_run::Writer>> {
        self.records.writer.clone()
    }

    /// A command of the host itself (`app.undo`, `app.redo`): one step back or forward. `None` = not a command of this kind.
    #[must_use]
    pub fn answer_app(&self, command: &str) -> Option<Answer> {
        match command {
            crate::registry::UNDO => Some(self.step_back(false)),
            crate::registry::REDO => Some(self.step_back(true)),
            _ => None,
        }
    }
}

impl Records for HeldRecords {
    fn side(&self) -> Vec<SideItem> {
        self.records.side()
    }
    fn is_sample(&self) -> bool {
        self.records.is_sample()
    }
    fn forms(&self) -> Arc<Vec<FormData>> {
        self.records.forms()
    }
    fn command_of(&self, form: &str, channel: &str) -> Option<String> {
        self.records.command_of(form, channel)
    }
    fn can_channel(&self, form: &str, channel: &str, record: Option<&str>) -> bool {
        self.records.can_channel(form, channel, record)
    }
    fn invoke(
        &self,
        form: &str,
        channel: &str,
        record: Option<&str>,
        values: &[(String, String)],
    ) -> Answer {
        self.records.invoke(form, channel, record, values)
    }
    fn stale(&self) -> Option<String> {
        self.records.stale()
    }
    fn history(&self) -> UndoState {
        self.session()
            .map_or_else(UndoState::default, |s| s.state())
    }
    fn step_back(&self, redo: bool) -> Answer {
        let Some(session) = self.session() else {
            return Answer::Refused(Reason::new("unsupported", ""));
        };
        if let Err(c) = session.step_back(redo) {
            // visible, never silent; the unit stays in the history
            eprintln!("fs-starter: {}", super::starter::line(&c));
            return Answer::Refused(Reason::new(c.code, ""));
        }
        // accepted: the forms are read again, as after a write
        let again = self.records.writer.as_ref().map(|w| w.reload());
        match again {
            Some(Ok(all)) => {
                *self
                    .records
                    .fresh
                    .write()
                    .unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(all));
                *self
                    .records
                    .stale
                    .write()
                    .unwrap_or_else(PoisonError::into_inner) = None;
            }
            _ => {
                *self
                    .records
                    .stale
                    .write()
                    .unwrap_or_else(PoisonError::into_inner) =
                    Some("host-notice-data-reload".to_owned());
            }
        }
        Answer::Accepted
    }
    fn changed(&self) -> u64 {
        self.session().map_or(0, |s| s.counter())
    }
}

/// The waits of the hold: the whole budget of a read (`limits.time`) is the ceiling, so the window never waits longer for the hold than
/// it waits for the read (the defaults of fs-rpc would block it 30 s + 10 s when no module answers).
fn times(limits: &Limits) -> ReconnectTimes {
    ReconnectTimes {
        attempt: limits.time / 4,
        give_up: limits.time / 2,
        ..ReconnectTimes::default()
    }
}

/// The line one event says on stderr; `None` for an event this host does not know yet (the enum grows).
#[must_use]
pub(super) fn event_line(module: &str, event: &HoldEvent) -> Option<String> {
    let what = match event {
        HoldEvent::Attached {
            resync_needed: false,
            ..
        } => "attached",
        HoldEvent::Attached {
            resync_needed: true,
            ..
        } => "attached resync",
        HoldEvent::GaveUp { error } => {
            return Some(format!(
                "fs-starter: hold module={module} gave-up error={}",
                error.code().as_str()
            ));
        }
        HoldEvent::Detached => "detached",
        _ => return None,
    };
    Some(format!("fs-starter: hold module={module} {what}"))
}

/// What the hold of one window needs besides the module: the negotiated level, the forms to watch and who this window is.
pub(super) struct Wish<'a> {
    pub level: u32,
    pub forms: &'a [ModuleForm],
    pub me: Identity,
}

/// Hold the service of `known`: the same program file and the same expectation as the read (`parts`, border 3), on the socket of the
/// module (border 4). From level 2 the connection also `introduce`s the window and `watch`es its forms (a [`Session`]); level 1
/// only holds. Blocks until the connection stands, at most `limits.time` (`times`); `say` gets each line.
///
/// # Errors
/// `host-notice-data-unreachable` (cause `hold`) when no connection stands in time; the codes of `parts` and `socket_of`.
pub(super) fn hold_module(
    known: &Known,
    limits: &Limits,
    wish: Wish<'_>,
    say: impl Fn(String) + Send + 'static,
) -> Result<Held, Coded> {
    let (_, expected, starter) = parts(known)?;
    let spec = HoldSpec::new(
        &known.name,
        socket_of(&known.name)?,
        expected,
        Box::new(starter),
    )
    .with_times(times(limits));
    let module = known.name.clone();
    let observer = move |event: HoldEvent| {
        if let Some(line) = event_line(&module, &event) {
            say(line);
        }
    };
    let unreachable = |_| cause(UNREACHABLE, &known.name, "hold");
    if wish.level < HISTORY_FROM {
        return hold(spec, observer).map(Held::Plain).map_err(unreachable);
    }
    let counter = ChangeCounter::new();
    let attach = Attach {
        module: known.name.clone(),
        me: wish.me.clone(),
        forms: wish
            .forms
            .iter()
            .map(|f| {
                // ONE spelling on the wire: `name@x.y.z` (a form never raises its patch)
                fs_interfaces::forms::key(
                    &f.form.name,
                    &Version::new(u64::from(f.form.major), u64::from(f.form.minor), 0),
                )
            })
            .collect(),
        counter: counter.clone(),
    };
    let hold = hold_calling(spec, move || Box::new(attach), observer).map_err(unreachable)?;
    Ok(Held::Session(Arc::new(Session::new(
        hold,
        &known.name,
        wish.me,
        limits.time,
        counter,
    ))))
}
#[cfg(test)]
mod tests {
    use super::*;
    use fs_rpc::{RpcError, RpcErrorCode};

    #[test]
    fn each_event_says_its_own_line_and_the_resync_is_told_apart() {
        let at = |resync_needed| HoldEvent::Attached {
            attempt: 1,
            resync_needed,
        };
        let line = |e: &HoldEvent| event_line("groups", e).unwrap();
        assert_eq!(line(&at(false)), "fs-starter: hold module=groups attached");
        assert_eq!(
            line(&at(true)),
            "fs-starter: hold module=groups attached resync"
        );
        assert_eq!(
            line(&HoldEvent::Detached),
            "fs-starter: hold module=groups detached"
        );
        let gave = HoldEvent::GaveUp {
            error: RpcError::new(RpcErrorCode::Unimplemented),
        };
        assert_eq!(
            line(&gave),
            "fs-starter: hold module=groups gave-up error=unimplemented"
        );
    }

    #[test]
    fn the_hold_never_waits_longer_than_a_read() {
        let t = times(&Limits::default());
        assert!(t.attempt + t.give_up < Limits::default().time);
        assert_eq!(t.ping, ReconnectTimes::default().ping);
    }
}
