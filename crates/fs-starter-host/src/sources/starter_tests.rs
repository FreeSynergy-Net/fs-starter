//! The line a read says (`read_lines`): module, form, count, time, provider, and the mark of an empty read.
//!
//! Pattern: Table of cases (one read, one line).

use super::*;
use fs_forms_bind::{FormRef, ModuleForm};
use std::time::Duration;

fn data(rows: usize) -> FormData {
    let declared = ModuleForm {
        data_type: String::new(),
        form: FormRef {
            name: "board".to_owned(),
            major: 1,
            minor: 0,
        },
        map: Vec::new(),
        fields: Vec::new(),
    };
    let rows = (0..rows)
        .map(|i| fs_forms_run::FormRow {
            key: i.to_string(),
            values: Vec::new(),
            may: Vec::new(),
        })
        .collect();
    FormData::read_only(declared, rows)
}

#[test]
fn a_read_says_module_form_count_and_time_and_an_empty_one_is_marked() {
    let full = FormRecords {
        provider: "real".to_owned(),
        forms: vec![data(3)],
        ..FormRecords::default()
    };
    let line = &read_lines("planning", &full, Duration::from_millis(7))[0];
    assert_eq!(
        line,
        "fs-starter: read module=planning form=board records=3 ms=7 provider=\"real\""
    );
    let empty = FormRecords {
        forms: vec![data(0)],
        ..full
    };
    assert!(
        read_lines("planning", &empty, Duration::ZERO)[0]
            .ends_with("records=0 ms=0 provider=\"real\" empty")
    );
    assert!(
        read_lines("planning", &FormRecords::default(), Duration::ZERO)[0].ends_with("no-form")
    );
}
