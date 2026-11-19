//! The fakes of the bench: each records what the starter did with it and carries the switches of the bench mutants.
//!
//! Pattern: Fake objects (interior mutability, one start each).

use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use fs_types::coded::Coded;

use crate::codes::{StarterCode, keys};
use crate::frontend::{Listing, ManifestSource};
use crate::gate::{ServerGate, Up};
use crate::names::{ActorName, ProgramName};
use crate::program::{Failure, McpEntry};
use crate::tell::{Level, Note, Tell};

/// Counts its calls; fails when asked to, unless blind.
pub(crate) struct FakeGate {
    pub(crate) fails: bool,
    pub(crate) blind: bool,
    pub(crate) calls: Cell<u32>,
}

impl ServerGate for FakeGate {
    fn ensure(&self, _: &Path) -> Result<Up, Coded<StarterCode>> {
        self.calls.set(self.calls.get() + 1);
        if self.fails && !self.blind {
            return Err(
                Coded::new(StarterCode::Server).with_text(keys::CAUSE, "server-unresponsive")
            );
        }
        Ok(Up::default())
    }
}

/// A fixed listing.
pub(crate) struct FakeManifests(pub(crate) Listing);

impl ManifestSource for FakeManifests {
    fn frontends(&self, _: &ProgramName) -> Listing {
        self.0.clone()
    }
}

/// Records notes, or drops them all (mutant `silent-tell`).
#[derive(Default)]
pub(crate) struct RecordingTell {
    pub(crate) silent: bool,
    pub(crate) notes: RefCell<Vec<Note>>,
}

impl Tell for RecordingTell {
    fn tell(&self, _: &ProgramName, note: &Note) {
        if !self.silent {
            self.notes.borrow_mut().push(note.clone());
        }
    }
}

/// Finds the known program files in one folder, answers `--check` as arranged, records, and opens with exit 0.
pub(crate) struct FakeLauncher<'t> {
    pub(crate) dir: PathBuf,
    pub(crate) known: Vec<String>,
    pub(crate) answers: Vec<(PathBuf, Option<i32>)>,
    pub(crate) check_ignored: bool,
    pub(crate) socket_dropped: bool,
    pub(crate) fresh_dropped: bool,
    /// Mutant `words-told`: tells the words it opens with, as a launcher that logs would.
    pub(crate) words_to: Option<(&'t dyn Tell, &'t ProgramName)>,
    pub(crate) checked: RefCell<Vec<PathBuf>>,
    pub(crate) opened: RefCell<Vec<(PathBuf, Vec<OsString>)>>,
}

impl crate::launch::Launcher for FakeLauncher<'_> {
    fn find(&self, exec: &str) -> Option<PathBuf> {
        self.known
            .iter()
            .any(|k| k == exec)
            .then(|| self.dir.join(exec))
    }

    fn check(&self, exe: &Path, _: &[OsString]) -> Option<i32> {
        self.checked.borrow_mut().push(exe.to_path_buf());
        if self.check_ignored {
            return Some(0);
        }
        self.answers
            .iter()
            .find(|(p, _)| p == exe)
            .map_or(Some(0), |(_, a)| *a)
    }

    fn open(&self, exe: &Path, words: &[OsString]) -> io::Result<i32> {
        let mut words = words.to_vec();
        if self.socket_dropped {
            words.drain(..2.min(words.len()));
        }
        if self.fresh_dropped {
            words.retain(|w| w != "--fresh");
        }
        if let Some((tell, program)) = self.words_to {
            let all: Vec<String> = words
                .iter()
                .map(|w| w.to_string_lossy().into_owned())
                .collect();
            let code = Coded::new(StarterCode::OpenFailed)
                .with_text(keys::WORD, all.join(" "))
                .erase();
            tell.tell(
                program,
                &Note {
                    level: Level::Hint,
                    code,
                },
            );
        }
        self.opened.borrow_mut().push((exe.to_path_buf(), words));
        Ok(0)
    }
}

/// Records the actor; mutant `actor-from-env` records the session's user instead, as a careless entry would.
pub(crate) struct FakeMcp {
    pub(crate) from_env: bool,
    pub(crate) actors: RefCell<Vec<String>>,
}

impl McpEntry for FakeMcp {
    fn serve_mcp(&self, actor: &ActorName) -> Result<(), Failure> {
        let who = if self.from_env {
            "user-of-the-session".to_owned()
        } else {
            actor.as_str().to_owned()
        };
        self.actors.borrow_mut().push(who);
        Ok(())
    }
}
