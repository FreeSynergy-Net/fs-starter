//! The problem of a source from number 2 reaches the records the WINDOW gets: `connect_wanted` of the port `Sources` (the same road the
//! window takes) hands out `HeldRecords`, and its `source_problems()` must say `(2, code)` while the forms of source 1 are there.
//! The service is the fake module (a real process started by the host through the registry, level 2). The refusal of an unknown scope by
//! the REAL planning service is tested at `read_pages` (`module_planning.rs`): the fake accepts any scope, and planning cannot be started
//! from here as a process (no binary of it in this workspace).
//!
//! Pattern: Adapter test (port `Sources` -> `dyn Records`); Unix only.
#![cfg(unix)]

mod support;
use std::path::Path;

use fs_forms_run::{Sources, Want};
use support::inproc::setup;
use support::install::{GOOD_FORMS, commands};

fn want(n: u32, t: &str, scope: &str) -> Want {
    Want {
        n,
        range: "groups@>=0.1.0, <0.2.0".into(),
        data_type: t.into(),
        scope: scope.into(),
    }
}

#[test]
fn an_unserved_second_source_reaches_the_window_records_as_a_problem_and_source_one_is_read() {
    let s = setup("groups", GOOD_FORMS, &commands("groups", "0.1.0"), 300);
    let records = s
        .sources
        .connect_wanted(
            &[want(1, "Group", ""), want(2, "Ghost", "")],
            None,
            Path::new("x"),
        )
        .unwrap();
    // the forms of source 1 are there and the problem of source 2 is on the records the WINDOW holds (`HeldRecords::source_problems`)
    assert!(
        records
            .forms()
            .iter()
            .any(|f| f.source == 1 && !f.rows.is_empty()),
        "the rows of source 1"
    );
    let problems = records.source_problems();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(
        (problems[0].0, problems[0].1.code.as_str()),
        (2, "host-notice-source-type-unserved")
    );
}
