//! SW-5: the HOST binds `<module>.<key>` from the contract key of the file (`capability`); the id never comes from the module.
//!
//! Tests: derivation from the folder, own prefix equal to the bare key, a foreign prefix never accepted, a given `id` must be the
//! derived one, a duplicate key refuses the set; no alias (`people.search` in module `people` is `people.people.search`). Mutants: the id is the key as written (no prefix added); the prefix is taken from the
//! `module` field instead of the folder; a foreign prefix is taken as it is; a given `id` is ignored; the own prefix is doubled;
//! `app.` is let through.

mod support;
use support::sets::json_keys;

use fs_starter_host::registry::{Actor, Refusal, Registry, parse};

fn file(module: &str, command: &str) -> String {
    format!(
        r#"{{"module":"{module}","version":"0.1.0","contract":"{module}","scope":"own-contract","commands":[{{{command},"call":"x","access":"read","help":{{"topic":"t","text":{{"en":"e","de":"d"}}}}}}]}}"#
    )
}

fn first_id(text: &str, folder: &str) -> Result<String, Refusal> {
    parse(text, folder).map(|s| s.items[0].command.id.as_str().to_owned())
}

#[test]
fn the_host_always_puts_the_module_name_before_the_key_no_alias() {
    let bare = file("planning", r#""capability":"card.move""#);
    let own = file("planning", r#""capability":"planning.card.move""#);
    assert_eq!(first_id(&bare, "planning").unwrap(), "planning.card.move");
    // a key that starts with the own module name is taken literally, never read as the id written out
    assert_eq!(
        first_id(&own, "planning").unwrap(),
        "planning.planning.card.move"
    );
    let people = file("people", r#""capability":"people.search""#);
    assert_eq!(first_id(&people, "people").unwrap(), "people.people.search");
}

#[test]
fn the_raw_contract_key_is_kept_for_invoke() {
    let set = parse(&file("people", r#""capability":"people.search""#), "people").unwrap();
    assert_eq!(set.items[0].key, "people.search");
    let mut r = Registry::new();
    r.add_bundled(Actor::Person, set).unwrap();
    let e = r.entries().iter().find(|e| e.module == "people").unwrap();
    assert_eq!(e.command.id.as_str(), "people.people.search");
    assert_eq!(e.key.as_deref(), Some("people.search"));
    // the shell's own commands have no key
    assert!(
        r.entries()
            .iter()
            .filter(|e| e.module == "app")
            .all(|e| e.key.is_none())
    );
}

#[test]
fn the_prefix_comes_from_the_folder_not_from_a_field_of_the_file() {
    // `module` says `planning`, the folder is `capture`: refused, and nothing is derived from the field
    let text = file("planning", r#""capability":"card.move""#);
    assert!(matches!(parse(&text, "capture"), Err(Refusal::ModuleName(n)) if n == "planning"));
}

#[test]
fn a_foreign_prefix_is_never_taken_as_it_is() {
    // without an id the key stays inside the own name space (`capture.tasks.create`), never `tasks.create`
    let text = file("capture", r#""capability":"tasks.create""#);
    assert_eq!(first_id(&text, "capture").unwrap(), "capture.tasks.create");
    // with an id of a foreign module: OutsideNamespace, with `app.`: ReservedPrefix, nothing is accepted
    let text = file(
        "capture",
        r#""id":"tasks.create","capability":"tasks.create""#,
    );
    assert!(
        matches!(first_id(&text, "capture"), Err(Refusal::OutsideNamespace(id)) if id.as_str() == "tasks.create")
    );
    let text = file("capture", r#""id":"app.undo","capability":"app.undo""#);
    assert!(
        matches!(first_id(&text, "capture"), Err(Refusal::ReservedPrefix(id)) if id.as_str() == "app.undo")
    );
    // a forged set that still reaches `add` with a foreign id is refused there
    let mut set = parse(&json_keys("capture", &["x"]), "capture").unwrap();
    set.items[0].command.id = "tasks.create".into();
    let why = Registry::new().add_bundled(Actor::Person, set).unwrap_err();
    assert!(matches!(&why[0], Refusal::OutsideNamespace(_)), "{why:?}");
}

#[test]
fn a_given_id_must_be_the_derived_one() {
    let same = file(
        "planning",
        r#""id":"planning.card.move","capability":"card.move""#,
    );
    assert_eq!(first_id(&same, "planning").unwrap(), "planning.card.move");
    let other = file(
        "planning",
        r#""id":"planning.card.other","capability":"card.move""#,
    );
    assert_eq!(
        first_id(&other, "planning"),
        Err(Refusal::Unreadable("command-id-differs".to_owned()))
    );
    assert_eq!(
        Refusal::Unreadable("command-id-differs".to_owned()).to_string(),
        "command-set-unreadable: command-id-differs"
    );
}

#[test]
fn the_same_key_twice_in_one_module_refuses_the_whole_set() {
    // already refused when the file is read (no set is made), not only in `add`
    assert_eq!(
        parse(
            &json_keys("planning", &["card.move", "card.move"]),
            "planning"
        ),
        Err(Refusal::Conflict("planning.card.move".into()))
    );
}

#[test]
fn a_bad_key_is_refused_as_a_bad_id() {
    let set = parse(&json_keys("planning", &["Card Move"]), "planning").unwrap();
    let why = Registry::new().add_bundled(Actor::Person, set).unwrap_err();
    assert!(matches!(&why[0], Refusal::BadId(_)), "{why:?}");
}
