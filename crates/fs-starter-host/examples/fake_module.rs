//! TEST SUPPORT, not a program of the suite: a module service that serves the contract `FormSource` with made-up groups data, so the
//! tests (and the pictures of SQ-4) can run the whole road of the host (registry, `forms.json`, `ensure_single`, hello, heartbeat, pages)
//! without a real module. It says `provider = "fake"`: the window shows "Sample data" and never passes the records off as real.
//!
//! The contract part is the strict `FakeSource` of the suite `fs-module` (fs-interfaces v0.1.0, crate `fs-module`: hello, blob, invoke and the
//! form `cards-in-columns@1.0.0`, all rules of level 1); this file is only the process around it (socket, heartbeat, `FAKE_MODE`) and the made-up
//! groups data of the other forms. Refusals leave as `fs-rpc` problems (type `form-source-<kind>`), as a real module does.
//!
//! Started by the host as `<module folder>/service serve`. The module name is `FAKE_MODULE` (default `groups`). `FAKE_MODE` makes
//! it misbehave for the tests of the borders: `real` (provider is not fake), `level` (a level the host does not know), `nobeat` (no
//! heartbeat), `hostile` (control characters, an over-long text, more records than the limit), `slow` (does not answer a page in time).
//!
//! Pattern: Fixture (a fake service) + Decorator (the process around `FakeSource`); Unix only (the heartbeat of fs-rpc is Unix only).

#[cfg(unix)]
mod unix {
    use std::time::Duration;

    use capnp_rpc::new_client;
    use fs_forms_run::sample_groups;
    use fs_interfaces::forms::{FORM_SOURCE_LEVEL, parse_key};
    use fs_interfaces::formsource_capnp::form_source;
    use fs_module::{FakeSource, Mode};
    use fs_rpc::{
        Caller, Heartbeat, PerConnection, Problem, ServerIdentity, Serving,
        run_until_signal_serving,
    };
    use std::rc::Rc;

    /// The form the strict `FakeSource` serves (its own data: three cards, the commands of the suite).
    const CARDS: &str = "cards-in-columns";

    /// One connection (= one window or program): its OWN `introduce`, the cards and the log shared with every other connection.
    struct Fake {
        mode: String,
        /// The contract: every rule of level 1 and 2 (version, scope, commands, arguments, rights, `result`, `invokeIf`, `revert`, `watch`).
        cards: Rc<FakeSource>,
    }

    /// The service: a new [`Fake`] for each connection, so two windows are two connections and `introduce` holds for one of them.
    struct Per {
        mode: String,
        _beat: Option<Heartbeat>,
        cards: FakeSource,
    }

    impl PerConnection for Per {
        fn bootstrap(&mut self, _: &Caller) -> capnp::capability::Client {
            let root: form_source::Client = new_client(Fake {
                mode: self.mode.clone(),
                cards: Rc::new(self.cards.connection()),
            });
            root.client
        }
    }

    /// A refusal of `FakeSource` (text `form-source-<kind>: <detail> (<status>)`) as the problem a real module sends.
    fn as_problem(e: capnp::Error) -> capnp::Error {
        let text = e.extra.clone();
        let (kind, rest) = text.split_once(": ").unwrap_or((text.as_str(), ""));
        let status = rest
            .rsplit_once('(')
            .and_then(|(_, s)| s.trim_end_matches(')').parse().ok())
            .unwrap_or(500);
        let detail = rest.rsplit_once(" (").map_or(rest, |(d, _)| d);
        if kind.starts_with("form-source-") {
            Problem::new(kind, kind, detail, status).into_error()
        } else {
            e
        }
    }

    impl Fake {
        fn is(&self, mode: &str) -> bool {
            self.mode == mode
        }
    }

    #[allow(clippy::unused_async_trait_impl)] // the generated trait wants async methods
    impl form_source::Server for Fake {
        async fn hello(
            self: capnp::capability::Rc<Self>,
            _: form_source::HelloParams,
            mut results: form_source::HelloResults,
        ) -> Result<(), capnp::Error> {
            let mut r = results.get();
            // `level`: below the minimum (refused); `level99`: a newer service (taken, the host works with its own level)
            r.set_level(match self.mode.as_str() {
                "level" => 0,
                "level99" => 99,
                "level1" => 1,
                _ => FORM_SOURCE_LEVEL,
            });
            r.set_provider(if self.is("real") { "groups" } else { "fake" });
            Ok(())
        }

