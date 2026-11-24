//! The exchange file of a module: the real file of `fs-capture`, every mandatory field, the module name = folder name.

mod support;

use fs_starter_host::registry::{Access, Actor, Refusal, Registry, parse};

/// A copy of `catalog/commands.json` of fs-capture @a9ca621.
const RAW: &str = include_str!("data/fs-capture-catalog/capture/catalog/commands.json");

/// The file as it is: id and capability both `capture.screens` (the id written out). Since the no-alias rule the capability is the
/// contract key WITHOUT the module name; the tests read this copy with the key prefix taken off (the id stays as it is).
fn real() -> String {
    RAW.replace("\"capability\": \"capture.", "\"capability\": \"")
}

#[test]
fn the_real_file_of_fs_capture_is_read_with_help_and_access() {
    let set = parse(&real(), "capture").unwrap();
    assert_eq!(
        (set.module.as_str(), set.version.as_str()),
        ("capture", "0.1.0")
    );
    assert_eq!(set.items.len(), 6);
    assert_eq!(set.items[2].access, Access::Write);
    assert_eq!(set.items[4].access, Access::Delete);
    assert_eq!(set.items[0].help.topic, "capture-screens");
    assert!(set.items[0].help.de.starts_with("Listet"));
    let mut r = Registry::new();
    r.add_bundled(Actor::Person, set).unwrap();
    let lines = r.lines();
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.contains("\tnew by module capture"))
            .count(),
        6
    );
    assert!(
        lines[6].starts_with("capture.screens\tcapture\t0.1"),
        "{}",
        lines[6]
    );
}

/// The real file with one change; every mandatory field and every rule must turn the test red when it is not kept.
fn changed(from: &str, to: &str) -> String {
    let real = real();
    assert!(real.contains(from), "{from}");
    real.replacen(from, to, 1)
}

#[test]
fn a_file_that_misses_a_mandatory_field_or_breaks_a_rule_is_unreadable() {
    let cause = |text: String, folder: &str| match parse(&text, folder) {
        Err(Refusal::Unreadable(c)) => c,
        other => format!("{other:?}"),
    };
    assert_eq!(cause("not json [".into(), "capture"), "json");
    assert_eq!(
        cause(changed("\"version\": \"0.1.0\",", ""), "capture"),
        "version"
    );
    assert_eq!(cause(changed("\"0.1.0\"", "\"0.x\""), "capture"), "version");
    assert_eq!(
        cause(changed("\"contract\": \"capture\",", ""), "capture"),
        "contract"
    );
    assert_eq!(
        cause(
            changed("\"contract\": \"capture\"", "\"contract\": \"zip\""),
            "capture"
        ),
        "contract"
    );
    assert_eq!(
        cause(changed("\"scope\": \"own-contract\",", ""), "capture"),
        "scope"
    );
    assert_eq!(cause(changed("own-contract", "all"), "capture"), "scope");
    assert_eq!(
        cause(
            changed("\"access\": \"read\"", "\"access\": \"admin\""),
            "capture"
        ),
        "command-access"
    );
    assert_eq!(
        cause(changed("\"access\": \"read\",", ""), "capture"),
        "command-access"
    );
    assert_eq!(
        cause(
            changed(
                "\"capability\": \"screens\"",
                "\"capability\": \"tasks.screens\""
            ),
            "capture"
        ),
        "command-id-differs"
    );
    assert_eq!(
        cause(
            changed(
                "\"de\": \"Listet die Bildschirme (Nummer, Name, Groesse, Skalierung, Hauptbildschirm).\"",
                "\"x\": \"y\""
            ),
            "capture"
        ),
        "command-help"
    );
    assert_eq!(
        cause(changed("\"topic\": \"capture-screens\", ", ""), "capture"),
        "command-help"
    );
    // no `id` is fine since SW-5 (the host derives it); no `capability` is not
    let no_id = parse(&changed("\"id\": \"capture.screens\", ", ""), "capture").unwrap();
    assert_eq!(no_id.items[0].command.id.as_str(), "capture.screens");
    assert_eq!(
        cause(
            changed("\"capability\": \"screens\"", "\"x\": 1"),
            "capture"
        ),
        "command-capability"
    );
    assert_eq!(
        cause(
            changed("\"id\": \"capture.screens\"", "\"id\": 7"),
            "capture"
        ),
        "command-id"
    );
    assert_eq!(cause(r#"{"module":"capture","version":"0.1.0","contract":"capture","scope":"own-contract","commands":[]}"#.into(), "capture"), "commands");
    assert_eq!(
        cause(
            r#"{"module":"capture","version":"0.1.0","contract":"capture","scope":"own-contract"}"#
                .into(),
            "capture"
        ),
        "commands"
    );
    assert_eq!(
        cause(changed("\"module\": \"capture\",", ""), "capture"),
        "module"
    );
}

#[test]
fn the_module_name_must_be_the_name_of_the_folder() {
    assert!(matches!(parse(&real(), "other"), Err(Refusal::ModuleName(n)) if n == "capture"));
}

#[test]
fn the_file_as_it_is_with_the_module_name_in_the_key_is_refused_there_is_no_alias() {
    // `capability: capture.screens` is the key `capture.screens` of the contract: the id is `capture.capture.screens`, the id written
    // in the file (`capture.screens`) is not that one
    assert_eq!(
        parse(RAW, "capture"),
        Err(Refusal::Unreadable("command-id-differs".to_owned()))
    );
}
