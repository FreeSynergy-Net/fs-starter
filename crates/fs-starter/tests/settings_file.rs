//! G4 (ENTWURF einstellungen-als-daten): the file `catalog/settings.json` gives the SAME declarations as the Rust code it replaced
//! (`OLD_*` below are those declarations, written out by hand before the Rust code was deleted), plus the new `fs.open.when-open`.
//! Every choice of `fs.open.when-open` has a `WhenOpen` and its words in both languages.
use fs_settings::{Declaration, Declares, GroupId, Kind, Owner, OwnerKind, SettingKey, Value};
use fs_starter::{StarterSettings, TEXT_DE, TEXT_EN};

fn key(owner: &str, local: &str) -> SettingKey {
    SettingKey::new(
        &Owner::new(OwnerKind::Framework, owner).expect("owner"),
        local,
    )
    .expect("key")
}

/// The old `fs.rpc.single.ask`: flag, yes by default, group `system`, profile.
fn old_single_ask() -> Declaration {
    Declaration::new(
        key("rpc", "single.ask"),
        Kind::Flag,
        Value::Flag(true),
        GroupId::new("system").expect("group"),
    )
}

/// The old `fs.mcp.enabled`: flag, NO by default, group `ai`, per device.
fn old_mcp_enabled() -> Declaration {
    Declaration::new(
        key("mcp", "enabled"),
        Kind::Flag,
        Value::Flag(false),
        GroupId::new("ai").expect("group"),
    )
    .per_device()
}

fn declared(name: &str) -> Declaration {
    StarterSettings
        .declarations()
        .expect("declares")
        .into_iter()
        .find(|d| d.key.as_str() == name)
        .unwrap_or_else(|| panic!("{name} is not declared"))
}

#[test]
fn g4_the_file_gives_the_declarations_of_the_old_code() {
    assert_eq!(declared("fs.rpc.single.ask"), old_single_ask());
    // the old code did not say `human-only`; the file does (ADR 0091 item 5), the rest is equal
    let mcp = declared("fs.mcp.enabled");
    assert_eq!(mcp.clone().human_only(), old_mcp_enabled().human_only());
    assert_eq!(mcp.default, Value::Flag(false), "MCP is off by default");
}

#[test]
fn g4_the_new_key_is_a_choice_of_the_four_words_and_asks_by_default() {
    let d = declared("fs.open.when-open");
    assert_eq!(d.default, Value::Choice("ask".to_owned()));
    let Kind::Choice { keys: list } = &d.kind else {
        panic!("a choice, got {:?}", d.kind)
    };
    assert_eq!(list, &["ask", "switch", "tab", "window"]);
}

#[test]
fn g4_every_choice_has_a_when_open_and_words_in_both_languages() {
    let d = declared("fs.open.when-open");
    let Kind::Choice { keys: list } = &d.kind else {
        panic!("a choice")
    };
    for choice in list {
        let word = fs_starter::WhenOpen::from_choice(choice)
            .unwrap_or_else(|| panic!("no WhenOpen for {choice}"));
        assert_eq!(word.as_choice(), choice);
        let id = d.choice_key(choice);
        for (language, text) in [("en", TEXT_EN), ("de", TEXT_DE)] {
            assert!(
                text.lines().any(|l| l.starts_with(&format!("{id} = "))),
                "{language}: {id} is missing"
            );
        }
    }
    assert_eq!(fs_starter::WhenOpen::ALL.len(), list.len());
    assert_eq!(fs_starter::WhenOpen::from_choice("nonsense"), None);
}
