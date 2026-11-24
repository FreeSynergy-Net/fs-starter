//! The records of a window that HOLDS its module: while the window lives, one connection to the service stands (`fs_rpc::hold`), so the
//! module does not end after its idle time; the window closing drops the records, the connection is closed, and the module ends by
//! itself after the idle time. Every attach, re-attach, give-up and the end is said on stderr (`fs-starter: hold module=M attached`,
//! `... attached resync`, `... gave-up`, `... detached`), never silent. A command still goes over its own connection per call.
//!
//! Pattern: Decorator (the records of the read, plus the handle) + RAII (`Hold`: the handle is the registration) + Observer (`HoldEvent`).

use std::sync::Arc;

use fs_components_kanban::Answer;
use fs_forms_run::{Coded, FormData, FormRecords, Records, SideItem};
use fs_rpc::{Hold, HoldEvent, HoldSpec, ReconnectTimes, hold};

use super::answer::Limits;
use super::known::Known;
use super::service::{UNREACHABLE, cause, parts, socket_of};

/// What `connect` gives the window: the records of the read and, as long as they live, the hold of the module.
///
/// `hold` is `None` only for a one-time command (`--picture`, `--check-system`): it ends the service it started, so nothing may
/// hold it again.
pub struct HeldRecords {
    records: FormRecords,
    _hold: Option<Hold>,
}

impl HeldRecords {
    /// The records of a read, held by `hold` (if any).
    #[must_use]
    pub fn new(records: FormRecords, hold: Option<Hold>) -> Self {
        Self {
            records,
            _hold: hold,
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

/// Hold the service of `known`: the same program file and the same expectation as the read (`parts`, border 3), on the socket of the
/// module (border 4). Blocks until the connection stands, at most `limits.time` (`times`); `say` gets each line.
///
/// # Errors
/// `host-notice-data-unreachable` (cause `hold`) when no connection stands in time; the codes of `parts` and `socket_of`.
pub(super) fn hold_module(
    known: &Known,
    limits: &Limits,
    say: impl Fn(String) + Send + 'static,
) -> Result<Hold, Coded> {
    let (_, expected, starter) = parts(known)?;
    let spec = HoldSpec::new(
        &known.name,
        socket_of(&known.name)?,
        expected,
        Box::new(starter),
    )
    .with_times(times(limits));
    let module = known.name.clone();
    hold(spec, move |event: HoldEvent| {
        if let Some(line) = event_line(&module, &event) {
            say(line);
        }
    })
    .map_err(|_| cause(UNREACHABLE, &known.name, "hold"))
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
