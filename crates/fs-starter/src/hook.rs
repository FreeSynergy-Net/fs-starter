//! Hooks around opening a frontend, so a building block above the starter can take part without an edge upwards.
//!
//! Pattern: Inversion of control (`StartHook`). First user: fs-update's probation (ADR 0069 addendum, item 5: 120 s; a new
//! version that dies before falls back to the previous one), wired in the program's `-bin` (ADR 0097 decision 1: fs-starter
//! never depends on fs-update).

use std::time::Duration;

use crate::names::{FrontendId, ProgramName};
use crate::program::Failure;

/// What a hook may know of a start. No resume token (only whether there is one).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartCx<'a> {
    /// The program.
    pub program: &'a ProgramName,
    /// Its version.
    pub version: &'a str,
    /// The frontend about to open.
    pub frontend: &'a FrontendId,
    /// `--fresh` was given.
    pub fresh: bool,
    /// `--resumed` was given.
    pub resumed: bool,
}

/// Whether the start goes on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HookOutcome {
    /// Go on.
    Go,
    /// Stop with this failure (told, then the exit).
    Stop(Failure),
}

/// How the frontend ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ended {
    /// Its exit code.
    pub code: i32,
    /// How long it ran.
    pub after: Duration,
}

/// A hook around opening the frontend. Both methods are required (no default methods, trait-delta rule 6).
pub trait StartHook {
    /// Before the frontend opens, after the server and the frontend are settled.
    fn before_open(&self, cx: &StartCx<'_>) -> HookOutcome;

    /// After the frontend ended (not called when it could not be run).
    fn after_end(&self, cx: &StartCx<'_>, ended: &Ended);
}
