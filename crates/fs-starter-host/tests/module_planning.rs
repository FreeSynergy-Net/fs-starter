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
                    "cards-in-columns@1.4.0",
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

/// The two `rows@1.0.0` entries of planning (type `Lane` and type `Task`) are told apart by their TYPE: the source of the lanes gets
/// the lane rows (scope `lanes:assignee`), the source of the tasks the task rows. Read through the same `read_pages` the host uses.
#[tokio::test(flavor = "current_thread")]
async fn two_rows_entries_are_not_mixed_up_the_lane_source_gets_lanes_the_task_source_gets_tasks() {
    use fs_starter_host::sources::declared::parse_entries;
    use fs_starter_host::sources::entries::{Wanted, select};
    use fs_starter_host::sources::{Limits, pages::read_pages};

    let forms = include_str!("../../../tests/data/planning-main-7db66a33/catalog/forms.json");
    let entries = parse_entries(forms).unwrap();
    let rows: Vec<_> = entries
        .iter()
        .filter(|e| e.form.form.name == "rows")
        .map(|e| e.data_type.as_str())
        .collect();
    assert_eq!(rows, ["Lane", "Task"], "planning declares both entries");
    let want = |t: &str, s: &str| Wanted {
        data_type: t.into(),
        scope: s.into(),
    };
    let wires = select(
        &entries,
        &[want("Lane", "lanes:assignee"), want("Task", "")],
    );
    // the Task source also takes `cards-in-columns` (type Task): every entry of the type of a source
    assert_eq!(wires.len(), 3);
    let at = |t: &str| {
        wires
            .iter()
            .position(|w| w.form.form.name == "rows" && w.data_type == t)
            .unwrap()
    };
    let (lane, task) = (at("Lane"), at("Task"));
    assert_eq!(wires[lane].scope, "lanes:assignee");
    assert_eq!(wires[task].scope, "");

    let dir = tempfile::Builder::new().prefix("fp").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, _stop) = serve(dir.path()).await;
            let source = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let uids = board_with(&socket, &source.client, &["a", "b", "c"]).await;
            let got = read_pages(&source.client, "planning", &wires, &Limits::default())
                .await
                .unwrap();
            let keys = |at: usize| {
                got[at]
                    .rows
                    .iter()
                    .map(|r| r.key.clone())
                    .collect::<Vec<_>>()
            };
            // the task source: one row per card
            assert_eq!(got[task].declared.form.name, "rows");
            assert_eq!(keys(task).len(), uids.len(), "the task rows");
            // the lane source: the lanes of the axis, never the cards
            for k in keys(lane) {
                assert!(!uids.contains(&k), "a lane row carries a task: {k}");
            }
            assert_ne!(keys(lane), keys(task), "Lane and Task got the same rows");
        })
        .await;
}
