//! Tolerant reader (ADR 0104 E3, card K2/K5): a capability the contract list of this host does not know is a command of a NEWER module:
//! only that command is left out and said (`--check-system`: yellow `command-refused`), the module stays. A real flaw of the file stays
//! red and refuses the module. A module below the need of a program is told with its package and "update".
//!
//! Runs on unix and Windows: no service is started (`Service::None`).

mod support;
use support::install::{GOOD_FORMS, Install, Service, groups};
use support::sets::{json_keys, modules_dir};

use fs_starter_host::registry::{Refusal, Registry};

fn one(module: &str, file: &str) -> (Registry, Vec<(String, Vec<Refusal>)>) {
    let dir = modules_dir(&[(module, file)]);
    let mut r = Registry::new();
    let refused = r.load_bundled(dir.path()).unwrap();
    (r, refused)
}

#[test]
fn one_unknown_capability_leaves_out_that_command_and_keeps_the_module() {
    let (r, refused) = one(
        "capture",
        &json_keys("capture", &["task.create", "task.steal"]),
    );
    assert!(refused.is_empty(), "{refused:?}");
    let ids: Vec<String> = r.lines();
    assert!(
        ids.iter().any(|l| l.starts_with("capture.task.create")),
        "{ids:?}"
    );
    assert!(!ids.iter().any(|l| l.contains("task.steal")), "{ids:?}");
    assert_eq!(
        r.skipped(),
        [("capture".to_owned(), "task.steal".to_owned())]
    );
}

#[test]
fn a_file_with_no_known_command_is_refused_as_before() {
    let (_, refused) = one("capture", &json_keys("capture", &["task.steal"]));
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert!(
        refused[0].1[0]
            .to_string()
            .contains("command-capability-unknown")
    );
}

#[test]
fn a_flaw_of_the_file_stays_a_refusal_of_the_module() {
    for bad in ["{not json", r#"{"module":"capture","version":3}"#] {
        let (r, refused) = one("capture", bad);
        assert_eq!(refused.len(), 1, "{bad}: {refused:?}");
        assert_eq!(r.skipped(), []);
    }
}

#[test]
fn check_system_says_each_left_out_command_yellow_with_its_cause() {
    let i = Install::new();
    i.module("groups", "0.1.0", GOOD_FORMS, &Service::None);
    let file = i.folder("groups").join("catalog").join("commands.json");
    std::fs::write(&file, json_keys("groups", &["task.create", "task.steal"])).unwrap();
    let out = i.run(&["--check-system"], "light", &[]);
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        text.contains(
            "yellow command-refused module=\"groups\" command=\"task.steal\" why=capability-unknown"
        ),
        "{text}"
    );
    assert_eq!(text.matches("command-refused").count(), 1, "{text}");
    assert!(!text.contains("module-refused"), "{text}");
    assert!(
        text.contains("module-down module=groups"),
        "the module is known: {text}"
    );
}

#[test]
fn a_module_below_the_need_names_its_package_and_says_update_a_newer_one_does_not() {
    for (version, old) in [("0.0.0", true), ("0.2.0", false)] {
        let i = Install::new();
        i.module("groups", version, GOOD_FORMS, &Service::None);
        let out = i.run(&["--check", "--description", &groups()], "light", &[]);
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        assert!(text.contains("red host-notice-data-contract"), "{text}");
        assert_eq!(
            text.contains("package=fs-module-groups action=update"),
            old,
            "{version}: {text}"
        );
    }
}
