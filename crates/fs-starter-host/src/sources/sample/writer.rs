//! The way of a gesture to a sample source: the port `Writer` over the units. A write goes as `invokeIf` over the held connection of the
//! unit that serves the form, so it is part of its history; nothing else is reached (no socket, no store).
//!
//! Pattern: Adapter - the port `fs_forms_run::Writer` on `FormSource.invokeIf` of the in-process module.

use std::sync::Arc;

use fs_forms_run::{Coded, FormData, Invocation, Writer, command_key};

use super::unit::Unit;

/// The forms read, and the sources (from number 2) that could not be read as `(number, code)`.
pub(super) type Read = (Vec<FormData>, Vec<(u32, Coded)>);

/// Every unit read again: the forms (with the keys of their commands) and the sources that could not be read.
pub(super) fn read_all(units: &[Arc<Unit>]) -> Result<Read, Coded> {
    let mut forms = Vec::new();
    let mut problems = Vec::new();
    for unit in units {
        let pages = unit.read()?;
        forms.extend(pages.forms);
        problems.extend(pages.problems);
    }
    for data in &mut forms {
        let keys: Vec<String> = data
            .declared
            .map
            .iter()
            .filter_map(|(channel, _)| command_key(data, channel).map(str::to_owned))
            .collect();
        data.commands = keys;
    }
    Ok((forms, problems))
}

/// The writer of the window over its sample sources.
#[derive(Debug)]
pub(super) struct SampleWriter {
    units: Vec<Arc<Unit>>,
}

impl SampleWriter {
    pub(super) fn new(units: Vec<Arc<Unit>>) -> Self {
        Self { units }
    }
}

impl Writer for SampleWriter {
    fn invoke(&self, call: &Invocation) -> Result<(), Coded> {
        let unit = self
            .units
            .iter()
            .find(|u| u.serves(&call.form))
            .ok_or_else(|| {
                Coded::new("host-notice-sample-form-unknown").with("form", &call.form)
            })?;
        let record = call.record.as_deref().unwrap_or("");
        unit.session()
            .invoke_if(&call.command, record, &call.args)
            .map(|_| ())
    }

    fn reload(&self) -> Result<Vec<FormData>, Coded> {
        read_all(&self.units).map(|(forms, _)| forms)
    }
}
