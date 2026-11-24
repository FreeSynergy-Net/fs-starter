//! SW-5: the HOST binds `<module>.<key>` from the contract key of the file (`capability`); the id never comes from the module.
//!
//! Tests: derivation from the folder, own prefix equal to the bare key, a foreign prefix never accepted, a given `id` must be the
//! derived one, a duplicate key refuses the set. Mutants: the id is the key as written (no prefix added); the prefix is taken from the
//! `module` field instead of the folder; a foreign prefix is taken as it is; a given `id` is ignored; the own prefix is doubled;
//! `app.` is let through.

mod support;
use support::sets::json_keys;

use fs_starter_host::registry::{Actor, Refusal, Registry, parse};

fn file(module: &str, command: &str) -> String {
    format!(
        r#"{{"module":"{module}","version":"0.1","contract":"{module}","scope":"own-contract","commands":[{{{command},"call":"x","access":"read","help":{{"topic":"t","text":{{"en":"e","de":"d"}}}}}}]}}"#
    )
}

fn first_id(text: &str, folder: &str) -> Result<String, Refusal> {
    parse(text, folder).map(|s| s.items[0].command.id.as_str().to_owned())
}

#[test]
fn a_bare_key_gets_the_prefix_of_the_folder_and_the_own_prefix_is_the_same_command() {
    let bare = file("planning", r#""capability":"card.move""#);
    let own = file("planning", r#""capability":"planning.card.move""#);
    assert_eq!(first_id(&bare, "planning").unwrap(), "planning.card.move");
    assert_eq!(first_id(&own, "planning").unwrap(), "planning.card.move");
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
    for keys in [
        ["card.move", "card.move"],
        ["card.move", "planning.card.move"],
    ] {
        let set = parse(&json_keys("planning", &keys), "planning").unwrap();
        let why = Registry::new().add_bundled(Actor::Person, set).unwrap_err();
        assert_eq!(
            why,
            [Refusal::Conflict("planning.card.move".into())],
            "{keys:?}"
        );
    }
}

#[test]
fn a_bad_key_is_refused_as_a_bad_id() {
    let set = parse(&json_keys("planning", &["Card Move"]), "planning").unwrap();
    let why = Registry::new().add_bundled(Actor::Person, set).unwrap_err();
    assert!(matches!(&why[0], Refusal::BadId(_)), "{why:?}");
}