        async fn page(
            self: capnp::capability::Rc<Self>,
            params: form_source::PageParams,
            mut results: form_source::PageResults,
        ) -> Result<(), capnp::Error> {
            if self.is("slow") {
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
            let form = params.get()?.get_form()?.to_str()?.to_owned();
            // strict like the contract: a key is `name@x.y` or `name@x.y.z`, nothing else is a form
            let Some((name, _)) = parse_key(&form) else {
                return Err(unknown_form());
            };
            if name == CARDS {
                return form_source::Server::page(self.cards.clone(), params, results)
                    .await
                    .map_err(as_problem);
            }
            let data = sample_groups();
            let Some(d) = data.iter().find(|d| d.declared.form.name == name) else {
                return Err(unknown_form());
            };
            let hostile = self.is("hostile");
            let count = if hostile { 5000 } else { d.rows.len() };
            let page = results.get().init_page();
            let mut list = page.init_records(u32::try_from(count).unwrap_or(0));
            for i in 0..count {
                let row = &d.rows[i % d.rows.len()];
                let mut rec = list.reborrow().get(u32::try_from(i).unwrap_or(0));
                rec.set_id(row.key.as_str());
                let n: usize = row.values.iter().map(|(_, v)| v.len()).sum();
                let mut values = rec.init_values(u32::try_from(n).unwrap_or(0));
                let mut at = 0;
                for (channel, vals) in &row.values {
                    let field = d
                        .declared
                        .map
                        .iter()
                        .find(|(c, _)| c == channel)
                        .and_then(|(_, f)| f.first())
                        .cloned()
                        .unwrap_or_default();
                    for v in vals {
                        let mut value = values.reborrow().get(at);
                        value.set_path(field.as_str());
                        if hostile && channel == "label" {
                            value
                                .set_text(format!("{v}\u{1b}[31m\nX{}", "y".repeat(5000)).as_str());
                        } else {
                            value.set_text(v.as_str());
                        }
                        at += 1;
                    }
                }
            }
            Ok(())
        }

        async fn blob(
            self: capnp::capability::Rc<Self>,
            params: form_source::BlobParams,
            results: form_source::BlobResults,
        ) -> Result<(), capnp::Error> {
            form_source::Server::blob(self.cards.clone(), params, results)
                .await
                .map_err(as_problem)
        }

        // No "yes to everything": the commands and the records are the ones of the suite (`card.move`, `card.create`, `card.delete`).
        async fn invoke(
            self: capnp::capability::Rc<Self>,
            params: form_source::InvokeParams,
            results: form_source::InvokeResults,
        ) -> Result<(), capnp::Error> {
            form_source::Server::invoke(self.cards.clone(), params, results)
                .await
                .map_err(as_problem)
        }

        // Level 2 (the contract of Stufe 3): the strict `FakeSource` answers; at `level1` it says `unimplemented` like a level 1 service.
        async fn invoke_if(
            self: capnp::capability::Rc<Self>,
            params: form_source::InvokeIfParams,
            results: form_source::InvokeIfResults,
        ) -> Result<(), capnp::Error> {
            form_source::Server::invoke_if(self.cards.clone(), params, results)
                .await
                .map_err(as_problem)
        }

        async fn revert(
            self: capnp::capability::Rc<Self>,
            params: form_source::RevertParams,
            results: form_source::RevertResults,
        ) -> Result<(), capnp::Error> {
            form_source::Server::revert(self.cards.clone(), params, results)
                .await
                .map_err(as_problem)
        }

        async fn changes(
            self: capnp::capability::Rc<Self>,
            params: form_source::ChangesParams,
            results: form_source::ChangesResults,
        ) -> Result<(), capnp::Error> {
            form_source::Server::changes(self.cards.clone(), params, results)
                .await
                .map_err(as_problem)
        }

        async fn introduce(
            self: capnp::capability::Rc<Self>,
            params: form_source::IntroduceParams,
            results: form_source::IntroduceResults,
        ) -> Result<(), capnp::Error> {
            form_source::Server::introduce(self.cards.clone(), params, results)
                .await
                .map_err(as_problem)
        }

        async fn watch(
            self: capnp::capability::Rc<Self>,
            params: form_source::WatchParams,
            results: form_source::WatchResults,
        ) -> Result<(), capnp::Error> {
            form_source::Server::watch(self.cards.clone(), params, results)
                .await
                .map_err(as_problem)
        }
    }

    fn unknown_form() -> capnp::Error {
        Problem::new(
            "form-source-unknown-form",
            "unknown-form",
            "no such form",
            404,
        )
        .into_error()
    }

    pub fn main() -> std::io::Result<()> {
        if std::env::args().nth(1).as_deref() != Some("serve") {
            eprintln!("fake-module: only `serve`");
            std::process::exit(2);
        }
        let module = std::env::var("FAKE_MODULE").unwrap_or_else(|_| "groups".into());
        let mode = std::env::var("FAKE_MODE").unwrap_or_default();
        let socket = fs_rpc::default_socket(&module).expect("a runtime folder");
        // `FAKE_IDLE_MS`: the service ends by itself after this long without a connection (what `MODULE_IDLE` is for a module), so a test
        // can show that it lives while held and ends after the hold dropped; without it the service runs until it is stopped
        let mut serving = Serving::new(ServerIdentity::new("fake-module", ""));
        if let Some(ms) = std::env::var("FAKE_IDLE_MS")
            .ok()
            .and_then(|v| v.parse().ok())
        {
            serving = serving.with_idle_timeout(Duration::from_millis(ms));
        }
        run_until_signal_serving(&socket.clone(), serving, || async move {
            let beat = if mode == "nobeat" {
                None
            } else {
                Some(Heartbeat::start(&socket)?)
            };
            let cards = FakeSource::new(Mode::Strict);
            let cards = if mode == "level1" {
                cards.with_level(1)
            } else {
                cards
            };
            Ok(Per {
                mode,
                _beat: beat,
                cards,
            })
        })
    }
}

#[cfg(unix)]
fn main() -> std::io::Result<()> {
    unix::main()
}

#[cfg(not(unix))]
fn main() {}
