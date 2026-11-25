//! The watches the host places on one connection against the REAL planning service (in this process, behind a Unix socket): two scopes on
//! the SAME form key are two watches that do not mix, and a form the module cannot watch (`rows@1.0.0`) is said and left out.
//!
//! Pattern: Observer test (a counter per `Attach`; the service filters by scope) + Fixture (a service in a temporary folder).

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use fs_forms_run::ChangeCounter;
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::tasks_capnp::tasks;
use fs_modules_planning::Planning;
use fs_objects::{Store, TursoStore};
use fs_rpc::OnAttach;
use fs_tasks_core::TaskRepo;
use tokio::task::LocalSet;

use super::{Attach, Identity};

const KEY: &str = "cards-in-columns@1.4.0";

async fn serve(dir: &Path) -> std::path::PathBuf {
    let owner = fs_objects::testing::owner();
    let options = fs_objects::OpenOptions::default().with_owner(owner.clone());
    let store = TursoStore::open_with(dir.join("objects.db"), options)
        .await
        .unwrap();
    let store: Arc<dyn Store> = Arc::new(store);
    let repo = Arc::new(TaskRepo::new(
        store,
        "https://local.invalid/users/owner",
        owner,
    ));
    let socket = dir.join("planning.sock");
    let bound = fs_rpc::bind_socket(&socket).await.unwrap();
    tokio::task::spawn_local(async move {
        let _ =
            fs_rpc::serve_callers(bound, Planning::new(repo, |_| {}), std::future::pending()).await;
    });
    socket
}

/// A card on `board` made over a connection that is NOT the one under test (so its change is an event for the watchers).
async fn create(source: &form_source::Client, board: &str, title: &str) {
    let mut request = source.invoke_request();
    {
        let mut p = request.get();
        p.set_command("task.create");
        p.set_record("");
        let mut list = p.init_args(3);
        for (i, (path, text)) in (0..).zip([("title", title), ("board", board), ("column", "todo")])
        {
            let mut v = list.reborrow().get(i);
            v.set_path(path);
            v.set_text(text);
        }
    }
    let answer = request.send().promise.await.unwrap();
    assert!(
        !answer.get().unwrap().get_failed(),
        "task.create on {board}"
    );
}

fn attach(forms: &[(&str, &str)], counter: &ChangeCounter) -> Attach {
    Attach {
        module: "planning".into(),
        me: Identity {
            program: "tasks".into(),
            instance: "watcher".into(),
        },
        forms: forms
            .iter()
            .map(|(f, s)| ((*f).to_owned(), (*s).to_owned()))
            .collect(),
        counter: counter.clone(),
    }
}

async fn settle() {
    tokio::time::sleep(Duration::from_millis(400)).await;
}

#[tokio::test(flavor = "current_thread")]
async fn two_scopes_on_the_same_key_are_two_watches_that_do_not_mix() {
    let dir = tempfile::Builder::new().prefix("fw").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let socket = serve(dir.path()).await;
            let writer = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let tasks = fs_rpc::connect::<tasks::Client>(&socket).await.unwrap();
            let boards = tasks.client.boards_request().send().pipeline.get_boards();
            for name in ["work", "other"] {
                let mut b = boards.create_request();
                b.get().set_name(name);
                b.send().promise.await.unwrap();
            }
            // three watchers on three connections: only "work"; "" (every board) and "work" on the SAME key; a scope the module does not know
            let (only_work, both, none) = (
                ChangeCounter::new(),
                ChangeCounter::new(),
                ChangeCounter::new(),
            );
            let (mut held, mut conns) = (Vec::new(), Vec::new());
            for (a, c) in [
                (attach(&[(KEY, "work")], &only_work), &only_work),
                (attach(&[(KEY, ""), (KEY, "work")], &both), &both),
                (attach(&[(KEY, "no-such")], &none), &none),
            ] {
                let conn = fs_rpc::connect::<form_source::Client>(&socket)
                    .await
                    .unwrap();
                held.push(a.attach(conn.client.client.clone()).await.unwrap());
                conns.push(conn);
                assert_eq!(c.get(), 0);
            }
            create(&writer.client, "work", "one").await;
            settle().await;
            // a change on "work": the watcher of "work" once, the one with BOTH scopes twice (one per watch), the one of an unknown scope (left out, said) not at all
            assert_eq!((only_work.get(), both.get(), none.get()), (1, 2, 0));
            create(&writer.client, "work", "two").await;
            settle().await;
            // each watch counts for itself: no sharing of a counter, no merging of two watches on one key
            assert_eq!((only_work.get(), both.get(), none.get()), (2, 4, 0));
            // a card made on the board "other": planning holds a card for EVERY board without a label (`scope.rs holds`), so the watch of
            // "work" gets it too (observed 3/6/0); the test pins what was observed (5 of 5 runs): each watch counts alone
            // (both = 2 x only_work) and the unknown scope stays silent. A board with a label would separate; the fixture has none.
            create(&writer.client, "other", "three").await;
            settle().await;
            assert_eq!((only_work.get(), both.get(), none.get()), (3, 6, 0));
            drop((held, conns));
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_form_the_module_cannot_watch_is_left_out_and_the_other_watch_stays() {
    let dir = tempfile::Builder::new().prefix("fw").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let socket = serve(dir.path()).await;
            let writer = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let tasks = fs_rpc::connect::<tasks::Client>(&socket).await.unwrap();
            let boards = tasks.client.boards_request().send().pipeline.get_boards();
            let mut b = boards.create_request();
            b.get().set_name("work");
            b.send().promise.await.unwrap();
            // `rows@1.0.0` (the lane list) is not watchable in planning: the attach still succeeds and the card form is watched
            let counter = ChangeCounter::new();
            let a = attach(&[("rows@1.0.0", "lanes:assignee"), (KEY, "")], &counter);
            let conn = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let held = a.attach(conn.client.client.clone()).await.unwrap();
            create(&writer.client, "work", "one").await;
            settle().await;
            assert_eq!(counter.get(), 1);
            drop(held);
        })
        .await;
}
