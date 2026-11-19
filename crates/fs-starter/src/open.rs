//! The start of a frontend: socket, server, manifests, choice, check, fallback, hooks, open.
//!
//! Pattern: Template method (the order is fixed; every step is a part of [`Parts`]). Generalises activities `starter.rs`
//! `run` (:95-149).

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::time::Instant;

use fs_types::coded::Coded;

use crate::codes::{StarterCode, keys};
use crate::decide::{Chosen, Next, decide, pick, precheck};
use crate::frontend::{Listing, Trust};
use crate::gate::taken_over_code;
use crate::hook::{Ended, HookOutcome, StartCx};
use crate::names::FrontendId;
use crate::program::Exit;
use crate::run::{Parts, say};
use crate::tell::Level;
use crate::words::StartWords;

/// A stable word for the class of a failed start (own table: fs-rpc keeps its `kind_token` private, see draft O6).
fn kind_word(kind: io::ErrorKind) -> &'static str {
    match kind {
        io::ErrorKind::NotFound => "not-found",
        io::ErrorKind::PermissionDenied => "permission-denied",
        _ => "other",
    }
}

/// Finds `id`: its manifest, its program file, the precheck, and (only if `ask`) its answer to `--check`.
fn locate(
    parts: &Parts<'_>,
    listing: &Listing,
    id: &FrontendId,
    ask: bool,
    socket: &[OsString],
) -> (Chosen, Option<PathBuf>) {
    let Some(found) = listing.get(id) else {
        return (Chosen::Missing, None);
    };
    if found.trust == Trust::LocalUnchecked {
        let code =
            Coded::new(StarterCode::ManifestUnchecked).with_text(keys::FRONTEND, id.as_str());
        say(parts, Level::Hint, code.erase());
    }
    let Some(exe) = parts.launcher.find(&found.manifest.exec) else {
        return (Chosen::Missing, None);
    };
    if let Err(code) = precheck(found, parts.program) {
        return (Chosen::Refused(code), Some(exe));
    }
    // The standard frontend is not asked: it opens anyway and shows a server that does not fit in its own window.
    let state = if ask {
        Chosen::Checked(parts.launcher.check(&exe, socket))
    } else {
        Chosen::Checked(Some(0))
    };
    (state, Some(exe))
}

/// Settles which frontend opens, telling every fallback; `Err` = nothing can open (already told).
fn settle(
    parts: &Parts<'_>,
    listing: &Listing,
    w: &StartWords,
    socket: &[OsString],
) -> Result<(FrontendId, PathBuf), Exit> {
    let program = parts.program;
    let standard = program.standard_frontend();
    let chosen = pick(
        w.frontend.as_ref(),
        parts.preference.frontend(program.name()),
        listing.default_frontend().map(|f| f.manifest.id.clone()),
        standard,
    );
    let (state, exe) = locate(parts, listing, &chosen, &chosen != standard, socket);
    let (next, exe) = match decide(&chosen, standard, state) {
        Next::Fallback(why) => {
            say(parts, Level::Hint, why.erase());
            let (state, exe) = locate(parts, listing, standard, false, socket);
            (decide(standard, standard, state), exe)
        }
        next => (next, exe),
    };
    match (next, exe) {
        (Next::Open(id, hint), Some(exe)) => {
            if let Some(code) = hint {
                say(parts, Level::Hint, code.erase());
            }
            Ok((id, exe))
        }
        (Next::Fail(code) | Next::Fallback(code), _) => {
            say(parts, Level::Error, code.erase());
            Err(Exit::FAILED)
        }
        (Next::Open(id, _), None) => {
            say(
                parts,
                Level::Error,
                Coded::new(StarterCode::FrontendMissing)
                    .with_text(keys::FRONTEND, id.as_str())
                    .erase(),
            );
            Err(Exit::FAILED)
        }
    }
}

/// The whole start of a frontend.
pub(crate) fn start(parts: &Parts<'_>, w: &StartWords) -> Exit {
    let program = parts.program;
    let Some(socket) = w.socket.clone().or_else(|| parts.default_socket.clone()) else {
        say(
            parts,
            Level::Error,
            Coded::new(StarterCode::NoSocket).erase(),
        );
        return Exit::FAILED;
    };
    // The server first. Not fatal: the frontend shows the same in its window (activities `starter.rs:117-121`).
    match parts.gate.ensure(&socket) {
        Ok(up) => {
            if let Some(t) = up.taken_over {
                say(parts, Level::Hint, taken_over_code(&t).erase());
            }
        }
        Err(code) => say(parts, Level::Error, code.erase()),
    }
    let listing = parts.manifests.frontends(program.name());
    for refused in &listing.refused {
        say(parts, Level::Hint, refused.clone().erase());
    }
    // Only the first `fs:default` counts (listing order); each further one is told, never silently ignored.
    for extra in listing.found.iter().filter(|f| f.manifest.default).skip(1) {
        let file = extra.file.to_string_lossy();
        let code = Coded::new(StarterCode::ManifestInvalid)
            .with_text(keys::FILE, file)
            .with_text(keys::FIELD, "fs:default");
        say(parts, Level::Hint, code.erase());
    }
    let mut words: Vec<OsString> = vec!["--socket".into(), socket.into_os_string()];
    let (id, exe) = match settle(parts, &listing, w, &words) {
        Ok(found) => found,
        Err(exit) => return exit,
    };
    let cx = StartCx {
        program: program.name(),
        version: program.version(),
        frontend: &id,
        fresh: w.fresh,
        resumed: w.resumed.is_some(),
    };
    for hook in parts.hooks {
        if let HookOutcome::Stop(failure) = hook.before_open(&cx) {
            say(parts, Level::Error, failure.code);
            return failure.exit;
        }
    }
    if w.fresh {
        words.push("--fresh".into());
    }
    if let Some(token) = &w.resumed {
        words.extend(["--resumed".into(), token.as_os_str().to_owned()]);
    }
    let began = Instant::now();
    match parts.launcher.open(&exe, &words) {
        Ok(code) => {
            let ended = Ended {
                code,
                after: began.elapsed(),
            };
            for hook in parts.hooks {
                hook.after_end(&cx, &ended);
            }
            Exit::of_frontend(code)
        }
        Err(e) => {
            let code = Coded::new(StarterCode::OpenFailed)
                .with_text(keys::FRONTEND, id.as_str())
                .with_text(keys::KIND, kind_word(e.kind()));
            say(parts, Level::Error, code.erase());
            Exit::FAILED
        }
    }
}
