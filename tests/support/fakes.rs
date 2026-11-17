//! Recording fakes of every port of the starter.

use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use fs_starter::{
    ActorName, Ended, Failure, FrontendId, HookOutcome, Launcher, Listing, ManifestSource,
    McpEntry, Note, Preference, ProgramName, ServerGate, StartCx, StartHook, StarterCode, Tell, Up,
};
use fs_types::coded::Coded;

pub struct FakeGate {
    pub answer: Result<Up, Coded<StarterCode>>,
    pub calls: Cell<u32>,
}

impl ServerGate for FakeGate {
    fn ensure(&self, _: &Path) -> Result<Up, Coded<StarterCode>> {
        self.calls.set(self.calls.get() + 1);
        self.answer.clone()
    }
}

pub struct FakeManifests(pub Listing);

impl ManifestSource for FakeManifests {
    fn frontends(&self, _: &ProgramName) -> Listing {
        self.0.clone()
    }
}

pub struct FakePref(pub Option<FrontendId>);

impl Preference for FakePref {
    fn frontend(&self, _: &ProgramName) -> Option<FrontendId> {
        self.0.clone()
    }
}

/// What `open` answers.
#[derive(Clone, Copy)]
pub enum OpenAnswer {
    Exit(i32),
    Err(io::ErrorKind),
}

/// Knows the program files `/bin/fs-demo-<id>` for the ids in `known`; answers `--check` per exe.
pub struct FakeLauncher {
    pub known: Vec<String>,
    pub checks: Vec<(PathBuf, Option<i32>)>,
    pub open_answer: OpenAnswer,
    pub checked: RefCell<Vec<PathBuf>>,
    pub opened: RefCell<Vec<(PathBuf, Vec<OsString>)>>,
}

impl Launcher for FakeLauncher {
    fn find(&self, exec: &str) -> Option<PathBuf> {
        self.known
            .iter()
            .any(|k| k == exec)
            .then(|| Path::new("/bin").join(exec))
    }
    fn check(&self, exe: &Path, _: &[OsString]) -> Option<i32> {
        self.checked.borrow_mut().push(exe.to_owned());
        self.checks
            .iter()
            .find(|(p, _)| p == exe)
            .map_or(Some(0), |(_, a)| *a)
    }
    fn open(&self, exe: &Path, words: &[OsString]) -> io::Result<i32> {
        self.opened
            .borrow_mut()
            .push((exe.to_owned(), words.to_vec()));
        match self.open_answer {
            OpenAnswer::Exit(c) => Ok(c),
            OpenAnswer::Err(k) => Err(io::Error::from(k)),
        }
    }
}

#[derive(Default)]
pub struct FakeTell(pub RefCell<Vec<Note>>);

impl Tell for FakeTell {
    fn tell(&self, _: &ProgramName, note: &Note) {
        self.0.borrow_mut().push(note.clone());
    }
}

impl FakeTell {
    pub fn keys(&self) -> Vec<String> {
        self.0
            .borrow()
            .iter()
            .map(|n| n.code.message_key())
            .collect()
    }
    pub fn note(&self, key: &str) -> Option<Note> {
        self.0
            .borrow()
            .iter()
            .find(|n| n.code.message_key() == key)
            .cloned()
    }
}

/// A hook that goes on or stops; records what it saw.
pub struct FakeHook {
    pub stop: Option<Failure>,
    pub before: RefCell<Vec<(bool, bool, String)>>,
    pub after: RefCell<Vec<(i32, Duration, bool)>>,
}

impl FakeHook {
    pub fn go() -> Self {
        Self {
            stop: None,
            before: RefCell::default(),
            after: RefCell::default(),
        }
    }
}

impl StartHook for FakeHook {
    fn before_open(&self, cx: &StartCx<'_>) -> HookOutcome {
        self.before
            .borrow_mut()
            .push((cx.fresh, cx.resumed, cx.frontend.to_string()));
        self.stop.clone().map_or(HookOutcome::Go, HookOutcome::Stop)
    }
    fn after_end(&self, cx: &StartCx<'_>, ended: &Ended) {
        self.after
            .borrow_mut()
            .push((ended.code, ended.after, cx.resumed));
    }
}

pub struct FakeMcp {
    pub actors: RefCell<Vec<String>>,
}

impl McpEntry for FakeMcp {
    fn serve_mcp(&self, actor: &ActorName) -> Result<(), Failure> {
        self.actors.borrow_mut().push(actor.to_string());
        Ok(())
    }
}

/// A writer that always fails.
pub struct FailingWriter(pub io::ErrorKind);

impl io::Write for FailingWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::from(self.0))
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::from(self.0))
    }
}
