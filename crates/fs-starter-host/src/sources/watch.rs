//! What happens FIRST on every connection the window holds (level 2): `introduce(program, instance, device)` and `watch(form, "", sink)`
//! for every form of the module. One per CONNECTION (= window): never per account or process, so a change that window A made reaches
//! window B as an event; only the changes of THIS connection (same program and instance) are left out, never "from the same person".
//! `device` stays empty in stage 3 (no stand-in key). The sink counts the change ([`ChangeCounter`]) and says
//! `fs-starter: watch module=M change=<activity> by=<program>` on stderr.
//!
//! Pattern: Observer (`ChangeSink` of the module, the counter is the window's side) + Template Method (`OnAttach`: runs again after every reconnect).

use std::any::Any;
use std::io;

use capnp::capability::{Client, FromClientHook};
use fs_forms_run::ChangeCounter;
use fs_interfaces::formsource_capnp::{change_sink, form_source};
use fs_rpc::OnAttach;
use futures::future::LocalBoxFuture;

use super::reply::{Change, change_of};

/// Who this connection says it is (`introduce`). `device` is not here: it stays empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Identity {
    pub program: String,
    pub instance: String,
}

/// NOT yet the hub: the `generation` of a `RecordGrip` (fs-forms-run `RecordHub`) is counted only by the `ChangeSink` the hub itself gives to
/// `Module::connect`, and the hub has no write path nor `history`/`step_back`. This host holds its module directly (no `Module`/`Link` adapter
/// yet), so only `changed` is counted here. `invokeIf` sends `expect` empty: the page carries no revision and `Link::writer` does not hand the
/// seen one on (REVIEW-s6-r3 S1, S2).
/// The receiver of the changes of one connection.
struct Sink {
    module: String,
    me: Identity,
    counter: ChangeCounter,
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

impl change_sink::Server for Sink {
    #[allow(clippy::unused_async_trait_impl)] // the generated trait wants an async method
    async fn changed(
        self: std::rc::Rc<Self>,
        p: change_sink::ChangedParams,
    ) -> Result<(), capnp::Error> {
        let change = change_of(p.get()?.get_change()?)?;
        if !is_mine(&change, &self.me) {
            self.counter.bump();
            eprintln!("{}", watch_line(&self.module, &change));
        }
        Ok(())
    }
}

/// The registration on a new connection.
pub(super) struct Attach {
    pub module: String,
    pub me: Identity,
    pub forms: Vec<String>,
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
            let mut subscriptions = Vec::new();
            for form in &self.forms {
                let mut request = source.watch_request();
                {
                    let mut p = request.get();
                    p.set_form(form.as_str());
                    p.set_scope("");
                    p.set_sink(capnp_rpc::new_client(Sink {
                        module: self.module.clone(),
                        me: self.me.clone(),
                        counter: self.counter.clone(),
                    }));
                }
                // a form the module cannot watch is said and left out (never silent, never the end of the hold)
                match request
                    .send()
                    .promise
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|a| {
                        a.get()
                            .and_then(fs_interfaces::formsource_capnp::form_source::watch_results::Reader::get_subscription)
                            .map_err(|e| e.to_string())
                    }) {
                    Ok(subscription) => subscriptions.push(subscription),
                    Err(_) => eprintln!(
                        "fs-starter: watch module={} form={form} unavailable",
                        self.module
                    ),
                }
            }
            // dropping the answer ends the subscriptions (the connection is gone or the hold ended)
            Ok(Box::new(subscriptions) as Box<dyn Any>)
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
