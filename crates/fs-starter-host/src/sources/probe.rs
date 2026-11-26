//! What a probe (`--check-system`) may read: only the forms that need NO scope. A form of the kind `single-record` is read in the scope
//! of ONE record (a profile id, `urn:uuid:...`); a module refuses any other scope with 404 `unknown-scope` (the contract stays). So the
//! probe does not read it: it names it (`single-record-not-probed`, yellow), never red and never silent. Reading it with the id of
//! the first row of the list form is not done: the pairing of a list form and a single form of one type is the layout's business.
//!
//! Pattern: Specification – one predicate ("needs a record scope") splits the reads of a selection into those the probe makes and those it names.

use super::entries::{Selection, Wire};

/// The form name (without version) whose scope is always one record id.
pub const SINGLE_RECORD: &str = "single-record";

/// The code of the yellow finding for a form the probe did not read.
pub const NOT_PROBED: &str = "single-record-not-probed";

/// What a probe did: one `read` line per form read, and the forms it left out (`name@major.minor`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Probed {
    /// The lines of the reads, as `read_lines` makes them.
    pub read: Vec<String>,
    /// The forms of the module that need a record scope and were not read.
    pub not_probed: Vec<String>,
}

impl Probed {
    /// The yellow findings (`single-record-not-probed module=M form=F`), one per form left out.
    #[must_use]
    pub fn not_probed_lines(&self, module: &str) -> Vec<String> {
        self.not_probed
            .iter()
            .map(|f| format!("{NOT_PROBED} module={module} form={f}"))
            .collect()
    }
}

/// The reads of `chosen` that need no scope, and the keys of the single-record forms left out.
#[must_use]
pub fn split(chosen: Selection) -> (Vec<Wire>, Vec<String>) {
    let (single, plain): (Vec<Wire>, Vec<Wire>) = chosen
        .wires
        .into_iter()
        .partition(|w| w.form.form.name == SINGLE_RECORD);
    let mut left: Vec<String> = Vec::new();
    for w in &single {
        let key = format!(
            "{}@{}.{}",
            w.form.form.name, w.form.form.major, w.form.form.minor
        );
        if !left.contains(&key) {
            left.push(key);
        }
    }
    (plain, left)
}
