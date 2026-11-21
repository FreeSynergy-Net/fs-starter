//! The parts of the starter library that the host builds itself: `Parts` with MCP switched off (`McpOff`), and the start hooks
//! around the window. Without this the host binary would not enforce "MCP is off after installation" (ADR 0097) and would not
//! give the update probation (ADR 0069 addendum, item 5) a place to hook in.
//!
//! The hook is a no-op: where the one host binds `fs-update` is an open point (ADR 0105 Offen 9). It is named, not invented:
//! the line `update-binding-pending` (see [`crate::pending`]) says it on every window start.
//!
//! Pattern: Facade over the `fs-starter` library (its `Parts`, `run` and `StartHook` stay the one implementation).

use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use fs_starter::{ContractReq, StarterCode, keys};
use fs_starter::{
    Ended, Exit, Failure, FrontendId, HookOutcome, McpOff, NoPreference, Parts, ProcessLauncher,
    Program, ProgramName, ResumeToken, ServerGate, StartCx, StartHook, StderrTell, Up,
    XdgManifests,
};
use fs_types::coded::Coded;

/// The host as a "program" for the library: it has a name and a version, no server and no frontends of its own.
struct HostProgram {
    name: ProgramName,
    contract: ContractReq,
    frontend: FrontendId,
}

impl HostProgram {
    fn new() -> Self {
        Self {
            name: ProgramName::parse("starter").expect("a valid name"),
            contract: ContractReq::parse("starter 0.0").expect("a valid contract"),
            frontend: FrontendId::parse("simple").expect("a valid frontend id"),
        }
    }
}

impl Program for HostProgram {
    fn name(&self) -> &ProgramName {
        &self.name
    }
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
    fn app_id(&self) -> &'static str {
        "net.freesynergy.Starter"
    }
    fn contract(&self) -> &ContractReq {
        &self.contract
    }
    fn standard_frontend(&self) -> &FrontendId {
        &self.frontend
    }
    fn serve(&self, _words: &[OsString]) -> Result<(), Failure> {
        // The host has no server: `serve` is a wrong word here, said with the starter's own code.
        Err(Failure {
            exit: Exit::USAGE,
            code: Coded::new(StarterCode::Usage)
                .with_text(keys::WORD, "serve")
                .erase(),
        })
    }
}

/// No server to make sure (the host opens a description, not a server): asking it is a bug, so it says `no-socket`.
struct NoGate;

impl ServerGate for NoGate {
    fn ensure(&self, _socket: &Path) -> Result<Up, Coded<StarterCode>> {
        Err(Coded::new(StarterCode::NoSocket))
    }
}

/// The default hook until `fs-update` is bound (ADR 0105 Offen 9): lets the start go, supplies no token, sees the end.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopHook;

