//! SOLL S1 of the host review: the write road of the HOST over the wire. `StarterSources::invoke` (the check of the own name space, the
//! key, `invoke::call`: args, `read_answer`, `Written`) and `refusal_or` on a failed page go through the real fake PROCESS (the strict
//! `FakeSource` of fs-module behind a socket), with no helper of the tests in between.
//!
//! Mutants: path and text swapped in `invoke.rs` = case 1 red (bad arguments); `kind_code` dropped = cases 2 and 3 red.
#![cfg(unix)]

mod support;
use fs_starter_host::sources::Written;
use support::inproc::{commands_of, setup};

/// A `forms.json` of the form the fake serves; `version` is the one the module claims.
fn forms(version: &str) -> String {
    format!(
        r#"[{{"name":"cards-in-columns","version":"{version}","type":"Card","map":{{"label":"title"}},"fields":{{"title":"text"}}}}]"#
    )
}

fn module(version: &str) -> support::inproc::InProc {
    setup(
        "cards",
        &forms(version),
        &commands_of("cards", "card.move"),
        2000,
    )
}

#[test]
fn a_command_with_its_argument_is_accepted_and_names_the_record() {
    let s = module("1.0");
    // the field `column` is the PATH, `done` the TEXT: swapped or lost they would be refused as bad arguments
    let got = s
        .sources
        .invoke(
            "cards",
            "cards.card.move",
            "c1",
            &[("column".to_owned(), "done".to_owned())],
        )
        .expect("accepted");
    assert_eq!(got.record, "c1");
    assert_ne!(got, Written::default());
}

#[test]
fn a_command_without_its_argument_is_refused_with_the_kind_of_the_service() {
    let s = module("1.0");
    let refused = s
        .sources
        .invoke("cards", "cards.card.move", "c1", &[])
        .unwrap_err();
    assert_eq!(refused.code, "form-source-bad-arguments");
}

#[test]
fn a_form_of_a_major_the_service_does_not_have_is_refused_on_the_read_road() {
    let s = module("9.0");
    let refused = s.sources.probe("cards").unwrap_err();
    assert_eq!(refused.code, "form-source-unknown-form");
}

#[test]
fn a_refusal_names_its_reason_in_the_data_next_to_the_code() {
    let s = module("1.0");
    let refused = s
        .sources
        .invoke("cards", "cards.card.move", "c1", &[])
        .unwrap_err();
    assert_eq!(refused.code, "form-source-bad-arguments");
    let reason = refused
        .params
        .iter()
        .find(|(k, _)| k == "reason")
        .map(|(_, v)| v.as_str())
        .expect("the data carry a reason");
    assert!(!reason.is_empty() && reason.len() <= 120, "{reason}");
    assert!(refused.params.contains(&("status".into(), "400".into())));
}
