//! The bench mutants: one wrong wiring each, and the rules that must catch it. `fs_conformance::prove` runs each on a fresh
//! bench; every rule of the suite is named by at least one (no `rule-without-mutant`).
//!
//! Pattern: Strategy (`Mutant<Bench>`; a switch on the fake, the tasks form `mutation.rs:8`).

use fs_conformance::{Applied, Mutant, MutantId, RuleId};

use super::bench::{Bench, BenchMutation};

/// A switch of the bench with its id and the rules it must break.
#[derive(Clone, Copy, Debug)]
pub struct BenchMutant {
    id: &'static str,
    mutation: BenchMutation,
    must_fail: &'static [&'static str],
}

impl Mutant<Bench> for BenchMutant {
    fn id(&self) -> MutantId {
        MutantId::new(self.id)
    }

    fn must_fail(&self) -> Vec<RuleId> {
        self.must_fail.iter().copied().map(RuleId::new).collect()
    }

    fn apply(&self, bench: &mut Bench) -> Applied {
        bench.set_mutation(self.mutation);
        Applied::Yes
    }
}

/// The twelve mutants (ids: `starter-bench.<name>`, well formed and unique: fs-conformance review condition A3).
#[must_use]
pub fn bench_mutants() -> Vec<Box<dyn Mutant<Bench>>> {
    use BenchMutation as M;
    let all: [BenchMutant; 12] = [
        BenchMutant {
            id: "starter-bench.version-v",
            mutation: M::VersionV,
            must_fail: &["starter.version"],
        },
        BenchMutant {
            id: "starter-bench.help-opens",
            mutation: M::HelpOpens,
            must_fail: &["starter.help"],
        },
        BenchMutant {
            id: "starter-bench.exit-swallowed",
            mutation: M::ExitSwallowed,
            must_fail: &[
                "starter.usage",
                "starter.frontend-missing",
                "starter.mcp-needs-actor",
            ],
        },
        BenchMutant {
            id: "starter-bench.socket-dropped",
            mutation: M::SocketDropped,
            must_fail: &["starter.open-standard"],
        },
        BenchMutant {
            id: "starter-bench.check-ignored",
            mutation: M::CheckIgnored,
            must_fail: &["starter.contract-fallback"],
        },
        BenchMutant {
            id: "starter-bench.silent-tell",
            mutation: M::SilentTell,
            must_fail: &[
                "starter.usage",
                "starter.contract-fallback",
                "starter.frontend-missing",
                "starter.server-problem",
                "starter.local-manifest",
                "starter.mcp-needs-actor",
            ],
        },
        BenchMutant {
            id: "starter-bench.gate-blind",
            mutation: M::GateBlind,
            must_fail: &["starter.server-problem"],
        },
        BenchMutant {
            id: "starter-bench.manifest-invents",
            mutation: M::ManifestInvents,
            must_fail: &["starter.frontend-missing"],
        },
        BenchMutant {
            id: "starter-bench.trust-claimed",
            mutation: M::TrustClaimed,
            must_fail: &["starter.local-manifest"],
        },
        BenchMutant {
            id: "starter-bench.actor-from-env",
            mutation: M::ActorFromEnv,
            must_fail: &["starter.mcp-actor"],
        },
        BenchMutant {
            id: "starter-bench.words-told",
            mutation: M::WordsTold,
            must_fail: &["starter.resumed-hidden"],
        },
        BenchMutant {
            id: "starter-bench.fresh-dropped",
            mutation: M::FreshDropped,
            must_fail: &["starter.fresh-passed"],
        },
    ];
    all.into_iter()
        .map(|m| Box::new(m) as Box<dyn Mutant<Bench>>)
        .collect()
}
