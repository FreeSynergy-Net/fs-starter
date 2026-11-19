//! The declaration of the scaffold's settings (ADR 0100 item 3): `fs.rpc.single.ask` (flag, default yes) and
//! `fs.mcp.enabled` (flag, default NO), the key names equal to the constants of fs-rpc, the words in both languages.
use std::collections::BTreeSet;

use fs_settings::{Catalog, Declaration, Declares, GroupId, Kind, Place, Value};
use fs_starter::{StarterSettings, TEXT_DE, TEXT_EN};

fn all() -> Vec<Declaration> {
    StarterSettings.declarations().expect("declares")
}

fn find(key: &str) -> Declaration {
    all()
        .into_iter()
        .find(|d| d.key.as_str() == key)
        .unwrap_or_else(|| panic!("{key} is not declared"))
}

/// Exactly the two keys, named by the constants of fs-rpc (one name, two places, kept in step here).
#[test]
fn the_two_keys_are_declared_under_the_names_of_fs_rpc() {
    let keys: Vec<String> = all().iter().map(|d| d.key.as_str().to_owned()).collect();
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&fs_rpc::SINGLE_ASK_KEY.to_owned()));
    assert!(keys.contains(&fs_rpc::MCP_ENABLED_KEY.to_owned()));
    assert_eq!(fs_rpc::SINGLE_ASK_KEY, "fs.rpc.single.ask");
    assert_eq!(fs_rpc::MCP_ENABLED_KEY, "fs.mcp.enabled");
}

/// Both are yes/no; asking is on by default, MCP is OFF by default.
#[test]
fn kind_and_default_of_each() {
    let ask = find(fs_rpc::SINGLE_ASK_KEY);
    assert_eq!(ask.kind, Kind::Flag);
    assert_eq!(ask.default, Value::Flag(true));
    let mcp = find(fs_rpc::MCP_ENABLED_KEY);
    assert_eq!(mcp.kind, Kind::Flag);
    assert_eq!(mcp.default, Value::Flag(false));
}

/// The catalog takes them (no duplicate, defaults fit their kind) and finds them by key.
#[test]
fn the_catalog_takes_both() {
    let catalog = Catalog::of(all()).expect("a catalog");
    for d in all() {
        assert_eq!(catalog.get(&d.key), Some(&d));
    }
}

/// A name and a description in German and English for each key (ADR 0057).
#[test]
fn the_words_exist_in_both_languages() {
    for d in all() {
        for (language, text) in [("en", TEXT_EN), ("de", TEXT_DE)] {
            for id in [d.label_key(), d.about_key()] {
                let has = text.lines().any(|l| l.starts_with(&format!("{id} = ")));
                assert!(has, "{language}: {id} is missing");
            }
        }
    }
}

/// The text of `id` in one file (the part after ` = `).
fn text_of(file: &str, id: &str) -> String {
    file.lines()
        .find_map(|l| l.strip_prefix(&format!("{id} = ")))
        .unwrap_or_else(|| panic!("{id} is missing"))
        .to_owned()
}

/// The placeholders `{$name}` of a message.
fn variables(text: &str) -> BTreeSet<String> {
    text.split("{$")
        .skip(1)
        .filter_map(|rest| rest.split('}').next())
        .map(|v| v.trim().to_owned())
        .collect()
}

/// The groups: the person's asking is a `system` matter, the MCP entry is `ai` (strategy decision, review K4b).
#[test]
fn the_groups_are_system_and_ai() {
    let group = |name: &str| GroupId::new(name).expect("a group");
    assert_eq!(find(fs_rpc::SINGLE_ASK_KEY).group, group("system"));
    assert_eq!(find(fs_rpc::MCP_ENABLED_KEY).group, group("ai"));
}

/// The place: the question is the person's (profile, the same on every device); the MCP entry belongs to THIS device.
#[test]
fn the_mcp_entry_is_per_device_the_question_is_per_profile() {
    assert_eq!(find(fs_rpc::SINGLE_ASK_KEY).place, Place::Profile);
    assert_eq!(find(fs_rpc::MCP_ENABLED_KEY).place, Place::Device);
}

/// German and English carry the same placeholders for every id (a variable only in one language would be lost or shown raw).
#[test]
fn german_and_english_have_the_same_variables() {
    for d in all() {
        for id in [d.label_key(), d.about_key()] {
            assert_eq!(
                variables(&text_of(TEXT_EN, &id)),
                variables(&text_of(TEXT_DE, &id)),
                "{id}: the placeholders differ"
            );
        }
    }
    // the helper itself sees a placeholder (else the test above proves nothing)
    assert_eq!(variables("a {$x} b {$y}").len(), 2);
    assert!(variables("no placeholder").is_empty());
}

/// The MCP switch says the entry is open, never that an AI is allowed (ADR 0100 E7: that is a right, not a setting).
#[test]
fn the_mcp_words_say_the_entry_is_open_never_that_an_ai_is_allowed() {
    let label = d_label(fs_rpc::MCP_ENABLED_KEY);
    assert_eq!(text_of(TEXT_EN, &label), "AI entry (local) open");
    assert_eq!(text_of(TEXT_DE, &label), "KI-Eingang (lokal) offen");
    for (file, words) in [
        (TEXT_EN, ["allow", "permit"]),
        (TEXT_DE, ["erlaub", "gestatt"]),
    ] {
        let t = text_of(file, &label).to_lowercase();
        assert!(
            words.iter().all(|w| !t.contains(w)),
            "the label promises a permission: {t}"
        );
    }
}

/// The MCP switch is OFF by default and both explanations say so (ADR 0054 note 2 (a)); the single-ask explanation does not claim it.
#[test]
fn the_mcp_explanations_say_off_by_default_in_both_languages() {
    let about = |file: &str| text_of(file, &find(fs_rpc::MCP_ENABLED_KEY).about_key());
    assert!(
        about(TEXT_EN).contains("Off by default."),
        "{}",
        about(TEXT_EN)
    );
    assert!(
        about(TEXT_DE).contains("Standardmaessig aus."),
        "{}",
        about(TEXT_DE)
    );
    assert!(
        !about(TEXT_DE).contains("Standardmaessig an"),
        "{}",
        about(TEXT_DE)
    );
}

fn d_label(key: &str) -> String {
    find(key).label_key()
}
