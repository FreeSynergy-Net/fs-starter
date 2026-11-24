//! More windows of one program (`--new-instance`, ADR 0097 and the draft N3): the first window keeps the endpoint
//! `<name>.window`; every further window takes the first FREE of `<name>.window-2` .. `<name>.window-9`. Who is free is decided
//! by the lock of `fs_rpc::bind_socket` (through [`crate::instance::claim_free`]), never by a window. A start without the flag
//! still hands its words to the first window.
//!
//! The name of a window for `introduce(instance)` is `<program>#<k>` ([`label`]; the first window is `#1`).
//!
//! Pattern: Strategy (the endpoint of slot `k` comes from a function) over the existing single-instance lock.

use std::path::PathBuf;
use std::process::ExitCode;

use fs_starter::{Exit, ProgramName};

use crate::instance::{self, Check, Claim, Guard, InstanceTimes};

/// The word of the host that asks for a new window next to the running ones.
pub const FLAG: &str = "--new-instance";
/// The code when all places `2..=9` are taken.
pub const FULL: &str = "instance-slots-full";
/// The first and the last place a further window can take.
const FIRST: u8 = 2;
const LAST: u8 = 9;

/// The words without `--new-instance`, and whether it was there (the flag is a word of the host, not of `fs-forms-run`).
/// It is taken out wherever it stands, also as the value of another word (`--description --new-instance`) and before
/// `--check-system`: a known edge, accepted (such a path is no program name).
#[must_use]
pub fn strip(args: &[String]) -> (Vec<String>, bool) {
    let words: Vec<String> = args.iter().filter(|a| *a != FLAG).cloned().collect();
    let found = words.len() != args.len();
    (words, found)
}

/// The name a window says in `introduce`: `<program>#<k>`.
#[must_use]
pub fn label(program: &str, k: u8) -> String {
    format!("{program}#{k}")
}

/// The endpoint of the further window `k` of the program: `default_socket("<name>.window-<k>")`. `None` without `XDG_RUNTIME_DIR`.
#[must_use]
pub fn endpoint_of(program: &ProgramName, k: u8) -> Option<PathBuf> {
    fs_rpc::default_socket(&format!("{program}.window-{k}"))
}

/// Take the smallest free place `2..=9`: `Some((k, guard))`, or `None` when there is no endpoint (no `XDG_RUNTIME_DIR`: the
/// program starts without the rule, like the first window does).
///
/// # Errors
/// `instance-slots-full` when `2..=9` are all taken, else the codes of [`instance::claim_free`].
pub fn claim_next(
    endpoint_of: &dyn Fn(u8) -> Option<PathBuf>,
    check: Check,
) -> Result<Option<(u8, Guard)>, String> {
    for k in FIRST..=LAST {
        let Some(path) = endpoint_of(k) else {
            return Ok(None);
        };
        if let Some(guard) = instance::claim_free(&path, check)? {
            return Ok(Some((k, guard)));
        }
    }
    Err(FULL.to_owned())
}

/// What a start holds: the place of its window (until it is closed) and the number of that window (`#1` is the first).
pub(crate) type Place = (Option<instance::Guard>, Option<u8>);

/// The single instance of the program: the first window holds the place and is `#1`; with `--new-instance` the window takes the
/// smallest free place `2..=9` (`slots.rs`). Nothing to hold: no program, no endpoint, or a picture, which has no window to protect
/// (a picture with `--new-instance` takes no place at all and hands nothing over). `Err(code)` this start ends (handed over: success).
/// A start that draws a picture takes the place only to learn who has it: a running window gets the words, else it lets go at once.
pub(crate) fn single(
    program: Option<&str>,
    args: &[String],
    picture: bool,
    new_window: bool,
) -> Result<Place, ExitCode> {
    let Some(name) = program else {
        return Ok((None, None));
    };
    let failed = |line: &str| {
        eprintln!("fs-starter: {line}");
        ExitCode::from(Exit::FAILED.code())
    };
    if new_window {
        if picture {
            return Ok((None, None));
        }
        let program = ProgramName::parse(name)
            .ok_or_else(|| failed(&format!("{}: {name}", instance::NAME_INVALID)))?;
        let of = |k| endpoint_of(&program, k);
        return match claim_next(&of, instance::words_are_a_start) {
            Ok(Some((k, guard))) => Ok((Some(guard), Some(k))),
            Ok(None) => {
                eprintln!("fs-starter: {}", instance::OFF);
                Ok((None, None))
            }
            Err(e) => Err(failed(&e)),
        };
    }
    let endpoint = instance::endpoint_for(name).map_err(|e| failed(&e))?;
    match instance::claim(endpoint.as_deref(), args, InstanceTimes::default()) {
        Ok(Claim::First(guard)) => Ok(((!picture).then_some(guard), Some(1))),
        Ok(Claim::HandedOver) => {
            eprintln!("fs-starter: {}", instance::HANDED_OVER);
            Err(ExitCode::from(Exit::DONE.code()))
        }
        Ok(Claim::Off(code)) => {
            eprintln!("fs-starter: {code}");
            Ok((None, None))
        }
        Err(e) => Err(failed(&e)),
    }
}
