//! The lines a read and a refusal say on stderr (and in `--check-system`): one line per form read, one line per code.
//!
//! Pattern: Formatter (records and codes in, one stable text line out; the words are what the tests and the check read).

use fs_forms_run::{Coded, FormRecords};

/// What a read gave, one line per form: `fs-starter: read module=M form=F records=N ms=T provider=P` (+ ` empty` when the service answered
/// with no record: the service is there, there is just nothing in it).
#[must_use]
pub fn read_lines(module: &str, records: &FormRecords, took: std::time::Duration) -> Vec<String> {
    if records.forms.is_empty() {
        return vec![format!(
            "fs-starter: read module={module} form=- records=0 ms={} provider={:?} no-form",
            took.as_millis(),
            records.provider
        )];
    }
    records
        .forms
        .iter()
        .map(|f| {
            let n = f.rows.len();
            format!(
                "fs-starter: read module={module} form={} records={n} ms={} provider={:?}{}",
                f.declared.form.name,
                took.as_millis(),
                records.provider,
                if n == 0 { " empty" } else { "" }
            )
        })
        .collect()
}

/// A code with its parameters as one line (`code key=value ...`).
#[must_use]
pub fn line(c: &Coded) -> String {
    let params: Vec<String> = c.params.iter().map(|(k, v)| format!("{k}={v}")).collect();
    format!("{} {}", c.code, params.join(" "))
        .trim_end()
        .to_owned()
}
