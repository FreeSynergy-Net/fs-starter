//! What the host does when the service ENDS a watch (410): Lagged (a new watch is heard, what was missed is read through `changes`, nothing
//! lost, nothing twice), Closed (the new watch is refused 503: the host asks again until one stands) and the stale `ended` of an old sink.
//! The service is a fake of `FormSource` that tells `ended` and refuses on command, behind the real capnp interface.
//!
//! Pattern: Fake (a service with a script) + Observer test (the counter of the window).

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use fs_forms_run::ChangeCounter;
use fs_interfaces::formsource_capnp::{change_sink, form_source};
use fs_rpc::Problem;
use tokio::task::LocalSet;

use super::Watch;
use crate::sources::watch::Identity;

/// One change of the fake: activity, program and instance of its origin.
type Item = (u128, &'static str, &'static str);

#[derive(Default)]
struct Shared {
    sinks: RefCell<Vec<change_sink::Client>>,
    /// Newest first (what `changes` says).
    items: RefCell<Vec<Item>>,
    /// The next this many `watch` calls are refused 503 (the feed of the module is gone).
    refuse: Cell<u32>,
}

struct Fake(Rc<Shared>);

#[allow(clippy::unused_async_trait_impl)] // the generated trait wants async methods
impl form_source::Server for Fake {
    async fn watch(
        self: Rc<Self>,
        p: form_source::WatchParams,
        mut r: form_source::WatchResults,
    ) -> Result<(), capnp::Error> {
        if self.0.refuse.get() > 0 {
            self.0.refuse.set(self.0.refuse.get() - 1);
            return Err(
                Problem::new("form-source-unavailable", "gone", "the feed is gone", 503)
                    .into_error(),
            );
        }
        self.0.sinks.borrow_mut().push(p.get()?.get_sink()?);
        r.get().set_subscription(capnp_rpc::new_client(Sub));
        Ok(())
    }

    async fn changes(
        self: Rc<Self>,
        p: form_source::ChangesParams,
        mut r: form_source::ChangesResults,
    ) -> Result<(), capnp::Error> {
        let p = p.get()?;
        let skip: usize = std::str::from_utf8(p.get_cursor()?.get_opaque()?)
            .ok()
            .and_then(|t| t.parse().ok())
            .unwrap_or(0);
        let limit = p.get_limit() as usize;
        let items = self.0.items.borrow();
        let end = items.len().min(skip + limit);
        let mut out = r.get();
        let mut list = out
            .reborrow()
            .init_items(u32::try_from(end - skip).unwrap());
        for (i, (activity, program, instance)) in (0u32..).zip(&items[skip..end]) {
            fill(list.reborrow().get(i), (*activity, program, instance));
        }
        if end < items.len() {
            out.init_next().set_opaque(end.to_string().as_bytes());
        }
        Ok(())
    }
}

struct Sub;
impl fs_interfaces::common_capnp::subscription::Server for Sub {}

fn fill(mut c: fs_interfaces::formsource_capnp::change_ref::Builder<'_>, item: Item) {
    let (activity, program, instance) = item;
    let mut u = c.reborrow().init_activity();
    u.set_hi(u64::try_from(activity >> 64).unwrap());
    u.set_lo(u64::try_from(activity & u128::from(u64::MAX)).unwrap());
    let mut o = c.init_origin();
    o.set_program(program);
    o.set_instance(instance);
}

/// Tell `change` through `sink` (as the service does).
async fn tell(sink: &change_sink::Client, item: Item) {
    let mut request = sink.changed_request();
    fill(request.get().init_change(), item);
    request.send().await.unwrap();
}

async fn end(sink: &change_sink::Client) {
    let mut request = sink.ended_request();
    {
        let mut p = request.get().init_problem();
        p.set_type("https://freesynergy.net/problems/feed-ended");
        p.set_title("The feed ended");
        p.set_detail("test");
        p.set_status(410);
    }
    request.send().promise.await.unwrap();
}

async fn until(what: &str, mut done: impl FnMut() -> bool) {
    for _ in 0..200 {
        if done() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("never happened: {what}");
}

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
            until("caught up", || s.counter.get() == 2).await;
            // 10 was seen live (not counted again), 11 is caught up, 12 is the own connection's (left out)
            tokio::time::sleep(Duration::from_millis(50)).await;
            assert_eq!(s.counter.get(), 2);
            assert_eq!(s.watch.status(), 0);
            // the same change told by the NEW watch as well is no duplicate
            tell(&sink(&s, 1), (11, "tasks", "other")).await;
            tell(&sink(&s, 1), (14, "planning", "x")).await;
            assert_eq!(s.counter.get(), 3);
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
            // the new watch counts
            tell(&sink(&s, 1), (20, "planning", "x")).await;
            assert_eq!(s.counter.get(), 1);
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
