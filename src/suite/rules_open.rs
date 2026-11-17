//! The rules of a start: the standard frontend, fallbacks, failures that must be told, and the words passed on.
//!
//! Pattern: Strategy (one struct per rule, generic over the fixture: fs-conformance review condition A1).

use super::check::{arrange, exit, gate, opened, param, rule, told, words_have};
use super::codes::{SuiteCode, keys};
use super::fixture::Arrange;
use crate::codes::{StarterCode, keys as starter_keys};
use crate::names::FrontendId;
use crate::program::Exit;
use crate::tell::Level;

/// A frontend next to the standard one.
fn other() -> FrontendId {
    FrontendId::parse("other").expect("a well-formed id")
}

/// A resume token the rules look for where it must not be.
const TOKEN: &str = "token-4f1c9a";

rule!(
    /// No words: the server is asked once, the standard frontend opens (not asked with `--check`) with `--socket <path>`, its exit is passed on.
    OpenStandard, "starter.open-standard", |f, _cx| {
        let standard = f.program().standard_frontend().clone();
        let exe = f.exe_of(&standard);
        let o = f.start(&[]);
        gate(&o, 1)?;
        // The standard frontend is never asked with `--check` (it opens anyway and shows a contract that does not fit itself).
        if !o.checked.is_empty() {
            return Err(fs_conformance::Coded::new(SuiteCode::Launched).with_text(keys::WHAT, "check").into());
        }
        let words = opened(&o, &exe, standard.as_str())?;
        if words.len() < 2 || words[0] != "--socket" || words[1].is_empty() {
            return Err(fs_conformance::Coded::new(SuiteCode::WordMissing).with_text(keys::WORD, "--socket").into());
        }
        exit(&o, Exit::DONE)?;
        Ok(())
    }
);

rule!(
    /// A frontend that answers `--check` with 3: told (`frontend-contract`, `fallback`), the standard frontend opens.
    ContractFallback, "starter.contract-fallback", |f, cx| {
        let (other, standard) = (other(), f.program().standard_frontend().clone());
        arrange(f, cx, &Arrange::ExtraFrontend(other.clone()))?;
        arrange(f, cx, &Arrange::FrontendCheck(other.clone(), Some(3)))?;
        let (exe_other, exe_standard) = (f.exe_of(&other), f.exe_of(&standard));
        let o = f.start(&["--frontend", "other"]);
        if o.checked != [exe_other] {
            return Err(fs_conformance::Coded::new(SuiteCode::NotLaunched).with_text(keys::FRONTEND, "other").into());
        }
        let note = told(&o, Level::Hint, StarterCode::FrontendContract)?;
        param(note, starter_keys::FRONTEND, "other")?;
        param(note, starter_keys::FALLBACK, standard.as_str())?;
        opened(&o, &exe_standard, standard.as_str())?;
        Ok(())
    }
);

rule!(
    /// The standard frontend is missing: exit 1, `frontend-missing` told as an error, nothing opens.
    FrontendMissing, "starter.frontend-missing", |f, cx| {
        let standard = f.program().standard_frontend().clone();
        arrange(f, cx, &Arrange::FrontendGone(standard.clone()))?;
        let o = f.start(&[]);
        exit(&o, Exit::FAILED)?;
        let note = told(&o, Level::Error, StarterCode::FrontendMissing)?;
        param(note, starter_keys::FRONTEND, standard.as_str())?;
        super::check::nothing_opened(&o)?;
        Ok(())
    }
);

rule!(
    /// The server gate fails: told as an error (`server`), the standard frontend still opens and says it in its window.
    ServerProblem, "starter.server-problem", |f, cx| {
        let standard = f.program().standard_frontend().clone();
        arrange(f, cx, &Arrange::ServerFails)?;
        let exe = f.exe_of(&standard);
        let o = f.start(&[]);
        told(&o, Level::Error, StarterCode::Server)?;
        opened(&o, &exe, standard.as_str())?;
        Ok(())
    }
);

rule!(
    /// A manifest put there by hand: told as a hint (`manifest-unchecked`), the frontend still opens.
    LocalManifest, "starter.local-manifest", |f, cx| {
        let standard = f.program().standard_frontend().clone();
        arrange(f, cx, &Arrange::LocalManifest(standard.clone()))?;
        let exe = f.exe_of(&standard);
        let o = f.start(&[]);
        let note = told(&o, Level::Hint, StarterCode::ManifestUnchecked)?;
        param(note, starter_keys::FRONTEND, standard.as_str())?;
        opened(&o, &exe, standard.as_str())?;
        Ok(())
    }
);

rule!(
    /// `--resumed <token>`: the frontend gets the token; stdout and every note never contain it.
    ResumedHidden, "starter.resumed-hidden", |f, _cx| {
        let standard = f.program().standard_frontend().clone();
        let exe = f.exe_of(&standard);
        let o = f.start(&["--resumed", TOKEN]);
        words_have(opened(&o, &exe, standard.as_str())?, &["--resumed", TOKEN])?;
        let leak = |place: &str| fs_conformance::Coded::new(SuiteCode::Leaked).with_text(keys::PLACE, place);
        if o.stdout.contains(TOKEN) {
            return Err(leak("stdout").into());
        }
        if o.told.iter().any(|n| n.code.to_string().contains(TOKEN)) {
            return Err(leak("note").into());
        }
        Ok(())
    }
);

rule!(
    /// `--fresh`: the frontend gets `--fresh` (and no `--resumed`).
    FreshPassed, "starter.fresh-passed", |f, _cx| {
        let standard = f.program().standard_frontend().clone();
        let exe = f.exe_of(&standard);
        let o = f.start(&["--fresh"]);
        let words = opened(&o, &exe, standard.as_str())?;
        words_have(words, &["--fresh"])?;
        if words.iter().any(|w| w == "--resumed") {
            return Err(fs_conformance::Coded::new(SuiteCode::WordMissing).with_text(keys::WORD, "--fresh").into());
        }
        exit(&o, Exit::DONE)?;
        Ok(())
    }
);

rule!(
    /// A manifest with `fs:default` opens when no word names a frontend (checked with `--check`); a word beats it.
    ManifestDefault, "starter.manifest-default", |f, cx| {
        let (other, standard) = (other(), f.program().standard_frontend().clone());
        arrange(f, cx, &Arrange::DefaultFrontend(other.clone()))?;
        let exe_other = f.exe_of(&other);
        let o = f.start(&[]);
        if o.checked != [exe_other.clone()] {
            return Err(fs_conformance::Coded::new(SuiteCode::NotLaunched).with_text(keys::FRONTEND, "other").into());
        }
        opened(&o, &exe_other, other.as_str())?;
        let o = f.start(&["--frontend", standard.as_str()]);
        opened(&o, &f.exe_of(&standard), standard.as_str())?;
        Ok(())
    }
);
