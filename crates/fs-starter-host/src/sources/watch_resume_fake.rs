//! The fake `FormSource` of the resume tests: tells `ended` and refuses on command, behind the real capnp interface (test only).
//!
//! Pattern: Fake (a service with a script).

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use fs_interfaces::formsource_capnp::{change_sink, form_source};
use fs_rpc::Problem;

/// One change of the fake: activity, program and instance of its origin.
pub(super) type Item = (u128, &'static str, &'static str);

#[derive(Default)]
pub(super) struct Shared {
    pub(super) sinks: RefCell<Vec<change_sink::Client>>,
    /// Newest first (what `changes` says).
    pub(super) items: RefCell<Vec<Item>>,
    /// The next this many `watch` calls are refused 503 (the feed of the module is gone).
    pub(super) refuse: Cell<u32>,
    /// `watch` and `changes` answer only after this long (a resume that takes time).
    pub(super) delay: Cell<Duration>,
}

pub(super) struct Fake(pub(super) Rc<Shared>);

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
        // the sink is known to the fake, the answer is late: the host is still inside `subscribe`
        tokio::time::sleep(self.0.delay.get()).await;
        r.get().set_subscription(capnp_rpc::new_client(Sub));
        Ok(())
    }

    async fn changes(
        self: Rc<Self>,
        p: form_source::ChangesParams,
        mut r: form_source::ChangesResults,
    ) -> Result<(), capnp::Error> {
        tokio::time::sleep(self.0.delay.get()).await;
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

pub(super) struct Sub;
impl fs_interfaces::common_capnp::subscription::Server for Sub {}

pub(super) fn fill(mut c: fs_interfaces::formsource_capnp::change_ref::Builder<'_>, item: Item) {
    let (activity, program, instance) = item;
    let mut u = c.reborrow().init_activity();
    u.set_hi(u64::try_from(activity >> 64).unwrap());
    u.set_lo(u64::try_from(activity & u128::from(u64::MAX)).unwrap());
    let mut o = c.init_origin();
    o.set_program(program);
    o.set_instance(instance);
}

/// Tell `change` through `sink` (as the service does).
pub(super) async fn tell(sink: &change_sink::Client, item: Item) {
    let mut request = sink.changed_request();
    fill(request.get().init_change(), item);
    request.send().await.unwrap();
}

pub(super) async fn end(sink: &change_sink::Client) {
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

pub(super) async fn until(what: &str, mut done: impl FnMut() -> bool) {
    for _ in 0..200 {
        if done() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("never happened: {what}");
}
