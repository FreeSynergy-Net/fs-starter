//! The contract `module_v1` against the fake module as a PROCESS (Huelle around `FakeSource`), over the real RPC connection of the host:
//! the suite `fs_module::conformance` (13 rules of level 1), the decoding of a refusal as a problem `form-source-<kind>` (`WP4a`), and the
//! border modes of the process, which must keep their meaning next to the strict contract.
#![cfg(unix)]

mod support;
use fs_module::conformance::run_conformance;
use fs_rpc::Problem;
use support::module::{FakeFixture, connect, invoke, start_fake};
use tokio::task::LocalSet;

fn runtime() -> tempfile::TempDir {
    tempfile::Builder::new().prefix("fm").tempdir().unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn the_suite_passes_on_the_fake_process_over_the_rpc_connection() {
    let run = runtime();
    let (_child, socket) = start_fake(run.path(), "");
    LocalSet::new()
        .run_until(async {
            let connection = connect(&socket).await;
            let client = connection.client.clone();
            let report = run_conformance(&FakeFixture(client)).await;
            assert!(report.failures.is_empty(), "{:#?}", report.failures);
            assert!(report.skipped.is_empty(), "{:#?}", report.skipped);
            assert!(report.checked > 30, "{}", report.checked);
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_refusal_of_the_process_decodes_as_a_problem_with_its_kind_and_status() {
    let run = runtime();
    let (_child, socket) = start_fake(run.path(), "");
    LocalSet::new()
        .run_until(async {
            let connection = connect(&socket).await;
            let client = connection.client.clone();
            let mut request = client.page_request();
            request.get().set_form("no-such-form@1.0.0");
            let error = request.send().promise.await.map(|_| ()).unwrap_err();
            let problem = Problem::from_error(&error).expect("a problem, not a bare text");
            assert_eq!(problem.problem_type, "form-source-unknown-form");
            assert_eq!(problem.status, 404);
            // a form of the right name but a major the service does not have is the same refusal
            let mut request = client.page_request();
            request.get().set_form("cards-in-columns@9.0.0");
            let error = request.send().promise.await.map(|_| ()).unwrap_err();
            let problem = Problem::from_error(&error).expect("a problem");
            assert_eq!(problem.problem_type, "form-source-unknown-form");
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn invoke_is_no_longer_yes_to_everything() {
    let run = runtime();
    let (_child, socket) = start_fake(run.path(), "");
    LocalSet::new()
        .run_until(async {
            let connection = connect(&socket).await;
            let c = connection.client.clone();
            let (failed, kind, _) = invoke(&c, "card.fly", "c1", &[]).await.unwrap();
            assert!(failed && kind == "form-source-unknown-command", "{kind}");
            let (failed, kind, _) = invoke(&c, "card.move", "c1", &[]).await.unwrap();
            assert!(failed && kind == "form-source-bad-arguments", "{kind}");
            let (failed, kind, _) = invoke(&c, "card.move", "c3", &[("column", "todo")])
                .await
                .unwrap();
            assert!(failed && kind == "form-source-forbidden", "{kind}");
            let (failed, kind, record) = invoke(&c, "card.move", "c1", &[("column", "done")])
                .await
                .unwrap();
            assert!(
                !failed && kind.is_empty() && record == "c1",
                "{kind} {record}"
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn the_border_modes_still_break_their_one_rule() {
    // `real` says another provider, `level` another level: exactly the rule 1 of the suite turns red, nothing else changes
    for (mode, rule) in [("real", "1 "), ("level", "1 ")] {
        let run = runtime();
        let (_child, socket) = start_fake(run.path(), mode);
        LocalSet::new()
            .run_until(async {
                let connection = connect(&socket).await;
                let client = connection.client.clone();
                let report = run_conformance(&FakeFixture(client)).await;
                assert!(
                    report.failures.iter().any(|f| f.starts_with(rule)),
                    "{mode}: {:#?}",
                    report.failures
                );
            })
            .await;
    }
}
