//! One watch of a window (`form` in `scope`) and what it does when the service ENDS it (contract `ChangeSink.ended`, fs-interfaces
//! `formsource.capnp` and `common.capnp`; fs-rpc 63e3a486 `WatchHub`): the last call of a feed that ended without the consumer dropping
//! its `Subscription`, problem status 410 (`feed-ended`). Changes may have been missed. The contract's way on: subscribe AGAIN, then read
//! the state (`changes`), in that order.
//!
//! - Lagged (the consumer fell behind, or the feed of the module lost changes): a new `watch` is heard again; the changes after the last
//!   one seen are read through `changes` and counted ONCE (no duplicate with what the new watch tells meanwhile).
//! - Closed (the feed of the module is gone): a new `watch` is refused with 503; the host asks again with growing waits until one
//!   stands, and says `unavailable status=503` meanwhile (never 410 to the window: 410 is the end of the old feed, not the state of the data).
//!
//! After EVERY successful subscribe (after Lagged, after Closed, at the first attach with 503) the counter is bumped once: the contract
//! says "subscribe again and then read the current state", and `changes` knows only the OWN program's changes, so what other programs
//! changed meanwhile is read only by the window reading its page again. An `ended` that comes while a resume runs is kept
//! (`again`) and the resume runs once more; it is never dropped.
//!
//! The host decides on the status of the answer to the new `watch`, never on the wording of the reason (that is a text for people).
//!
//! Pattern: Observer - the `ChangeSink` of the module tells the window's watch; Deviation: after its end the observer subscribes AGAIN
//! (the window's side re-registers itself) + Memento (the watermark of what was seen).

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

use fs_forms_run::ChangeCounter;
use fs_interfaces::common_capnp::subscription;
use fs_interfaces::formsource_capnp::form_source;
use fs_rpc::{Backoff, Problem};

use super::reply::{Change, change_of};
use super::watch::{Identity, is_mine, watch_line};
use super::watch_seen::Seen;
use super::watch_sink::Sink;

/// Status of `ended` (a feed that ended).
pub(super) const ENDED: u16 = 410;
/// Status of a `watch` the service refuses because its feed is gone.
pub(super) const UNAVAILABLE: u16 = 503;
/// One page of the catch-up, and the most pages read (a longer gap is said, not read to the end).
const PAGE: u32 = 50;
const PAGES: usize = 20;
/// The waits before asking again after a 503 (`fs_rpc::Backoff`: the last step repeats, it is the longest).
const WAITS: [Duration; 7] = [
    Duration::from_millis(100),
    Duration::from_millis(200),
    Duration::from_millis(400),
    Duration::from_millis(800),
    Duration::from_millis(1600),
    Duration::from_millis(3200),
    Duration::from_secs(5),
];

/// The watch of one form in one scope on one connection; the window holds it (dropping it drops the subscription).
pub(super) struct Watch {
    module: String,
    me: Identity,
    form: String,
    scope: String,
    counter: ChangeCounter,
    source: form_source::Client,
    seen: RefCell<Seen>,
    subscription: RefCell<Option<subscription::Client>>,
    /// Which sink is the current one: the `ended` of an older sink is stale and ignored.
    generation: Cell<u64>,
    /// A resume runs (one only).
    resuming: Cell<bool>,
    /// An `ended` came while the resume ran: the resume runs once more (never dropped).
    again: Cell<bool>,
    /// The newest generation whose end was taken (a repeated `ended` of the same sink is one end).
    ended_seen: Cell<u64>,
    /// 0: a watch stands (or the connection is gone and the hold attaches anew); else the status of the last refusal (503: the feed is gone).
    status: Cell<u16>,
}

impl Watch {
    pub(super) fn new(
        module: &str,
        me: &Identity,
        (form, scope): &(String, String),
        counter: &ChangeCounter,
        source: &form_source::Client,
    ) -> Rc<Self> {
        Rc::new(Self {
            module: module.to_owned(),
            me: me.clone(),
            form: form.clone(),
            scope: scope.clone(),
            counter: counter.clone(),
            source: source.clone(),
            seen: RefCell::default(),
            subscription: RefCell::new(None),
            generation: Cell::new(0),
            resuming: Cell::new(false),
            again: Cell::new(false),
            ended_seen: Cell::new(0),
            status: Cell::new(0),
        })
    }

    /// 0: a watch stands; else the status of the last refusal ([`UNAVAILABLE`]: none stands and the host asks again).
    #[cfg(all(test, unix))] // only the unix tests read it (the tests module is unix-only)
    pub(super) fn status(&self) -> u16 {
        self.status.get()
    }

    /// Say what was seen: count a change of another connection once, whoever tells it first (the watch or the catch-up).
    pub(super) fn take(&self, change: &Change, how: &str) {
        if !self.seen.borrow_mut().first(change.activity) {
            return;
        }
        if !is_mine(change, &self.me) {
            self.counter.bump();
            eprintln!("{}{how}", watch_line(&self.module, change));
        }
    }

