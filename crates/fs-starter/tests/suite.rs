//! K16 (suite part), K17, K18, K29 of ENTWURF section 5: the suite `starter` over the bench and its mutants.
#![cfg(feature = "conformance")]
mod support;

use std::collections::HashSet;

use fs_conformance::{ErrorCode, Options, RuleId, Suite, message_key, prove, run};
use fs_starter::suite::{Bench, Outcome, StarterFixture, StarterSuite, SuiteCode, bench_mutants};
use support::Demo;

fn bench() -> Bench {
    Bench::new(Box::new(Demo::default()))
}

/// K16 (suite part): the suite's codes.
#[test]
fn k16_suite_codes() {
    assert_eq!(SuiteCode::ALL.len(), 10);
    let names: HashSet<_> = SuiteCode::ALL.iter().map(|c| c.as_str()).collect();
    assert_eq!(names.len(), SuiteCode::ALL.len());
    for c in SuiteCode::ALL {
        let n = c.as_str();
        assert!(
            n.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
                && !n.starts_with('-')
                && !n.ends_with('-'),
            "{n}"
        );
        assert_eq!(message_key(*c), format!("starter-suite-{n}"));
    }
}

/// K17: the bench with the demo program is conformant: thirteen rules checked, none skipped.
#[tokio::test]
async fn k17_bench_conformant() {
    let mut bench = bench();
    let report = run(&StarterSuite, &mut bench, &Options::default()).await;
    assert!(report.is_conformant(), "{:?}", report.failures);
    assert_eq!(report.checked, 13);
    assert_eq!(report.skipped.len(), 0);
}

/// K18: every one of the thirteen bench mutants is caught, and every rule is named by one.
#[tokio::test]
async fn k18_prove() {
    let fresh = || Box::new(bench());
    let report = prove(&StarterSuite, &fresh, &bench_mutants(), &Options::default()).await;
    assert!(report.is_proven(), "{:?}", report.failures);
    assert_eq!(report.caught.len(), 13);
}

/// K29: the suite also runs on a `dyn` fixture.
#[tokio::test]
async fn k29_dyn_fixture() {
    let mut bench = bench();
    let dynamic: &mut dyn StarterFixture = &mut bench;
    let report = run(&StarterSuite, dynamic, &Options::default()).await;
    assert!(report.is_conformant(), "{:?}", report.failures);
    let as_dyn: &dyn Suite<Bench> = &StarterSuite;
    let ids: Vec<RuleId> = as_dyn.rules().iter().map(|r| r.id()).collect();
    assert_eq!(ids.len(), 13);
    let _: Option<Outcome> = None; // the fixture's outcome type is public
}
