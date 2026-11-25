//! S3-8: the held connection at level 2 over the real fake PROCESS (the strict `FakeSource` of fs-module with level 2 behind a socket;
//! one connection per window, the cards and the log shared). Two windows of this process are two connections (`introduce` with
//! their own instance): a change of window A reaches window B as an event (its counter grows), A's own counter does not (only the
//! own connection is left out); `app.undo`/`app.redo` go over `revert`; a change of B on the same card makes A's undo a conflict with
//! its code, and the card is not overwritten.
//!
//! Mutants: the filter by account (not by connection) = case 2 red (B hears nothing); `revert` without the history put back = case 3
//! red (the undo is lost after the conflict); no `introduce` = case 3 red (the conflict names no program).
#![cfg(unix)]

mod support;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use fs_components_kanban::{Answer, Reason};
use fs_forms_run::{Invocation, Records};
use fs_starter_host::sources::HeldRecords;
use support::inproc::{commands_of, setup};

const FORMS: &str = r#"[{"name":"cards-in-columns","version":"1.0","type":"Card","map":{"label":"title"},"fields":{"title":"text"}}]"#;

fn window(s: &support::inproc::InProc) -> HeldRecords {
    s.sources
        .connect_held("cards@>=0.1.0, <0.2.0", |_| {})
        .expect("read and hold at level 2")
}

fn mv(w: &HeldRecords, card: &str, column: &str) {
    let call = Invocation {
        form: "cards-in-columns".into(),
        command: "card.move".into(),
        record: Some(card.into()),
        args: vec![("column".into(), column.into())],
    };
    w.writer()
        .expect("a writer")
        .invoke(&call)
        .expect("accepted");
}

fn waits_for(what: &str, ok: impl Fn() -> bool) {
    waits_within(what, Duration::from_secs(5), ok);
}

fn waits_within(what: &str, limit: Duration, ok: impl Fn() -> bool) {
    let began = Instant::now();
    while !ok() {
        assert!(began.elapsed() < limit, "{what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// One service per test binary: the tests take turns, so a change of one is never an event of another.
static TURN: Mutex<()> = Mutex::new(());

fn module() -> (MutexGuard<'static, ()>, support::inproc::InProc) {
    let turn = TURN.lock().unwrap_or_else(PoisonError::into_inner);
    (
        turn,
        setup("cards", FORMS, &commands_of("cards", "card.move"), 5000),
    )
}

#[test]
fn a_write_is_in_the_history_and_app_undo_and_redo_go_over_revert() {
    let (_turn, s) = module();
    let w = window(&s);
    assert_eq!((w.history().undoable, w.history().redoable), (0, 0));
    mv(&w, "c1", "done");
    assert_eq!((w.history().undoable, w.history().redoable), (1, 0));
    assert_eq!(w.answer_app("app.undo"), Some(Answer::Accepted));
    assert_eq!((w.history().undoable, w.history().redoable), (0, 1));
    assert_eq!(w.answer_app("app.redo"), Some(Answer::Accepted));
    assert_eq!((w.history().undoable, w.history().redoable), (1, 0));
    // no step left: refused with its code, not silent, not unsupported
    assert_eq!(
        w.answer_app("app.redo"),
        Some(Answer::Refused(Reason::new("host-notice-undo-empty", "")))
    );
    assert_eq!(w.answer_app("app.quit"), None);
}

#[test]
fn a_change_of_window_a_reaches_window_b_and_only_the_own_connection_is_left_out() {
    let (_turn, s) = module();
    let (a, b) = (window(&s), window(&s));
    let (a0, b0) = (a.changed(), b.changed());
    mv(&a, "c1", "done");
    waits_for("window B heard the change of window A", || b.changed() > b0);
    // A's own change comes back to A as an event too, but it is the own connection (program AND instance): left out
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(a.changed(), a0, "the own change is not an event");
    // and the same the other way round: B's change reaches A, and nothing is left out because of "the same program"
    mv(&b, "c2", "done");
    waits_for("window A heard the change of window B", || a.changed() > a0);
}

#[test]
fn an_undo_after_another_window_changed_the_card_is_a_conflict_and_keeps_the_step() {
    let (_turn, s) = module();
    let (a, b) = (window(&s), window(&s));
    mv(&a, "c1", "done");
    mv(&b, "c1", "todo");
    assert_eq!(
        a.answer_app("app.undo"),
        Some(Answer::Refused(Reason::new(
            "host-notice-undo-conflict",
            ""
        )))
    );
    // nothing overwritten, nothing lost: the step is still there
    assert_eq!((a.history().undoable, a.history().redoable), (1, 0));
    // B can still take back its own change
    assert_eq!(b.answer_app("app.undo"), Some(Answer::Accepted));
}

fn up(s: &support::inproc::InProc) -> bool {
    std::os::unix::net::UnixStream::connect(&s.socket).is_ok()
}

/// The service is down: a stop (if it runs) and the wait until the socket does not answer.
fn down(s: &support::inproc::InProc) {
    if up(s) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let local = tokio::task::LocalSet::new();
        let _ = local.block_on(&runtime, fs_rpc::stop_server(&s.socket));
    }
    waits_for("the service is down", || !up(s));
}

fn one_time(s: &support::inproc::InProc) -> HeldRecords {
    s.sources
        .clone()
        .ending_what_it_started()
        .connect_held("cards@>=0.1.0, <0.2.0", |_| {})
        .expect("read and hold at level 2")
}

/// Mutant: `end_started` gives no session (the old `Held::None`) = `answer_app` is `Refused(unsupported)`, the first assert is red;
/// no guard = the socket still answers after the drop, the second is red; a guard that ends a service that was up = the third is red.
#[test]
fn a_one_time_run_has_a_history_and_ends_the_service_it_started_itself() {
    let (_turn, s) = module();
    down(&s);
    let w = one_time(&s);
    assert!(up(&s), "the one-time run holds the service it started");
    mv(&w, "c1", "done");
    assert_eq!((w.history().undoable, w.history().redoable), (1, 0));
    assert_eq!(w.answer_app("app.undo"), Some(Answer::Accepted));
    assert_eq!((w.history().undoable, w.history().redoable), (0, 1));
    drop(w);
    // well below the idle time of the fake (5000 ms): only the guard can have ended it, not the idle end
    waits_within(
        "the started service ended with the drop",
        Duration::from_secs(2),
        || !up(&s),
    );
}

/// The SAME process must go on: the log of the fake lives in its memory, so an undo over the older window only works if the service
/// was never ended (a restart by the hold of `other` would give an empty log; `up()` alone cannot tell, the hold starts it again).
/// Mutant: the guard ends a service that was up before = the undo is refused, red.
#[test]
fn a_one_time_run_leaves_a_service_that_was_up_before() {
    let (_turn, s) = module();
    down(&s);
    let other = window(&s);
    mv(&other, "c1", "done");
    let w = one_time(&s);
    drop(w);
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(other.answer_app("app.undo"), Some(Answer::Accepted));
}
