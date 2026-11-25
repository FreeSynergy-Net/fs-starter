//! The update hook of the host (UPD-4, ENTSCHEID-update section 4; ADR 0069 path A, ADR 0105 open point 9): the host notices that its own
//! file or the `service` of a module was replaced by a package manager, asks the old one to save (heartbeat `save`), ends it, and
//! lets the new one start; it checks how the last run ended and says it at the next start. It draws nothing: it says lines with
//! stable codes (the shell turns them into the status line).
//!
//! Pattern: Observer + Strategy. Observer: `Watched` files that are looked at by an active object (`watch.rs`); Strategy: `Transaction`
//! (the package lock), `Calm` (the rest rule), `Act` (what ending and restarting mean). The hook itself is an Adapter that puts them
//! behind the library's `StartHook` (Inversion of control, `fs_starter::StartHook`).
//!
//! What is there and what is not (said, never silent): the own window is NOT ended without a calm signal of the window (none yet:
//! `NoSignal`), so the new own file runs at the next start (ADR 0069 item 5); modules are ended after `save`. Without `fs-session` (SB-12)
//! a restart cannot restore the session: `update-session-not-restored` at the next start. The source of updates (UPD-3, `fs-update`)
//! is not here: the hook reacts to a replaced file, it does not fetch.

mod act;
mod hook;
mod marker;
mod quiet;
mod stamp;
mod watch;

pub use act::RpcAct;
pub use hook::{ACTIVE_CODE, ACTIVE_NOTE, UpdateHook};
pub use marker::{Mark, Marker, Said, State};
pub use quiet::{Calm, NoSignal, PackageLocks, SETTLE, Transaction};
pub use stamp::{Change, Role, Stamp, Watched};
pub use watch::{Act, POLL, Running, Say, Watcher};

#[cfg(test)]
mod tests;
