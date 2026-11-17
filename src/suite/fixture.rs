//! The fixture of the suite: a program's starter that can be arranged and started, with everything it did recorded.
//!
//! Pattern: Test fixture behind a trait (a program may bring its own; [`super::Bench`] is the standard).

use std::ffi::OsString;
use std::path::PathBuf;

use crate::names::FrontendId;
use crate::program::{Exit, Program};
use crate::tell::Note;

/// A setup for the next start. Cleared by [`StarterFixture::reset`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arrange {
    /// A second frontend with a packaged manifest that fits.
    ExtraFrontend(FrontendId),
    /// The answer of this frontend to `--check` (`None`: it cannot run).
    FrontendCheck(FrontendId, Option<i32>),
    /// This frontend has no manifest and no program file.
    FrontendGone(FrontendId),
    /// This frontend's manifest lies in the user level, put there by hand.
    LocalManifest(FrontendId),
    /// The server gate fails (`server` with cause `server-unresponsive`).
    ServerFails,
}

/// What one start did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The exit.
    pub exit: Exit,
    /// What went to stdout.
    pub stdout: String,
    /// Every note, in order.
    pub told: Vec<Note>,
    /// How often the server gate was asked.
    pub gate_calls: u32,
    /// Program files asked with `--check`, in order.
    pub checked: Vec<PathBuf>,
    /// Program files opened, with their words, in order.
    pub opened: Vec<(PathBuf, Vec<OsString>)>,
    /// The actors the MCP entry was called with.
    pub mcp_actors: Vec<String>,
}

/// A program's starter under test.
pub trait StarterFixture {
    /// The program as it should behave (the expectations come from here).
    fn program(&self) -> &dyn Program;

    /// The program file a frontend of this fixture is found at.
    fn exe_of(&self, id: &FrontendId) -> PathBuf;

    /// Clears every arrangement (never a mutant). Every rule calls it first: the frame does not reset between rules.
    fn reset(&mut self);

    /// Arranges the next start; `false` = this fixture cannot (the rule then reports the need as missing).
    fn arrange(&mut self, setup: &Arrange) -> bool;

    /// Runs `fs-<program> <words>`.
    fn start(&mut self, words: &[&str]) -> Outcome;
}
