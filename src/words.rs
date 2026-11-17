//! The words of `fs-<program>`: read once, typed, strict (POSIX Utility Conventions, XBD 12.2: options with their value as the
//! next word; a wrong word is exit 2).
//!
//! Pattern: Interpreter (a small grammar) → Value object [`Words`]. Taken from activities `fs-activities-bin/src/starter.rs`
//! `read` (:51) and `main.rs` (:28-39), six copies today.

use std::ffi::OsString;
use std::path::PathBuf;

use crate::coded_stub::Coded;

use crate::codes::{StarterCode, keys};
use crate::names::{ActorName, FrontendId, ProgramName, ResumeToken};

/// What the person (or a script) asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Words {
    /// `--version` / `-V`, alone.
    Version,
    /// `--help` / `-h`, alone.
    Help,
    /// `serve` and the server's own words, unread here.
    Serve(Vec<OsString>),
    /// No first word of the above: start a frontend.
    Start(StartWords),
    /// `mcp --as <actor>`.
    Mcp {
        /// The acting AI, exactly as given.
        actor: ActorName,
    },
}

/// The words of a start. `fresh` and `resumed` are only passed on (S1: no edge to fs-session).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StartWords {
    /// `--frontend ID`.
    pub frontend: Option<FrontendId>,
    /// `--socket PATH`.
    pub socket: Option<PathBuf>,
    /// `--fresh`: start without the saved session (ADR 0069 rule 8).
    pub fresh: bool,
    /// `--resumed TOKEN`: the restart after an update (ADR 0069 addendum).
    pub resumed: Option<ResumeToken>,
}

fn usage(word: &str) -> Coded<StarterCode> {
    Coded::new(StarterCode::Usage).with_text(keys::WORD, word)
}

/// Reads the words after the program name.
///
/// # Errors
/// `usage` (parameter `word`: the unknown, repeated or empty-valued option, or the second word after `--version`/`--help`;
/// never the value after `--resumed`), `actor-missing` (`mcp` without `--as NAME`).
pub fn parse(words: &[OsString]) -> Result<Words, Coded<StarterCode>> {
    let first = words.first().map(|w| w.to_string_lossy());
    match first.as_deref() {
        Some("--version" | "-V") => alone(words, Words::Version),
        Some("--help" | "-h") => alone(words, Words::Help),
        Some("serve") => Ok(Words::Serve(words[1..].to_vec())),
        Some("mcp") => mcp(&words[1..]),
        _ => start(words).map(Words::Start),
    }
}

fn alone(words: &[OsString], w: Words) -> Result<Words, Coded<StarterCode>> {
    match words.get(1) {
        None => Ok(w),
        Some(second) => Err(usage(&second.to_string_lossy())),
    }
}

fn mcp(rest: &[OsString]) -> Result<Words, Coded<StarterCode>> {
    match rest {
        [flag, name] if flag == "--as" => {
            let text = name.to_string_lossy();
            ActorName::parse(&text)
                .map(|actor| Words::Mcp { actor })
                .ok_or_else(|| usage(&text))
        }
        [flag, _, extra, ..] if flag == "--as" => Err(usage(&extra.to_string_lossy())),
        [other, ..] if other != "--as" => Err(usage(&other.to_string_lossy())),
        _ => Err(Coded::new(StarterCode::ActorMissing)),
    }
}

fn start(words: &[OsString]) -> Result<StartWords, Coded<StarterCode>> {
    let mut w = StartWords::default();
    let mut it = words.iter();
    while let Some(word) = it.next() {
        let name = word.to_string_lossy().into_owned();
        let mut value = || {
            it.next()
                .filter(|v| !v.is_empty() && !v.to_string_lossy().starts_with("--"))
                .ok_or_else(|| usage(&name))
        };
        match name.as_str() {
            "--frontend" if w.frontend.is_none() => {
                let v = value()?.to_string_lossy().into_owned();
                w.frontend = Some(FrontendId::parse(&v).ok_or_else(|| usage(&name))?);
            }
            "--socket" if w.socket.is_none() => w.socket = Some(PathBuf::from(value()?)),
            "--fresh" if !w.fresh && w.resumed.is_none() => w.fresh = true,
            "--resumed" if w.resumed.is_none() && !w.fresh => {
                w.resumed = Some(ResumeToken::parse(value()?).ok_or_else(|| usage(&name))?);
            }
            _ => return Err(usage(&name)),
        }
    }
    Ok(w)
}

/// The synopsis for `--help`: grammar only, no prose, so no language (the help itself is the documentation, ADR 0072).
#[must_use]
pub fn synopsis(program: &ProgramName) -> String {
    let p = format!("fs-{program}");
    format!(
        "{p} [--frontend ID] [--socket PATH] [--fresh | --resumed TOKEN]\n{p} serve [WORDS]\n{p} mcp --as ACTOR\n{p} --version | --help\n"
    )
}
