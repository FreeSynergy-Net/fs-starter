//! One whole start with fakes: `World`.

use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

use fs_starter::{Exit, Level, Listing, McpEntry, Parts, StartHook, Trust, Up, run};

use super::{
    Demo, FakeGate, FakeHook, FakeLauncher, FakeManifests, FakeMcp, FakePref, FakeTell, OpenAnswer,
    found, os,
};

/// Everything one start needs. Fields are changed by the test before `go`.
pub struct World {
    pub program: Demo,
    pub gate: FakeGate,
    pub manifests: FakeManifests,
    pub pref: FakePref,
    pub launcher: FakeLauncher,
    pub tell: FakeTell,
    pub hooks: Vec<FakeHook>,
    pub mcp: Option<FakeMcp>,
    pub default_socket: Option<PathBuf>,
}

impl World {
    /// The standard frontend `simple` is listed (packaged) and found; the gate is fine; the frontend exits 0.
    pub fn new() -> Self {
        let listing = Listing {
            found: vec![found("simple", Trust::Packaged)],
            refused: Vec::new(),
        };
        Self {
            program: Demo::default(),
            gate: FakeGate {
                answer: Ok(Up::default()),
                calls: Cell::new(0),
            },
            manifests: FakeManifests(listing),
            pref: FakePref(None),
            launcher: FakeLauncher {
                known: vec!["fs-demo-simple".to_owned()],
                checks: Vec::new(),
                open_answer: OpenAnswer::Exit(0),
                checked: RefCell::default(),
                opened: RefCell::default(),
            },
            tell: FakeTell::default(),
            hooks: Vec::new(),
            mcp: Some(FakeMcp {
                actors: RefCell::default(),
            }),
            default_socket: Some(PathBuf::from("/run/demo.sock")),
        }
    }

    /// Adds a usable frontend `id` (packaged, program file known).
    pub fn with_frontend(mut self, id: &str, trust: Trust) -> Self {
        self.manifests.0.found.push(found(id, trust));
        self.launcher.known.push(format!("fs-demo-{id}"));
        self
    }

    pub fn exe(id: &str) -> PathBuf {
        PathBuf::from(format!("/bin/fs-demo-{id}"))
    }

    pub fn go(&self, words: &[&str]) -> Exit {
        self.go_out(words, &mut Vec::new())
    }

    pub fn go_out(&self, words: &[&str], out: &mut dyn io::Write) -> Exit {
        let hooks: Vec<&dyn StartHook> = self.hooks.iter().map(|h| h as &dyn StartHook).collect();
        let parts = Parts {
            program: &self.program,
            gate: &self.gate,
            manifests: &self.manifests,
            preference: &self.pref,
            launcher: &self.launcher,
            tell: &self.tell,
            hooks: &hooks,
            mcp: self.mcp.as_ref().map(|m| m as &dyn McpEntry),
            default_socket: self.default_socket.clone(),
        };
        run(&parts, &os(words), out)
    }

    /// Runs and returns stdout as text too.
    pub fn go_text(&self, words: &[&str]) -> (Exit, String) {
        let mut out = Vec::new();
        let exit = self.go_out(words, &mut out);
        (exit, String::from_utf8(out).unwrap())
    }

    pub fn opened(&self) -> Vec<(PathBuf, Vec<OsString>)> {
        self.launcher.opened.borrow().clone()
    }
}

pub fn level_of(w: &World, key: &str) -> Option<Level> {
    w.tell.note(key).map(|n| n.level)
}
