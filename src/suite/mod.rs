//! The conformance suite `starter` (feature `conformance`; review condition C1: the suite lives in the crate it checks).
//!
//! What it checks is a program's WIRING of the starter: its `Program`, its parts and its entry, run through a fixture
//! ([`StarterFixture`]; the standard one is [`Bench`] with fakes and the program's real `Program`). Every rule has at least
//! one mutant of the bench ([`bench_mutants`]) that makes it fail (`fs_conformance::prove`).
//!
//! fs-conformance is never in the normal graph of a shipped program: this module exists only with the feature
//! `conformance`, which is in no `default` (fs-conformance review condition A4).
//!
//! Pattern: Abstract factory (`StarterSuite` makes the rules) + generic rules `impl<F: StarterFixture + ?Sized> Rule<F>`
//! (fs-conformance review condition A1).

mod bench;
mod check;
mod codes;
mod fakes;
mod fixture;
mod mutants;
mod rules_open;
mod rules_words;

use fs_conformance::{Rule, Suite, SuiteId};

pub use bench::{Bench, BenchMutation};
pub use codes::{SuiteCode, keys as suite_keys};
pub use fixture::{Arrange, Outcome, StarterFixture};
pub use mutants::{BenchMutant, bench_mutants};

/// The suite `starter`.
#[derive(Clone, Copy, Debug, Default)]
pub struct StarterSuite;

impl<F: StarterFixture + ?Sized> Suite<F> for StarterSuite {
    fn id(&self) -> SuiteId {
        SuiteId::new("starter")
    }

    fn rules(&self) -> Vec<Box<dyn Rule<F>>> {
        vec![
            Box::new(rules_words::Version),
            Box::new(rules_words::Help),
            Box::new(rules_words::Usage),
            Box::new(rules_open::OpenStandard),
            Box::new(rules_open::ContractFallback),
            Box::new(rules_open::FrontendMissing),
            Box::new(rules_open::ServerProblem),
            Box::new(rules_open::LocalManifest),
            Box::new(rules_words::McpNeedsActor),
            Box::new(rules_words::McpActor),
            Box::new(rules_open::ResumedHidden),
            Box::new(rules_open::FreshPassed),
        ]
    }
}
