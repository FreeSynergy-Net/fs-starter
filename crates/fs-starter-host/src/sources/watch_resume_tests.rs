//! What the host does when the service ENDS a watch (410): Lagged (a new watch is heard, what was missed is read through `changes`, nothing
//! lost, nothing twice), Closed (the new watch is refused 503: the host asks again until one stands) and the stale `ended` of an old sink.
//! The service is a fake of `FormSource` that tells `ended` and refuses on command, behind the real capnp interface.
//!
//! Pattern: Fake (a service with a script) + Observer test (the counter of the window).

use std::rc::Rc;
use std::time::Duration;

use fs_forms_run::ChangeCounter;
use fs_interfaces::formsource_capnp::{change_sink, form_source};
use tokio::task::LocalSet;

use super::Watch;
use crate::sources::watch::Identity;
use crate::sources::watch_resume_fake::{Fake, Shared, end, tell, until};

struct Setup {
    shared: Rc<Shared>,
    counter: ChangeCounter,
    watch: Rc<Watch>,
}

async fn setup() -> Setup {
    let shared = Rc::new(Shared::default());
    let source: form_source::Client = capnp_rpc::new_client(Fake(Rc::clone(&shared)));
    let counter = ChangeCounter::new();
    let me = Identity {
        program: "tasks".into(),
        instance: "me".into(),
    };
    let watch = Watch::new(
        "planning",
        &me,
        &("cards@1.4.0".into(), String::new()),
        &counter,
        &source,
    );
    watch.subscribe().await.unwrap();
    watch.baseline().await;
    Setup {
        shared,
        counter,
        watch,
    }
}

fn sink(s: &Setup, n: usize) -> change_sink::Client {
    s.shared.sinks.borrow()[n].clone()
}

#[tokio::test(flavor = "current_thread")]
async fn lagged_subscribes_again_and_reads_what_was_missed_once() {
    LocalSet::new()
        .run_until(async {
            let s = setup().await;
            tell(&sink(&s, 0), (10, "tasks", "other")).await;
            assert_eq!(s.counter.get(), 1);
            // 11 comes while the feed is cut (the watch never heard it), 12 is the own one, `changes` says the own program's
            s.shared.items.borrow_mut().extend([
                (12, "tasks", "me"),
                (11, "tasks", "other"),
                (10, "tasks", "other"),
            ]);
            end(&sink(&s, 0)).await;
            until("a new watch", || s.shared.sinks.borrow().len() == 2).await;
            until("caught up and read again", || s.counter.get() == 3).await;
            // 10 was seen live (not counted again), 11 is caught up, 12 is the own connection's (left out), +1 for the page read again
            tokio::time::sleep(Duration::from_millis(50)).await;
            assert_eq!(s.counter.get(), 3);
            assert_eq!(s.watch.status(), 0);
            // the same change told by the NEW watch as well is no duplicate
            tell(&sink(&s, 1), (11, "tasks", "other")).await;
            tell(&sink(&s, 1), (14, "planning", "x")).await;
            assert_eq!(s.counter.get(), 4);
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn closed_is_refused_503_until_a_new_watch_stands_never_410() {
    LocalSet::new()
        .run_until(async {
            let s = setup().await;
            s.shared.refuse.set(2);
            end(&sink(&s, 0)).await;
            until("503 while the feed is gone", || s.watch.status() == 503).await;
            assert!(s.shared.sinks.borrow().len() == 1, "no watch stands yet");
            until("a new watch", || s.shared.sinks.borrow().len() == 2).await;
            assert_eq!(s.shared.refuse.get(), 0, "asked again after each 503");
            until("the state is live", || s.watch.status() == 0).await;
            // the page is read again once the watch stands, then the new watch counts
            until("the page is read again", || s.counter.get() == 1).await;
            tell(&sink(&s, 1), (20, "planning", "x")).await;
            assert_eq!(s.counter.get(), 2);
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn the_ended_of_an_old_sink_and_a_repeated_one_start_nothing() {
    LocalSet::new()
        .run_until(async {
            let s = setup().await;
            let old = sink(&s, 0);
            end(&old).await;
            end(&old).await; // repeated while resuming: one resume only
            until("a new watch", || s.shared.sinks.borrow().len() == 2).await;
            tokio::time::sleep(Duration::from_millis(100)).await;
            end(&old).await; // stale: the current sink is the second
            tokio::time::sleep(Duration::from_millis(100)).await;
            assert_eq!(s.shared.sinks.borrow().len(), 2);
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_change_of_another_program_only_in_the_store_makes_the_window_read_again() {
    LocalSet::new()
        .run_until(async {
            let s = setup().await;
            // the miss is of ("planning", "x"): `changes` (own program) does not say it, only the page read again can
            s.shared.items.borrow_mut().push((30, "tasks", "me"));
            end(&sink(&s, 0)).await;
            until("a new watch", || s.shared.sinks.borrow().len() == 2).await;
            until("the counter moved", || s.counter.get() == 1).await;
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn first_attach_refused_503_reads_again_when_a_watch_stands() {
    LocalSet::new()
        .run_until(async {
            let s = setup().await;
            s.shared.refuse.set(1);
            let again = Watch::new(
                "planning",
                &Identity {
                    program: "tasks".into(),
                    instance: "me".into(),
                },
                &("cards@1.4.0".into(), String::new()),
                &s.counter,
                &capnp_rpc::new_client(Fake(Rc::clone(&s.shared))),
            );
            assert_eq!(again.subscribe().await.unwrap_err().0, 503);
            again.start_resume();
            until("a watch stands", || s.shared.sinks.borrow().len() == 2).await;
            until("the page is read again", || s.counter.get() == 1).await;
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn an_ended_during_the_catch_up_is_resumed_again_never_dropped() {
    LocalSet::new()
        .run_until(async {
            let s = setup().await;
            s.shared.delay.set(Duration::from_millis(150));
            end(&sink(&s, 0)).await;
            until("the second watch stands", || {
                s.shared.sinks.borrow().len() == 2
            })
            .await;
            // the resume still reads `changes` (slow): the new sink ends too
            end(&sink(&s, 1)).await;
            until("a third watch", || s.shared.sinks.borrow().len() == 3).await;
            until("read again after the last", || s.counter.get() == 2).await;
            tell(&sink(&s, 2), (40, "planning", "x")).await;
            assert_eq!(s.counter.get(), 3, "the third sink is the live one");
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn an_ended_during_the_subscribe_is_resumed_again_never_dropped() {
    LocalSet::new()
        .run_until(async {
            let s = setup().await;
            s.shared.delay.set(Duration::from_millis(150));
            end(&sink(&s, 0)).await;
            // the fake knows the new sink while its `watch` answers late: the sink ends inside `subscribe`
            until("the second watch stands", || {
                s.shared.sinks.borrow().len() == 2
            })
            .await;
            end(&sink(&s, 1)).await;
            until("a third watch", || s.shared.sinks.borrow().len() == 3).await;
            until("the state read again", || s.counter.get() >= 2).await;
        })
        .await;
}
