//! Which frontend opens: pure functions, no process, no file (tested without either).
//!
//! Pattern: Strategy as pure functions. `decide` is activities `starter.rs` `decide` (:31; six copies, calendars :27 its own
//! form) with typed codes instead of sentences, and calendars' rule that a missing standard frontend fails visibly (exit 1).

use fs_types::coded::Coded;

use crate::codes::{StarterCode, keys};
use crate::frontend::Found;
use crate::names::{FrontendId, parse_version};
use crate::program::{Exit, Program};

/// The frontend to try first: the word, else the person's choice, else the installation's `fs:default`, else the program's
/// standard frontend (ADR 0094 item 4 b).
#[must_use]
pub fn pick(
    word: Option<&FrontendId>,
    chosen_by_person: Option<FrontendId>,
    manifest_default: Option<FrontendId>,
    standard: &FrontendId,
) -> FrontendId {
    word.cloned()
        .or(chosen_by_person)
        .or(manifest_default)
        .unwrap_or_else(|| standard.clone())
}

fn contract(frontend: &FrontendId, field: &str) -> Coded<StarterCode> {
    Coded::new(StarterCode::FrontendContract)
        .with_text(keys::FRONTEND, frontend.as_str())
        .with_text(keys::FIELD, field)
}

/// The check of a manifest against the program before anything runs (ADR 0094 item 4 c): `<extends>` is the program,
/// its minimum program version is not above the program's, every `fs:requires` is served by the program's contract.
///
/// # Errors
/// `frontend-contract` with `frontend` and `field` (`extends`, `requires`, `fs:requires`).
pub fn precheck(found: &Found, program: &dyn Program) -> Result<(), Coded<StarterCode>> {
    let m = &found.manifest;
    if m.extends != program.app_id() {
        return Err(contract(&m.id, "extends"));
    }
    if let Some(min) = &m.min_program {
        let own = parse_version(program.version());
        if own.is_none_or(|own| own < *min) {
            return Err(contract(&m.id, "requires"));
        }
    }
    if m.requires.iter().any(|r| !r.matches(program.contract())) {
        return Err(contract(&m.id, "fs:requires"));
    }
    Ok(())
}

/// What is known about the chosen frontend before it opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Chosen {
    /// No usable manifest, or its program file is not found.
    Missing,
    /// The manifest does not fit the program ([`precheck`]).
    Refused(Coded<StarterCode>),
    /// Asked with `--check`: its exit code, `None` = could not run. The standard frontend is not asked.
    Checked(Option<i32>),
}

/// What the starter does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Next {
    /// Open this frontend; tell the hint first, if any (the standard frontend opens even when it does not fit: its own
    /// window says what is wrong).
    Open(FrontendId, Option<Coded<StarterCode>>),
    /// Tell `why` (it names `fallback`), then go on with the standard frontend.
    Fallback(Coded<StarterCode>),
    /// Tell it and end with exit 1: nothing can open.
    Fail(Coded<StarterCode>),
}

/// Decides from what is known about `chosen`.
#[must_use]
pub fn decide(chosen: &FrontendId, standard: &FrontendId, state: Chosen) -> Next {
    let named = |code: StarterCode| Coded::new(code).with_text(keys::FRONTEND, chosen.as_str());
    if chosen == standard {
        return match state {
            Chosen::Missing => Next::Fail(named(StarterCode::FrontendMissing)),
            Chosen::Refused(code) => Next::Open(chosen.clone(), Some(code)),
            Chosen::Checked(_) => Next::Open(chosen.clone(), None),
        };
    }
    let fallback = |code: Coded<StarterCode>| {
        Next::Fallback(code.with_text(keys::FALLBACK, standard.as_str()))
    };
    match state {
        Chosen::Missing => fallback(named(StarterCode::FrontendMissing)),
        Chosen::Refused(code) => fallback(code),
        Chosen::Checked(None) => fallback(named(StarterCode::FrontendUnstartable)),
        Chosen::Checked(Some(code)) if Exit::of_frontend(code) == Exit::CONTRACT => {
            fallback(named(StarterCode::FrontendContract).with_text(keys::FIELD, "check"))
        }
        // Any other answer: it runs and can speak for itself in its window (activities `starter.rs:42`).
        Chosen::Checked(Some(_)) => Next::Open(chosen.clone(), None),
    }
}
