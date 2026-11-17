//! The suite `starter`: its id and its twelve rules.
//!
//! Pattern: Abstract factory (`StarterSuite` makes the rules) + generic rules `impl<F: StarterFixture + ?Sized> Rule<F>`.

use fs_conformance::{Rule, Suite, SuiteId};

use super::fixture::StarterFixture;
use super::{rules_open, rules_words};

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
