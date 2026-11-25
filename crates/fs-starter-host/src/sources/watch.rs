//! What happens FIRST on every connection the window holds (level 2): `introduce(program, instance, device)` and `watch(form, scope, sink)`
//! for every read of the window: the SAME key under two scopes (the lane list and the cards of a board) are two watches with their own sink;
//! a form the module cannot watch (planning: `rows@1.0.0`) is said (`watch ... unavailable`) and left out, the other watches stay. One per CONNECTION (= window): never per account or process, so a change that window A made reaches
//! window B as an event; only the changes of THIS connection (same program and instance) are left out, never "from the same person".
//! `device` stays empty in stage 3 (no stand-in key). The sink counts the change ([`ChangeCounter`]) and says
//! `fs-starter: watch module=M change=<activity> by=<program>` on stderr.
//!
//! When the service ENDS a watch (410) the watch is built again and what was missed is read: [`super::watch_resume`].
//!
//! Pattern: Observer (`ChangeSink` of the module, the counter is the window's side) + Template Method (`OnAttach`: runs again after every reconnect).

use std::any::Any;
use std::io;

use capnp::capability::{Client, FromClientHook};
use fs_forms_run::ChangeCounter;
use fs_interfaces::formsource_capnp::form_source;
use fs_rpc::OnAttach;
use futures::future::LocalBoxFuture;

use super::reply::Change;
use super::watch_resume::{UNAVAILABLE, Watch};

/// Who this connection says it is (`introduce`). `device` is not here: it stays empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Identity {
    pub program: String,
    pub instance: String,
}

/// The line one change says on stderr.
#[must_use]
pub(super) fn watch_line(module: &str, change: &Change) -> String {
    format!(
        "fs-starter: watch module={module} change={} by={}",
        change.id(),
        change.program
    )
}

/// Whether the change came over THIS connection (same program and instance): the only thing that is left out.
#[must_use]
pub(super) fn is_mine(change: &Change, me: &Identity) -> bool {
    change.program == me.program && change.instance == me.instance
}

/// The registration on a new connection.
pub(super) struct Attach {
    pub module: String,
    pub me: Identity,
    pub forms: Vec<(String, String)>,
    pub counter: ChangeCounter,
}

impl OnAttach for Attach {
    fn attach(&self, client: Client) -> LocalBoxFuture<'_, io::Result<Box<dyn Any>>> {
        Box::pin(async move {
            let source: form_source::Client = FromClientHook::new(client.hook);
            let mut intro = source.introduce_request();
            {
                let mut p = intro.get();
                p.set_program(self.me.program.as_str());
                p.set_instance(self.me.instance.as_str());
                p.set_device("");
            }
            intro.send().promise.await.map_err(io::Error::other)?;
            let mut watches = Vec::new();
            for form in &self.forms {
                let watch = Watch::new(&self.module, &self.me, form, &self.counter, &source);
                match watch.subscribe().await {
                    Ok(()) => {
                        watch.baseline().await;
                        watches.push(watch);
                    }
                    // the feed of the module is gone: the watch is kept and asks again until one stands (503, never 410)
                    Err((UNAVAILABLE, _)) => {
                        watch.start_resume();
                        watches.push(watch);
                    }
                    // a form the module cannot watch is said and left out (never silent, never the end of the hold)
                    Err(_) => eprintln!(
                        "fs-starter: watch module={} form={} unavailable",
                        self.module, form.0
                    ),
                }
            }
            // dropping the answer ends the subscriptions (the connection is gone or the hold ended)
            Ok(Box::new(watches) as Box<dyn Any>)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(program: &str, instance: &str) -> Change {
        Change {
            activity: 0x0190_0000_0000_7000_8000_0000_0000_000b,
            program: program.into(),
            instance: instance.into(),
            person: "p".into(),
        }
    }

    #[test]
    fn only_the_change_of_the_own_connection_is_left_out_never_one_of_the_same_program_or_person() {
        let me = Identity {
            program: "tasks".into(),
            instance: "7/1".into(),
        };
        assert!(is_mine(&change("tasks", "7/1"), &me));
        // another window of the same program in the same process, another process, another program: all are events
        assert!(!is_mine(&change("tasks", "7/2"), &me));
        assert!(!is_mine(&change("tasks", "8/1"), &me));
        assert!(!is_mine(&change("planning", "7/1"), &me));
    }

    #[test]
    fn the_line_names_the_module_the_change_and_the_program() {
        let l = watch_line("planning", &change("tasks", "7/1"));
        assert_eq!(
            l,
            "fs-starter: watch module=planning change=01900000-0000-7000-8000-00000000000b by=tasks"
        );
    }
}

#[cfg(all(test, unix))]
#[path = "watch_tests.rs"]
mod scoped_tests;
