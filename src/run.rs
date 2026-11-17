//! `run`: the whole of `fs-<program>`, over its parts. A program's `main` is `fs_starter::main_exit(&parts)`.
//!
//! Pattern: Template method + Facade. Replaces activities `fs-activities-bin/src/main.rs` (:26-53; six copies, tasks/people/...
//! have their own `main.rs`).

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use fs_types::coded::{Code, Coded};

use crate::codes::StarterCode;
use crate::frontend::ManifestSource;
use crate::gate::ServerGate;
use crate::hook::StartHook;
use crate::launch::Launcher;
use crate::program::{Exit, Failure, McpEntry, Preference, Program};
use crate::tell::{Level, Note, Tell};
use crate::words::{Words, parse, synopsis};

/// Everything the starter works with. The program's `-bin` fills it (standard parts: [`crate::SingleGate`],
/// [`crate::XdgManifests::from_env`], [`crate::NoPreference`], [`crate::ProcessLauncher::standard`], [`crate::StderrTell`]).
pub struct Parts<'a> {
    /// The program.
    pub program: &'a dyn Program,
    /// The server gate.
    pub gate: &'a dyn ServerGate,
    /// Where frontend manifests come from.
    pub manifests: &'a dyn ManifestSource,
    /// The person's choice of frontend.
    pub preference: &'a dyn Preference,
    /// Runs frontends.
    pub launcher: &'a dyn Launcher,
    /// Where notes go.
    pub tell: &'a dyn Tell,
    /// Hooks around opening (fs-update's probation, ...); in order.
    pub hooks: &'a [&'a dyn StartHook],
    /// The MCP entry, if this build has one.
    pub mcp: Option<&'a dyn McpEntry>,
    /// The socket when no `--socket` is given (`fs_rpc::default_socket(<name>)`); `None` = there is none.
    pub default_socket: Option<PathBuf>,
}

/// Tells one note.
pub(crate) fn say(parts: &Parts<'_>, level: Level, code: Code) {
    parts.tell.tell(parts.program.name(), &Note { level, code });
}

fn failed(parts: &Parts<'_>, f: Failure) -> Exit {
    say(parts, Level::Error, f.code);
    f.exit
}

/// Runs `fs-<program> <words>`; `out` is where `--version` and `--help` write (stdout).
pub fn run(parts: &Parts<'_>, words: &[OsString], out: &mut dyn Write) -> Exit {
    let program = parts.program;
    let words = match parse(words) {
        Ok(w) => w,
        Err(code) => {
            say(parts, Level::Error, code.erase());
            return Exit::USAGE;
        }
    };
    match words {
        Words::Version => written(
            parts,
            writeln!(out, "fs-{} {}", program.name(), program.version()),
        ),
        Words::Help => written(parts, out.write_all(synopsis(program.name()).as_bytes())),
        Words::Serve(rest) => program
            .serve(&rest)
            .map_or_else(|f| failed(parts, f), |()| Exit::DONE),
        Words::Mcp { actor } => {
            // The actor is the one given, unchanged; the entry decides whether MCP is on and what the actor may do.
            if let Some(entry) = parts.mcp {
                entry
                    .serve_mcp(&actor)
                    .map_or_else(|f| failed(parts, f), |()| Exit::DONE)
            } else {
                say(
                    parts,
                    Level::Error,
                    Coded::new(StarterCode::McpMissing).erase(),
                );
                Exit::FAILED
            }
        }
        Words::Start(start) => crate::open::start(parts, &start),
    }
}

/// A failed write of `--version`/`--help` (closed stdout) is exit 1, told.
fn written(parts: &Parts<'_>, r: io::Result<()>) -> Exit {
    match r {
        Ok(()) => Exit::DONE,
        Err(e) => {
            let kind = if e.kind() == io::ErrorKind::BrokenPipe {
                "broken-pipe"
            } else {
                "other"
            };
            let code = Coded::new(StarterCode::OutputFailed).with_text(crate::keys::KIND, kind);
            say(parts, Level::Error, code.erase());
            Exit::FAILED
        }
    }
}

/// `run` with the process's words and stdout: the whole `main` of a program.
#[must_use]
pub fn main_exit(parts: &Parts<'_>) -> ExitCode {
    let words: Vec<OsString> = std::env::args_os().skip(1).collect();
    ExitCode::from(run(parts, &words, &mut io::stdout().lock()).code())
}