    /// `watch` on the connection, with a new sink. Replaces (and so drops) the old subscription.
    ///
    /// # Errors
    /// The status of the refusal (503 when the feed is gone; 0 when the answer carried no problem) and the text.
    pub(super) async fn subscribe(self: &Rc<Self>) -> Result<(), (u16, String)> {
        let generation = self.generation.get() + 1;
        self.generation.set(generation);
        let mut request = self.source.watch_request();
        {
            let mut p = request.get();
            p.set_form(self.form.as_str());
            p.set_scope(self.scope.as_str());
            p.set_sink(capnp_rpc::new_client(Sink {
                watch: Rc::downgrade(self),
                generation,
            }));
        }
        let failed = |e: &capnp::Error| {
            (
                Problem::from_error(e).map_or(0, |p| p.status),
                e.to_string(),
            )
        };
        let answer = request.send().promise.await.map_err(|e| failed(&e))?;
        let held = answer
            .get()
            .and_then(form_source::watch_results::Reader::get_subscription)
            .map_err(|e| failed(&e))?;
        *self.subscription.borrow_mut() = Some(held);
        self.status.set(0);
        Ok(())
    }

    /// The newest change of the own program as the starting point of the watermark (nothing is counted). Without one (a service that
    /// cannot say) a later end cannot be caught up and says so.
    pub(super) async fn baseline(&self) {
        if let Ok(page) = self.read_page(&[], 1).await {
            let mut seen = self.seen.borrow_mut();
            if let Some(c) = page.0.first() {
                seen.first(c.activity);
            } else if seen.high.is_none() {
                seen.high = Some(0);
            }
        }
    }

    /// One page of `changes` (newest first) and the cursor of the next (empty: the end).
    async fn read_page(&self, cursor: &[u8], limit: u32) -> capnp::Result<(Vec<Change>, Vec<u8>)> {
        let mut request = self.source.changes_request();
        {
            let mut p = request.get();
            p.set_scope(self.scope.as_str());
            p.set_run("");
            p.set_limit(limit);
            p.init_cursor().set_opaque(cursor);
        }
        let answer = request.send().promise.await?;
        let out = answer.get()?;
        let mut items = Vec::new();
        for c in out.get_items()? {
            items.push(change_of(c)?);
        }
        Ok((items, out.get_next()?.get_opaque()?.to_vec()))
    }

    /// Read what came after `floor` (the watermark when the feed ended), newest first, and count it once.
    async fn catch_up(&self, floor: Option<u128>) {
        let Some(floor) = floor else {
            eprintln!(
                "fs-starter: watch module={} form={} caught-up=no-baseline",
                self.module, self.form
            );
            return;
        };
        let (mut cursor, mut read, mut reached) = (Vec::new(), 0usize, false);
        'pages: for _ in 0..PAGES {
            let Ok((items, next)) = self.read_page(&cursor, PAGE).await else {
                eprintln!(
                    "fs-starter: watch module={} form={} caught-up=failed",
                    self.module, self.form
                );
                return;
            };
            for change in &items {
                if change.activity <= floor {
                    reached = true;
                    break 'pages;
                }
                read += 1;
                self.take(change, " caught-up");
            }
            if next.is_empty() {
                reached = true;
                break;
            }
            cursor = next;
        }
        let how = if reached {
            "caught-up"
        } else {
            "caught-up=incomplete"
        };
        eprintln!(
            "fs-starter: watch module={} form={} {how} changes={read}",
            self.module, self.form
        );
    }

    /// After `ended`: subscribe again, then read the state (the changes of the own program, then the window reads its page again). A
    /// refusal 503 waits and asks again until a watch stands (or the window is gone). An `ended` that came meanwhile runs it once more.
    async fn resume(weak: Weak<Self>) {
        let mut waits = Backoff::new(&WAITS);
        loop {
            let Some(me) = weak.upgrade() else { return };
            let floor = me.seen.borrow().high;
            match me.subscribe().await {
                Ok(()) => {
                    me.catch_up(floor).await;
                    // contract: "subscribe again and then read the current state"; changes of other programs are only read this way
                    me.counter.bump();
                    eprintln!(
                        "fs-starter: watch module={} form={} resubscribed reread",
                        me.module, me.form
                    );
                    waits.reset();
                    if me.again.replace(false) {
                        continue;
                    }
                    me.resuming.set(false);
                    return;
                }
                Err((status, text)) => {
                    me.status.set(status);
                    eprintln!(
                        "fs-starter: watch module={} form={} unavailable status={status} note=\"{}\"",
                        me.module,
                        me.form,
                        text.chars().take(80).collect::<String>()
                    );
                    if status != UNAVAILABLE {
                        // not "the feed is gone": the connection or the form is, and the hold attaches anew on its own
                        me.again.set(false);
                        me.resuming.set(false);
                        return;
                    }
                }
            }
            drop(me);
            tokio::time::sleep(waits.next_pause()).await;
        }
    }

    /// Begin asking for a new watch (the first `watch` of a connection was refused 503).
    pub(super) fn start_resume(self: &Rc<Self>) {
        if !self.resuming.replace(true) {
            tokio::task::spawn_local(Self::resume(Rc::downgrade(self)));
        }
    }

    /// The feed of generation `generation` ended with `status`: start the one resume (stale and repeated ends are ignored; an end
    /// during a resume is kept for one more run).
    pub(super) fn ended(self: &Rc<Self>, generation: u64, status: u16) {
        if generation != self.generation.get() || generation <= self.ended_seen.get() {
            return;
        }
        self.ended_seen.set(generation);
        eprintln!(
            "fs-starter: watch module={} form={} ended status={status}",
            self.module, self.form
        );
        if self.resuming.replace(true) {
            // the new subscription ended while the resume still runs: run it once more after
            self.again.set(true);
            return;
        }
        tokio::task::spawn_local(Self::resume(Rc::downgrade(self)));
    }
}

#[cfg(all(test, unix))]
#[path = "watch_resume_tests.rs"]
mod tests;
