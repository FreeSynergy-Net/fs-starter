//! The make of the sample records of a window: the description in, the held units and the records out ([`connect`]).
//!
//! Pattern: Facade over the units (one sample module per `[[source]]`), see the parent module.

use std::sync::Arc;
use std::time::Instant;

use fs_forms_run::{CatalogBlockForms, Coded, Described, FormRecords, Records};
use fs_forms_testdata::{SAMPLE_PROVIDER, SampleSpec};
use fs_types::coded::{ErrorCode, Param, message_key};

use super::unit::Unit;
use super::writer::SampleWriter;
use crate::sources::connect::identity;
use crate::sources::held::{Held, HeldRecords};
use crate::sources::starter::{StarterSources, read_lines};

/// The name the lines and codes of a sample source say (it is no module of the registry).
pub(super) const NAME: &str = "sample";

/// A failure of the test data as a code of the host: `host-notice-sample-<what>` with the code of the builder and its parameters.
pub(super) fn failed<C: ErrorCode>(what: &str, e: &fs_types::coded::Coded<C>) -> Coded {
    let mut c =
        Coded::new(format!("host-notice-sample-{what}")).with("cause", message_key(e.code()));
    for (k, v) in e.params() {
        c = match v {
            Param::Text(t) => c.with(k, t.clone()),
            Param::Number(n) => c.with(k, n.to_string()),
            _ => c,
        };
    }
    c
}

/// The records of a window with `--sample-data`: one held sample module per `[[source]]`.
///
/// # Errors
/// `host-notice-sample-no-description` (the window has no description to make test data from), `host-notice-sample-spec` (the
/// wishes cannot be met: the code of the builder is the `cause`), `host-notice-sample-build`, `host-notice-sample-start`.
pub(in crate::sources) fn connect(
    sources: &StarterSources,
    described: Described<'_>,
) -> Result<Arc<dyn Records>, Coded> {
    let layout = described
        .layout()
        .ok_or_else(|| Coded::new("host-notice-sample-no-description"))?;
    let specs = SampleSpec::from_description(layout, &CatalogBlockForms, described.sample())
        .map_err(|e| failed("spec", &e))?;
    let began = Instant::now();
    let me = identity(sources.program.as_deref(), sources.slot);
    let mut units: Vec<Arc<Unit>> = Vec::new();
    for spec in &specs {
        let mut set = spec.build().map_err(|e| failed("build", &e))?;
        super::bind::commands(&mut set);
        units.push(Arc::new(Unit::start(
            set,
            spec.source,
            &me,
            sources.limits,
        )?));
    }
    let first = units
        .first()
        .ok_or_else(|| Coded::new("host-notice-sample-no-source"))?;
    let session = Arc::clone(first.session());
    let (forms, problems) = super::writer::read_all(&units)?;
    let records = FormRecords {
        provider: SAMPLE_PROVIDER.to_owned(),
        forms,
        problems,
        writer: Some(Arc::new(SampleWriter::new(units))),
        ..FormRecords::default()
    };
    // what was made, never silent (as for a real module): the person can tell sample lines from real ones
    for line in read_lines(NAME, &records, began.elapsed()) {
        eprintln!("{line}");
    }
    Ok(Arc::new(HeldRecords::new(
        records,
        Held::Session(session),
        None,
    )))
}
