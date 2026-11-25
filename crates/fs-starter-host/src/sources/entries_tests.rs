//! Tests of [`super::select`]: the number, type and scope of a source decide which entries are read, and how they are numbered.
//!
//! Pattern: Registry test (key form + type).

use super::*;
use fs_forms_bind::FormRef;

fn entry(name: &str, data_type: &str) -> Entry {
    Entry {
        form: ModuleForm {
            data_type: data_type.into(),
            form: FormRef {
                name: name.into(),
                major: 1,
                minor: 0,
            },
            map: Vec::new(),
            fields: Vec::new(),
        },
        data_type: data_type.into(),
    }
}

fn want(n: u32, t: &str, s: &str) -> Wanted {
    Wanted {
        n,
        data_type: t.into(),
        scope: s.into(),
    }
}

#[test]
fn a_source_takes_only_the_entries_of_its_type_with_its_own_scope() {
    let entries = [entry("rows", "Lane"), entry("rows", "Task")];
    let got = select(
        &entries,
        &[want(1, "Task", ""), want(2, "Lane", "lanes:assignee")],
    );
    let got: Vec<_> = got
        .wires
        .iter()
        .map(|w| (w.source, w.data_type.as_str(), w.scope.as_str()))
        .collect();
    // the number of the layout (from 1) goes with each read: source 1 = Task, source 2 = Lane, never swapped, never from 0
    assert_eq!(got, [(1, "Task", ""), (2, "Lane", "lanes:assignee")]);
}

#[test]
fn two_sources_of_one_type_with_other_scopes_are_two_reads() {
    let entries = [entry("rows", "Task")];
    let got = select(&entries, &[want(1, "Task", ""), want(2, "Task", "x")]);
    let got: Vec<_> = got
        .wires
        .iter()
        .map(|w| (w.source, w.scope.as_str()))
        .collect();
    assert_eq!(got, [(1, ""), (2, "x")]);
}

#[test]
fn a_source_without_a_type_is_placed_by_the_stop_gap_with_its_own_number() {
    let entries = [
        entry("rows", "Lane"),
        entry("rows", "Task"),
        entry("cards", "Task"),
    ];
    let got = select(&entries, &[want(4, "", "")]);
    assert!(
        got.wires
            .iter()
            .all(|w| w.source == 4 && w.data_type == "Task"),
        "{:?}",
        got.wires
    );
    assert_eq!(got.wires.len(), 2);
}

#[test]
fn a_type_nobody_asks_for_is_not_read() {
    let entries = [entry("rows", "Lane"), entry("rows", "Task")];
    let got = select(&entries, &[want(1, "Task", "")]);
    assert_eq!(got.wires.len(), 1);
    assert_eq!(got.wires[0].data_type, "Task");
}

#[test]
fn without_a_wish_an_ambiguous_form_is_read_only_for_the_primary_type_and_says_so() {
    let entries = [
        entry("cards", "Task"),
        entry("rows", "Lane"),
        entry("rows", "Task"),
    ];
    let got = select(&entries, &[]);
    let read: Vec<_> = got
        .wires
        .iter()
        .map(|w| {
            (
                w.form.form.name.as_str(),
                w.data_type.as_str(),
                w.scope.as_str(),
            )
        })
        .collect();
    assert_eq!(read, [("cards", "Task", ""), ("rows", "Task", "")]);
    assert_eq!(got.notices.len(), 1);
    assert!(got.notices[0].contains("Lane, Task"), "{:?}", got.notices);
}

#[test]
fn without_a_wish_and_without_a_primary_type_an_ambiguous_form_is_not_read() {
    let entries = [entry("rows", "Lane"), entry("rows", "Task")];
    let got = select(&entries, &[]);
    assert_eq!(got.wires, []);
    assert!(got.notices[0].contains("not read"), "{:?}", got.notices);
}

#[test]
fn a_source_type_no_entry_has_is_said() {
    let got = select(&[entry("rows", "Task")], &[want(3, "Ghost", "x")]);
    assert_eq!(got.wires, []);
    assert!(got.notices[0].contains("Ghost"), "{:?}", got.notices);
    // and it is a problem of THAT source (shown by the window), not only a line on stderr
    assert_eq!(got.problems.len(), 1);
    assert_eq!(got.problems[0].0, 3);
    assert_eq!(got.problems[0].1.code, "host-notice-source-type-unserved");
}
