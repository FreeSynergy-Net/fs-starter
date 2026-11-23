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
    assert!(!out.contains("red") && !out.contains("yellow"), "{out}");
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
    let forms = r#"[{"name":"checklist","version":"1.0","type":"Group",
        "map":{"items":"Group","done":"closed","label":"name"},"fields":{"closed":"bool","name":"text"}}]"#;
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
