//! TEST SUPPORT, not a program of the suite: a module service that serves the contract `FormSource` with made-up groups data, so the
//! tests (and the pictures of SQ-4) can run the whole road of the host (registry, `forms.json`, `ensure_single`, hello, heartbeat, pages)
//! without a real module. It says `provider = "fake"`: the window shows "Sample data" and never passes the records off as real.
//!
//! Started by the host as `<module folder>/service serve`. The module name is `FAKE_MODULE` (default `groups`). `FAKE_MODE` makes
//! it misbehave for the tests of the borders: `real` (provider is not fake), `level` (a level the host does not know), `nobeat` (no
//! heartbeat), `hostile` (control characters, an over-long text, more records than the limit), `slow` (does not answer a page in time).
//!
//! Pattern: Fixture (a fake service); Unix only (the heartbeat of fs-rpc is Unix only).

#[cfg(unix)]
mod unix {
    use std::time::Duration;

    use capnp_rpc::new_client;
    use fs_forms_run::sample_groups;
    use fs_interfaces::forms::FORM_SOURCE_LEVEL;
    use fs_interfaces::formsource_capnp::form_source;
    use fs_rpc::{Heartbeat, ServerIdentity, run_until_signal_as};

    struct Fake {
        mode: String,
        _beat: Option<Heartbeat>,
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
            r.set_level(if self.is("level") {
                99
            } else {
                FORM_SOURCE_LEVEL
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
            let name = form.split('@').next().unwrap_or("");
            let data = sample_groups();
            let Some(d) = data.iter().find(|d| d.declared.form.name == name) else {
                return Err(capnp::Error::failed("form-source-unknown-form".into()));
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
            _: form_source::BlobParams,
            _: form_source::BlobResults,
        ) -> Result<(), capnp::Error> {
            Err(capnp::Error::unimplemented("no blobs".into()))
        }

        async fn invoke(
            self: capnp::capability::Rc<Self>,
            _: form_source::InvokeParams,
            mut results: form_source::InvokeResults,
        ) -> Result<(), capnp::Error> {
            results.get().set_failed(false);
            Ok(())
        }
    }

    pub fn main() -> std::io::Result<()> {
        if std::env::args().nth(1).as_deref() != Some("serve") {
            eprintln!("fake-module: only `serve`");
            std::process::exit(2);
        }
        let module = std::env::var("FAKE_MODULE").unwrap_or_else(|_| "groups".into());
        let mode = std::env::var("FAKE_MODE").unwrap_or_default();
        let socket = fs_rpc::default_socket(&module).expect("a runtime folder");
        run_until_signal_as(
            &socket.clone(),
            ServerIdentity::new("fake-module", ""),
            || async move {
                let beat = if mode == "nobeat" {
                    None
                } else {
                    Some(Heartbeat::start(&socket)?)
                };
                let root: form_source::Client = new_client(Fake { mode, _beat: beat });
                Ok(root.client)
            },
        )
    }
}

#[cfg(unix)]
fn main() -> std::io::Result<()> {
    unix::main()
}

#[cfg(not(unix))]
fn main() {}
