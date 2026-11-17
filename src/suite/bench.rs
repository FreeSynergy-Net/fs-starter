//! The standard fixture: a program's real `Program` with fake parts, and the switches of the bench mutants.
//!
//! Pattern: Test fixture + Decorator (`VPrefixed`, mutant `version-v`). A program runs the suite as
//! `fs_conformance::run(&StarterSuite, &mut Bench::new(Box::new(MyProgram)), &Options::default())`.

use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::path::PathBuf;

use super::fakes::{FakeGate, FakeLauncher, FakeManifests, FakeMcp, RecordingTell};
use super::fixture::{Arrange, Outcome, StarterFixture};
use crate::frontend::{Found, FrontendManifest, Listing, Place, Trust};
use crate::names::{ContractReq, FrontendId, ProgramName};
use crate::program::{Exit, Failure, NoPreference, Program};
use crate::run::{Parts, run};

/// One wrong wiring a program could have. Each must make at least one rule fail (`bench_mutants`).
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchMutation {
    /// The version is printed as `v<version>`.
    VersionV,
    /// `--help` opens the frontend.
    HelpOpens,
    /// Every exit becomes 0.
    ExitSwallowed,
    /// The frontend loses `--socket <path>`.
    SocketDropped,
    /// `--check` is never really asked (always 0).
    CheckIgnored,
    /// Notes go nowhere.
    SilentTell,
    /// The gate never fails.
    GateBlind,
    /// A gone frontend is still listed and found.
    ManifestInvents,
    /// A hand-made manifest is claimed as packaged.
    TrustClaimed,
    /// The MCP entry takes the session's user instead of the actor given.
    ActorFromEnv,
    /// The launcher tells the words it opens with.
    WordsTold,
    /// The frontend loses `--fresh`.
    FreshDropped,
}

/// The bench. Folder of program files: `/bench/bin`; socket: [`Bench::SOCKET`].
pub struct Bench {
    program: Box<dyn Program>,
    mutation: Option<BenchMutation>,
    setups: Vec<Arrange>,
}

impl Bench {
    /// The socket the bench gives as default.
    pub const SOCKET: &'static str = "/bench/run/program.sock";

    /// A bench over `program`, without arrangements and without mutant.
    #[must_use]
    pub fn new(program: Box<dyn Program>) -> Self {
        Self {
            program,
            mutation: None,
            setups: Vec::new(),
        }
    }

    /// Switches a mutant on (kept across [`StarterFixture::reset`]).
    pub fn set_mutation(&mut self, mutation: BenchMutation) {
        self.mutation = Some(mutation);
    }

    fn is(&self, m: BenchMutation) -> bool {
        self.mutation == Some(m)
    }

    fn has(&self, setup: &Arrange) -> bool {
        self.setups.contains(setup)
    }

    fn listing(&self) -> Listing {
        let p = &*self.program;
        let mut ids = vec![p.standard_frontend().clone()];
        ids.extend(self.setups.iter().filter_map(|s| match s {
            Arrange::ExtraFrontend(id) => Some(id.clone()),
            _ => None,
        }));
        let mut out = Listing::default();
        for id in ids {
            if self.has(&Arrange::FrontendGone(id.clone()))
                && !self.is(BenchMutation::ManifestInvents)
            {
                continue;
            }
            let local = self.has(&Arrange::LocalManifest(id.clone()));
            let trust = if local && !self.is(BenchMutation::TrustClaimed) {
                Trust::LocalUnchecked
            } else {
                Trust::Packaged
            };
            let manifest = FrontendManifest {
                exec: format!("fs-{}-{id}", p.name()),
                extends: p.app_id().to_owned(),
                requires: vec![p.contract().clone()],
                min_program: None,
                id,
            };
            let file = PathBuf::from(format!("/bench/share/{}.metainfo.xml", manifest.id));
            out.found.push(Found {
                manifest,
                place: if local { Place::User } else { Place::System },
                trust,
                file,
            });
        }
        out
    }
}

