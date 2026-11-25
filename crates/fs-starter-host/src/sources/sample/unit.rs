//! One sample source: a `SampleModule` held in this process (`hold_in_process`, no socket, no second process) behind the wrapper
//! `Guarded`, with the `Session` of the window on it. Reading goes through `read_pages` on the capability of the module, exactly as
//! for a service.
//!
//! Pattern: Adapter (the hold of a local capability in place of the hold of a socket; same handle `Hold`, same `Session`).

use std::sync::Arc;

use capnp::capability::FromClientHook;
use fs_forms_run::{ChangeCounter, Coded};
use fs_forms_testdata::{SampleModule, SampleSet};
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::module::Actor;
use fs_rpc::{Caller, FixedPerson, Guarded, Job, PerConnection, Ports, hold_in_process};
use fs_types::{BasicValue, PersonArt, PersonRef, Reference};

use super::make::{NAME, failed};
use crate::sources::answer::Limits;
use crate::sources::entries::Wire;
use crate::sources::pages::{Pages, read_pages};
use crate::sources::service::{UNREACHABLE, cause};
use crate::sources::session::Session;
use crate::sources::watch::Identity;

/// The person of the sample: made up, in this process only; it owns nothing and is no account.
const PERSON: &str = "urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e01";

fn person() -> PersonRef {
    PersonRef {
        id: Reference::parse(PERSON).expect("an absolute reference"),
        art: PersonArt::Person,
        abzug: None,
    }
}

/// A sample source of the description (`source` counts from 1) and the reads that go with it.
pub(super) struct Unit {
    session: Arc<Session>,
    wires: Vec<Wire>,
    limits: Limits,
}

impl Unit {
    /// Hold the module of `set` and introduce this window to it (history of the window, `app.undo`).
    ///
    /// # Errors
    /// `host-notice-sample-module` (the set gives no module), `host-notice-data-unreachable` (cause `hold`, `introduce`).
    pub(super) fn start(
        set: SampleSet,
        source: u32,
        me: &Identity,
        limits: Limits,
    ) -> Result<Self, Coded> {
        let wires = set
            .declared
            .iter()
            .map(|form| Wire {
                source,
                data_type: form.data_type.clone(),
                form: form.clone(),
                scope: String::new(),
            })
            .collect();
        // the module is `!Send` (built on the thread of the hold): checked here first, so the build there cannot fail
        SampleModule::new(&set).map_err(|e| failed("module", &e))?;
        let hold = hold_in_process(NAME, move || {
            let ports = Ports::new(|_: &Caller| Actor::Person, FixedPerson(person()));
            let module = SampleModule::new(&set).expect("the set was checked");
            Guarded::new(module, ports).bootstrap(&Caller::new("in-process", None))
        })
        .map_err(|_| cause(UNREACHABLE, NAME, "hold"))?;
        let session = Arc::new(Session::new(
            hold,
            NAME,
            me.clone(),
            limits.time,
            ChangeCounter::new(),
        ));
        introduce(&session, me)?;
        Ok(Self {
            session,
            wires,
            limits,
        })
    }

    pub(super) fn session(&self) -> &Arc<Session> {
        &self.session
    }

    /// Whether this source serves the form `name`.
    pub(super) fn serves(&self, name: &str) -> bool {
        self.wires.iter().any(|w| w.form.form.name == name)
    }

    /// The pages of every form of this source, read through the module.
    ///
    /// # Errors
    /// The codes of `read_pages`, `host-notice-data-unreachable` (cause `hold`).
    pub(super) fn read(&self) -> Result<Pages, Coded> {
        let (wires, limits) = (self.wires.clone(), self.limits);
        let job: Job<Result<Pages, Coded>> = Box::new(move |client| {
            Box::pin(async move {
                let source: form_source::Client = FromClientHook::new(client.hook);
                Ok(read_pages(&source, NAME, &wires, &limits).await)
            })
        });
        self.session.run(job)?
    }
}

/// `introduce`: the window says who it is (a revert is allowed only for the program that made the change).
fn introduce(session: &Session, me: &Identity) -> Result<(), Coded> {
    let (program, instance) = (me.program.clone(), me.instance.clone());
    let job: Job<()> = Box::new(move |client| {
        Box::pin(async move {
            let source: form_source::Client = FromClientHook::new(client.hook);
            let mut request = source.introduce_request();
            {
                let mut p = request.get();
                p.set_program(program.as_str());
                p.set_instance(instance.as_str());
                p.set_device("");
            }
            request
                .send()
                .promise
                .await
                .map(|_| ())
                .map_err(std::io::Error::other)
        })
    });
    session.run(job)
}

impl std::fmt::Debug for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Unit")
            .field("wires", &self.wires.len())
            .finish_non_exhaustive()
    }
}
