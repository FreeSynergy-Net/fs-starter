//! The declaration of the scaffold's settings (ADR 0100 item 3): `fs.rpc.single.ask` (flag, default yes) and
//! `fs.mcp.enabled` (flag, default NO), the key names equal to the constants of fs-rpc, the words in both languages.
use fs_settings::{Catalog, Declaration, Declares, Kind, Value};
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
