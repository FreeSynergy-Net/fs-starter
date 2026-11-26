//! SQ-4, `--check`: the match of the forms a module declares against the form catalog and the components, WITHOUT starting a service.
//! Green = rc 0; yellow (a form no component draws, a module that is not there) = a message and rc 0; red (a required channel not
//! mapped, a version outside the range) = the contract code 3. Mutants: the match off (no red), the range off (no red).
//!
//! Runs on unix and Windows: no service is started here (`Service::None`), so nothing in these tests needs a unix socket.

mod support;
use support::install::{GOOD_FORMS, Install, Service, groups};

fn check(i: &Install) -> (Option<i32>, String) {
    let out = i.run(&["--check", "--description", &groups()], "light", &[]);
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn a_module_that_fits_is_green() {
    let i = Install::new();
    i.module("groups", "0.1.0", GOOD_FORMS, &Service::None);
    let (rc, out) = check(&i);
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("source groups 0.1.0 checked"), "{out}");
    // the one yellow that is meant: the module has no `catalog/types.bin` (pin-13b); nothing else is yellow, nothing red
    assert!(
        out.contains("yellow no-type-catalog module=groups"),
        "{out}"
    );
    let rest = out.replace("yellow no-type-catalog module=groups", "");
    assert!(!rest.contains("red") && !rest.contains("yellow"), "{out}");
    assert!(!i.started("groups"), "--check starts no service");
}

#[test]
fn a_required_channel_left_out_is_red_and_ends_with_3() {
    let i = Install::new();
    let forms = GOOD_FORMS.replace(r#""label":"name","#, "");
    i.module("groups", "0.1.0", &forms, &Service::None);
    let (rc, out) = check(&i);
    assert_eq!(rc, Some(3), "{out}");
    assert!(
        out.contains("red form-channel-unmapped") && out.contains("channel=label"),
        "{out}"
    );
}

#[test]
fn a_version_outside_the_range_is_red_with_the_contract_notice() {
    for version in ["0.2.0", "0.0.0"] {
        let i = Install::new();
        i.module("groups", version, GOOD_FORMS, &Service::None);
        let (rc, out) = check(&i);
        assert_eq!(rc, Some(3), "{version}: {out}");
        assert!(out.contains("red host-notice-data-contract"), "{out}");
        assert!(out.contains(&format!("version={version}")), "{out}");
    }
}

#[test]
fn a_module_that_is_not_there_is_yellow_and_says_so() {
    let i = Install::new();
    let (rc, out) = check(&i);
    assert_eq!(rc, Some(0), "{out}");
    assert!(
        out.contains("yellow host-notice-source-unknown") && out.contains("contract=groups"),
        "{out}"
    );
}

#[test]
fn a_form_no_component_draws_is_yellow_not_red() {
    let i = Install::new();
    // `members@1.0.0` is a form of the interface catalog (frozen) that no component of the catalog draws any more (they draw `rows`)
    let forms = r#"[{"name":"members","version":"1.0","type":"Member",
        "map":{"records":"Member","person":"person","role":"role"},"fields":{"person":"reference","role":"reference"}}]"#;
    i.module("groups", "0.1.0", forms, &Service::None);
    let (rc, out) = check(&i);
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("yellow form-no-component"), "{out}");
}

#[test]
fn a_forms_file_that_cannot_be_read_is_red_never_skipped() {
    let i = Install::new();
    i.module("groups", "0.1.0", "[{\"name\":", &Service::None);
    let (rc, out) = check(&i);
    assert_eq!(rc, Some(3), "{out}");
    assert!(
        out.contains("red host-notice-data-contract") && out.contains("cause=json"),
        "{out}"
    );
}

#[test]
fn a_module_in_a_folder_the_command_line_names_is_never_bound() {
    // border 1: `--modules DIR` is a word of `--list-commands`; a window never takes it, and the module is not known
    let i = Install::new();
    let other = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(other.path().join("groups/catalog")).unwrap();
    std::fs::write(
        other.path().join("groups/catalog/commands.json"),
        support::install::commands("groups", "0.1.0"),
    )
    .unwrap();
    std::fs::write(other.path().join("groups/catalog/forms.json"), GOOD_FORMS).unwrap();
    let out = i.run(
        &[
            "--check",
            "--description",
            &groups(),
            "--modules",
            other.path().to_str().unwrap(),
        ],
        "light",
        &[],
    );
    assert_eq!(
        out.status.code(),
        Some(2),
        "a window word list does not take --modules"
    );
    let out = i.run(&["--check", "--description", &groups()], "light", &[]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("source-unknown"));
}

#[test]
fn a_type_catalog_that_is_no_message_is_red_with_its_cause() {
    let i = Install::new();
    i.module("groups", "0.1.0", GOOD_FORMS, &Service::None);
    std::fs::write(
        i.folder("groups").join("catalog").join("types.bin"),
        b"not a message",
    )
    .unwrap();
    let (rc, out) = check(&i);
    assert_eq!(rc, Some(3), "{out}");
    assert!(
        out.contains("red type-catalog-broken module=groups cause=damaged"),
        "{out}"
    );
}

/// A module with a type catalog (the real one of person) and, if given, the packed texts of person.
fn module_with_catalog(texts: Option<&[u8]>) -> Install {
    let i = Install::new();
    i.module("groups", "0.1.0", GOOD_FORMS, &Service::None);
    let catalog = i.folder("groups").join("catalog");
    let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/person/catalog");
    std::fs::copy(data.join("types.bin"), catalog.join("types.bin")).unwrap();
    if let Some(manifest) = texts {
        std::fs::create_dir(catalog.join("texts")).unwrap();
        std::fs::write(catalog.join("texts/manifest.mf2m"), manifest).unwrap();
    }
    i
}

#[test]
fn a_type_catalog_without_texts_is_yellow_heads_would_be_word_ids() {
    let (rc, out) = check(&module_with_catalog(None));
    assert_eq!(rc, Some(0), "{out}");
    assert!(
        out.contains("yellow no-module-texts module=groups"),
        "{out}"
    );
    assert!(
        !out.contains("no-type-catalog") && !out.contains("red"),
        "{out}"
    );
}

#[test]
fn texts_that_do_not_load_are_red_with_their_cause() {
    let (rc, out) = check(&module_with_catalog(Some(b"not a manifest")));
    assert_eq!(rc, Some(3), "{out}");
    assert!(
        out.contains("red module-texts-broken module=groups cause="),
        "{out}"
    );
}