/// Mutant `version-v`: the program, with `v` before its version.
struct VPrefixed<'p>(&'p dyn Program, String);

impl Program for VPrefixed<'_> {
    fn name(&self) -> &ProgramName {
        self.0.name()
    }
    fn version(&self) -> &str {
        &self.1
    }
    fn app_id(&self) -> &str {
        self.0.app_id()
    }
    fn contract(&self) -> &ContractReq {
        self.0.contract()
    }
    fn standard_frontend(&self) -> &FrontendId {
        self.0.standard_frontend()
    }
    fn serve(&self, words: &[OsString]) -> Result<(), Failure> {
        self.0.serve(words)
    }
}

impl StarterFixture for Bench {
    fn program(&self) -> &dyn Program {
        &*self.program
    }

    fn exe_of(&self, id: &FrontendId) -> PathBuf {
        PathBuf::from("/bench/bin").join(format!("fs-{}-{id}", self.program.name()))
    }

    fn reset(&mut self) {
        self.setups.clear();
    }

    fn arrange(&mut self, setup: &Arrange) -> bool {
        self.setups.push(setup.clone());
        true
    }

    fn start(&mut self, words: &[&str]) -> Outcome {
        let listing = self.listing();
        let known = listing
            .found
            .iter()
            .map(|f| f.manifest.exec.clone())
            .collect();
        let answers = self.setups.iter().filter_map(|s| match s {
            Arrange::FrontendCheck(id, answer) => Some((self.exe_of(id), *answer)),
            _ => None,
        });
        let tell = RecordingTell {
            silent: self.is(BenchMutation::SilentTell),
            notes: RefCell::default(),
        };
        let gate = FakeGate {
            fails: self.has(&Arrange::ServerFails),
            blind: self.is(BenchMutation::GateBlind),
            calls: Cell::new(0),
        };
        let mcp = FakeMcp {
            from_env: self.is(BenchMutation::ActorFromEnv),
            actors: RefCell::default(),
        };
        let manifests = FakeManifests(listing);
        let name = self.program.name().clone();
        let launcher = FakeLauncher {
            dir: PathBuf::from("/bench/bin"),
            known,
            answers: answers.collect(),
            check_ignored: self.is(BenchMutation::CheckIgnored),
            socket_dropped: self.is(BenchMutation::SocketDropped),
            fresh_dropped: self.is(BenchMutation::FreshDropped),
            words_to: self
                .is(BenchMutation::WordsTold)
                .then_some((&tell as &dyn crate::tell::Tell, &name)),
            checked: RefCell::default(),
            opened: RefCell::default(),
        };
        let decorated = VPrefixed(&*self.program, format!("v{}", self.program.version()));
        let program: &dyn Program = if self.is(BenchMutation::VersionV) {
            &decorated
        } else {
            &*self.program
        };
        let parts = Parts {
            program,
            gate: &gate,
            manifests: &manifests,
            preference: &NoPreference,
            launcher: &launcher,
            tell: &tell,
            hooks: &[],
            mcp: Some(&mcp),
            default_socket: Some(PathBuf::from(Self::SOCKET)),
        };
        let mut words: Vec<OsString> = words.iter().map(|w| OsString::from(*w)).collect();
        if self.is(BenchMutation::HelpOpens) && words.len() == 1 && words[0] == "--help" {
            words.clear();
        }
        let mut out = Vec::new();
        let mut exit = run(&parts, &words, &mut out);
        if self.is(BenchMutation::ExitSwallowed) {
            exit = Exit::DONE;
        }
        Outcome {
            exit,
            stdout: String::from_utf8_lossy(&out).into_owned(),
            told: tell.notes.take(),
            gate_calls: gate.calls.get(),
            checked: launcher.checked.take(),
            opened: launcher.opened.take(),
            mcp_actors: mcp.actors.take(),
        }
    }
}