impl StartHook for NoopHook {
    fn before_open(&self, _cx: &StartCx<'_>) -> HookOutcome {
        HookOutcome::Go
    }
    fn after_end(&self, _cx: &StartCx<'_>, _ended: &Ended) {}
    fn resume_token(&self, _cx: &StartCx<'_>) -> Option<ResumeToken> {
        None
    }
}

/// Builds the `Parts` of the host (MCP off, the no-op hook) and hands them to `f`.
fn with_parts<R>(f: impl FnOnce(&Parts<'_>) -> R) -> R {
    with_hooks(&[&NoopHook], f)
}

/// Builds the `Parts` of the host with the given hooks (the host passes the no-op hook; tests pass recorders).
fn with_hooks<R>(hooks: &[&dyn StartHook], f: impl FnOnce(&Parts<'_>) -> R) -> R {
    let program = HostProgram::new();
    let (gate, manifests, launcher) = (
        NoGate,
        XdgManifests::from_env(),
        ProcessLauncher::standard(),
    );
    let parts = Parts {
        program: &program,
        gate: &gate,
        manifests: &manifests,
        preference: &NoPreference,
        launcher: &launcher,
        tell: &StderrTell,
        hooks,
        mcp: None,
        mcp_switch: &McpOff,
        default_socket: None,
    };
    f(&parts)
}

/// The words `mcp --as <actor>` go through the library: with `McpOff` they end in `mcp-disabled`, exit 1; the host never serves MCP.
#[must_use]
pub fn run_library(words: &[OsString], out: &mut dyn io::Write) -> ExitCode {
    with_parts(|parts| ExitCode::from(fs_starter::run(parts, words, out).code()))
}

/// Runs `open` (the window) between the hooks: `before_open` may stop the start, `after_end` sees the code and the time (the
/// update hook decides on the probation there; the default hook does nothing).
///
/// Session part pending: this does not call `StartHook::resume_token`, and `StartCx.fresh` is always `false` (even with `--fresh`),
/// because the host does not read the start words of the library here. `resumed` is only what the caller passes. Both belong to the
/// session part that is not built yet (README and CHANGELOG list it).
pub fn around_window(frontend: &str, resumed: bool, open: impl FnOnce() -> ExitCode) -> ExitCode {
    with_parts(|parts| run_around(parts, frontend, resumed, open))
}

fn run_around(
    parts: &Parts<'_>,
    frontend: &str,
    resumed: bool,
    open: impl FnOnce() -> ExitCode,
) -> ExitCode {
    let id =
        FrontendId::parse(frontend).unwrap_or_else(|| parts.program.standard_frontend().clone());
    let cx = StartCx {
        program: parts.program.name(),
        version: parts.program.version(),
        frontend: &id,
        fresh: false, // not wired: session part pending (see the doc above)
        resumed,
    };
    for hook in parts.hooks {
        if let HookOutcome::Stop(f) = hook.before_open(&cx) {
            eprintln!("fs-starter: {}", f.code);
            return ExitCode::from(f.exit.code());
        }
    }
    let started = Instant::now();
    let code = open();
    // ExitCode has no accessor; the hook gets the time, and 0/1 as the coarse result known here.
    let ended = Ended {
        code: i32::from(code != ExitCode::SUCCESS),
        after: started.elapsed(),
    };
    for hook in parts.hooks {
        hook.after_end(&cx, &ended);
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    /// Records the calls; `stop` makes `before_open` refuse.
    struct Spy<'a> {
        stop: bool,
        log: &'a RefCell<Vec<String>>,
    }

    impl StartHook for Spy<'_> {
        fn before_open(&self, _cx: &StartCx<'_>) -> HookOutcome {
            self.log.borrow_mut().push("before".into());
            if self.stop {
                HookOutcome::Stop(Failure {
                    exit: Exit::FAILED,
                    code: Coded::new(StarterCode::Usage).erase(),
                })
            } else {
                HookOutcome::Go
            }
        }
        fn after_end(&self, _cx: &StartCx<'_>, ended: &Ended) {
            self.log.borrow_mut().push(format!("after {}", ended.code));
        }
        fn resume_token(&self, _cx: &StartCx<'_>) -> Option<ResumeToken> {
            None
        }
    }

    #[test]
    fn a_stopping_before_open_prevents_the_window_and_after_end() {
        let log = RefCell::new(Vec::new());
        let opened = Cell::new(false);
        let spy = Spy {
            stop: true,
            log: &log,
        };
        let code = with_hooks(&[&spy], |p| {
            run_around(p, "simple", false, || {
                opened.set(true);
                ExitCode::SUCCESS
            })
        });
        assert!(!opened.get(), "the window must not run after a Stop");
        assert_eq!(code, ExitCode::from(Exit::FAILED.code()));
        assert_eq!(*log.borrow(), ["before"], "no after_end without a window");
    }

    #[test]
    fn after_end_runs_after_the_window_with_its_result() {
        let log = RefCell::new(Vec::new());
        let opened = Cell::new(false);
        let spy = Spy {
            stop: false,
            log: &log,
        };
        let code = with_hooks(&[&spy], |p| {
            run_around(p, "simple", false, || {
                opened.set(true);
                ExitCode::FAILURE
            })
        });
        assert!(opened.get());
        assert_eq!(code, ExitCode::FAILURE);
        assert_eq!(*log.borrow(), ["before", "after 1"]);
    }
}
