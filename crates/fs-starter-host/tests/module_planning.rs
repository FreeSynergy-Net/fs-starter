//! The contract `module_v1` against the REAL planning service (fs-modules-planning, the commit the Cargo.toml names) behind a real Unix
//! socket in a temporary folder, over the same RPC connection the host uses: the suite `fs_module::conformance` and the refusal of
//! planning decoded as a problem `form-source-<kind>` (`WP4a`). Planning has no rights per person yet: only rule 11 "forbidden" is skipped.
#![cfg(unix)]

mod support;
use fs_interfaces::formsource_capnp::form_source;
use fs_module::conformance::run_conformance;
use fs_rpc::Problem;
use support::planning::{Planning3, board_with, serve};
use tokio::task::LocalSet;

#[tokio::test(flavor = "current_thread")]
async fn the_suite_passes_on_the_real_planning_service() {
    let dir = tempfile::Builder::new().prefix("fp").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop) = serve(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let other = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let uids = board_with(&socket, &source.client, &["a", "b", "c"]).await;
            let fixture = Planning3 {
                client: source.client.clone(),
                other: other.client.clone(),
                uids,
            };
            let report = run_conformance(&fixture).await;
            assert!(report.failures.is_empty(), "{:#?}", report.failures);
            assert!(
                // what the fixture of planning does not offer (no rights per person yet, no image field, no black-out, no ordered move or value fields in it)
                report.skipped.iter().all(|s| [
                    "11 forbidden",
                    "20 blob",
                    "L2-9 gone",
                    "21 place",
                    "22 standard"
                ]
                .iter()
                .any(|p| s.starts_with(p))),
                "{:#?}",
                report.skipped
            );
            assert!(report.checked > 30, "{}", report.checked);
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_refusal_of_planning_decodes_as_a_problem_of_its_kind() {
    let dir = tempfile::Builder::new().prefix("fp").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop) = serve(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            for (form, scope, kind) in [
                ("no-such-form@1.0.0", "", "form-source-unknown-form"),
                (
                    "cards-in-columns@1.3.0",
                    "no-such",
                    "form-source-unknown-scope",
                ),
            ] {
                let mut request = source.client.page_request();
                {
                    let mut p = request.get();
                    p.set_form(form);
                    p.set_scope(scope);
                }
                let error = request.send().promise.await.map(|_| ()).unwrap_err();
                let problem = Problem::from_error(&error).expect("a problem, not a bare text");
                assert_eq!(problem.problem_type, kind, "{form} {scope}");
                assert_eq!(problem.status, 404);
            }
        })
        .await;
}
