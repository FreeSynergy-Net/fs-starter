//! `fs-starter`: the program shell as a library. The `main` of every program becomes [`main_exit`] over its [`Parts`].
//!
//! `fs-<program>` without words is the starter (ADR 0063, ADR 0094 item 4): it makes sure the server runs ([`ServerGate`],
//! standard [`SingleGate`] over `fs_rpc::ensure_single`), picks the frontend (word > the person's choice > standard
//! frontend), checks its manifest ([`precheck`]), asks the frontend whether the contract fits (`--check`), and opens it, or
//! falls back to the standard frontend and says so. Further words: `--version`, `--help` (a synopsis, no prose: the help
//! itself is the documentation, ADR 0072), `serve` (the program's server), `mcp --as <actor>` (the MCP entry, if the
//! program build has one).
//!
//! Rules of the starter:
//! - Nothing fails silently: every failure and every fallback is a [`Note`] for [`Tell`], with a code and parameters, never
//!   a sentence (ADR 0057; the program words it in its MessageFormat 2 files).
//! - `mcp` needs `--as <actor>`; the starter never takes the actor from the session, the environment or the person:
//!   an AI works as its own named actor and never as the person (ADR 0091; rule "an AI may work, but not take over the user").
//!   The starter asks the [`McpSwitch`] (`fs.mcp.enabled`, off by default) before the MCP entry is served; what the actor may do
//!   is decided by the MCP entry, never here.
//! - The token of `--resumed` goes to the frontend only; it is in no note, no output, no hook (ADR 0069 addendum, item 5).
//! - The starter knows no session, no module and no update (review condition S1): `--fresh`/`--resumed` are words it
//!   passes on; update probation hooks in through [`StartHook`] (no edge to fs-update; ADR 0105 (a): the one host
//!   binds the hook, fs-update binding open, ADR 0105 open point 9).
//! - Every choice the person can make stays theirs: the frontend comes from [`Preference`] when no word names one.
//!
//! Pattern: Template method ([`run`]) + Strategy (every part is a trait with a standard implementation and a fake) +
//! Value objects ([`Words`], [`Exit`], codes).

#![deny(missing_docs)]
#![warn(clippy::pedantic)]

mod codes;
mod decide;
mod frontend;
mod gate;
mod hook;
mod launch;
mod manifest_xml;
mod mcp;
mod names;
mod open;
mod program;
mod run;
mod settings;
mod tell;
mod words;

#[cfg(feature = "conformance")]
pub mod suite;

pub use codes::{StarterCode, keys};
pub use decide::{Chosen, Next, decide, pick, precheck};
pub use frontend::{Found, FrontendManifest, Listing, ManifestSource, Place, Trust};
pub use gate::{ServerGate, SingleGate, TakenOverNote, Up, server_code};
pub use hook::{Ended, HookOutcome, PROBATION, StartCx, StartHook};
pub use launch::{CHECK_LIMIT, Launcher, ProcessLauncher};
pub use manifest_xml::{MAX_MANIFEST_BYTES, XdgManifests, read_manifest};
pub use mcp::{McpOff, McpSwitch};
pub use names::{ActorName, ContractReq, FrontendId, ProgramName, ResumeToken, Version3};
pub use program::{Exit, Failure, McpEntry, NoPreference, Preference, Program};
pub use run::{Parts, main_exit, run};
pub use settings::{MCP_ENABLED, SINGLE_ASK, StarterSettings, TEXT_DE, TEXT_EN};
pub use tell::{Level, Note, StderrTell, Tell};
pub use words::{StartWords, Words, parse, synopsis};
