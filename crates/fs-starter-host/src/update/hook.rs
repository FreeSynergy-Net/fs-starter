//! The hook itself: the mark of the last run and the watcher of this run, behind `StartHook`.
//!
//! Pattern: Adapter – puts the Observer/Strategy parts of this module behind the library's `StartHook`.

use std::sync::{Arc, Mutex};

use super::{
    Mark, Marker, NoSignal, POLL, PackageLocks, Role, RpcAct, Running, SETTLE, Say, State, Watched,
    Watcher,
};
use fs_starter::{Ended, HookOutcome, ResumeToken, StartCx, StartHook};

/// How long the end of the window waits for the watcher.
const STOP_WAIT: std::time::Duration = std::time::Duration::from_secs(2);

/// The code of the line that says the hook is there (replaces `update-binding-pending`).
pub const ACTIVE_CODE: &str = "update-hook-active";

/// The line `--check-system` and the window start say: the hook is bound, the source of updates is not.
pub const ACTIVE_NOTE: &str = "update-hook-active note=\"watches the own file and the module services; the source of updates (UPD-3, fs-update) and the session (SB-12) are not there; the own window waits for a calm signal\"";

/// The hook: the marker of the last run, and the watcher of this run.
pub struct UpdateHook {
    program: String,
    marker: Option<Marker>,
    watcher: Mutex<Option<Watcher>>,
    running: Mutex<Option<Running>>,
    say: Say,
}

fn say_stderr(line: &str) {
    eprintln!("fs-starter: {line}");
}

impl UpdateHook {
    /// The hook of the host for `program`: watches the own file and the service of every known module.
    #[must_use]
    pub fn standard(
        program: &str,
        instance: Option<u8>,
        modules: &[crate::sources::Known],
    ) -> Self {
        let mut files = Vec::new();
        if let Some(own) = std::env::current_exe()
            .ok()
            .and_then(|p| Watched::begin(p, Role::Own))
        {
            files.push(own);
        }
        for k in modules {
            if let Some(w) = k
                .service()
                .ok()
                .and_then(|p| Watched::begin(p, Role::Module(k.name.clone())))
            {
                files.push(w);
            }
        }
        let marker = Marker::standard(program, instance);
        let act = RpcAct {
            own_exe: std::env::current_exe().unwrap_or_default(),
            args: std::env::args_os().skip(1).collect(),
            marker: marker.clone(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        };
        let watcher = Watcher::new(
            files,
            Box::new(PackageLocks::standard()),
            Box::new(NoSignal),
            Box::new(act),
            SETTLE,
        );
        Self::with(program, marker, watcher, Arc::new(say_stderr))
    }

    /// With given parts (tests).
    #[must_use]
    pub fn with(program: &str, marker: Option<Marker>, watcher: Watcher, say: Say) -> Self {
        Self {
            program: program.to_owned(),
            marker,
            watcher: Mutex::new(Some(watcher)),
            running: Mutex::new(None),
            say,
        }
    }
}

impl StartHook for UpdateHook {
    fn before_open(&self, cx: &StartCx<'_>) -> HookOutcome {
        let mut after_update = false;
        match &self.marker {
            Some(m) => {
                if let Some(last) = m.read() {
                    (self.say)(&last.said(&self.program).0);
                    after_update = last.after_update || last.state == State::Restarted;
                }
                if m.write(&Mark::running(cx.version, after_update)).is_err() {
                    (self.say)(
                        "update-marker-unwritable note=\"a crash of this run will not be told at the next start\"",
                    );
                }
            }
            None => (self.say)(
                "update-marker-off note=\"no state folder (XDG_STATE_HOME, HOME): a crash will not be told at the next start\"",
            ),
        }
        if let Some(w) = self.watcher.lock().ok().and_then(|mut g| g.take()) {
            if w.is_empty() {
                (self.say)(
                    "update-nothing-to-watch note=\"the own file and the module services cannot be stamped\"",
                );
            } else if let Ok(mut r) = self.running.lock() {
                *r = Some(Running::spawn(w, POLL, Arc::clone(&self.say)));
            }
        }
        HookOutcome::Go
    }

    fn after_end(&self, _cx: &StartCx<'_>, ended: &Ended) {
        if let Some(r) = self.running.lock().ok().and_then(|mut g| g.take())
            && !r.stop(STOP_WAIT)
        {
            (self.say)(
                "update-watcher-hung note=\"the watcher did not end in time (a call to a module hangs); it is left behind, the window ends\"",
            );
        }
        let Some(m) = &self.marker else { return };
        if ended.code == 0 {
            m.clear();
        } else if let Some(mut mark) = m.read() {
            mark.state = State::Exited(ended.code);
            let _ = m.write(&mark);
        }
    }

    fn resume_token(&self, _cx: &StartCx<'_>) -> Option<ResumeToken> {
        None
    }
}
