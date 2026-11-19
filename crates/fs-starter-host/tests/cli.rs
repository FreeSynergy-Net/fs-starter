//! The words and exit codes of the host binary (no window is opened: `--check`, `--version`, `--help`, wrong words).
//!
//! Tests: example-based; the exit codes are the ones of `fs_starter::Exit`; the line `skeleton-part-pending` is said once.
//! Mutants (see the log of the run): usage exit 2 -> 1; `--version` prints another name; the pending line left out; the line
//! printed for `--version` too; `--help` without the host's own words.

mod support;
use support::{host, tasks, text};

use fs_starter::Exit;

#[test]
fn version_prints_the_name_and_the_version_and_ends_with_done() {
    let out = host(&["--version"], "light");
    assert_eq!(out.status.code(), Some(i32::from(Exit::DONE.code())));
    assert_eq!(
        text(&out.stdout).trim(),
        format!("fs-starter {}", env!("CARGO_PKG_VERSION"))
    );
    assert!(
        !text(&out.stderr).contains("skeleton-part-pending"),
        "no pending line for --version"
    );
    assert_eq!(host(&["-V"], "light").stdout, out.stdout, "-V is --version");
}

#[test]
fn help_names_the_host_and_the_words_of_the_window_host() {
    let out = host(&["--help"], "light");
    assert_eq!(out.status.code(), Some(i32::from(Exit::DONE.code())));
    let help = text(&out.stdout);
    assert!(help.starts_with("fs-starter "), "{help}");
    assert!(help.contains("--sample-data"), "{help}");
    assert!(help.contains("--style"), "{help}");
    assert!(
        !text(&out.stderr).contains("skeleton-part-pending"),
        "no pending line for --help"
    );
}

#[test]
fn wrong_words_end_with_the_usage_code_and_a_message() {
    for args in [&["--no-such-option"][..], &[][..], &["--description"][..]] {
        let out = host(args, "light");
        assert_eq!(
            out.status.code(),
            Some(i32::from(Exit::USAGE.code())),
            "{args:?}: {}",
            text(&out.stderr)
        );
        let err = text(&out.stderr);
        assert!(err.starts_with("fs-starter: "), "{err}");
        assert!(err.contains("--help"), "{err}");
        assert_eq!(text(&out.stdout), "", "wrong words print nothing on stdout");
    }
}

#[test]
fn a_description_is_checked_without_a_window_and_the_pending_parts_are_named_once() {
    let out = host(&["--description", &tasks(), "--check"], "light");
    let err = text(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(i32::from(Exit::DONE.code())),
        "{err}"
    );
    assert_eq!(
        text(&out.stdout).trim(),
        "fs-starter: the description of the frontend board is there"
    );
    assert_eq!(err.matches("skeleton-part-pending").count(), 1, "{err}");
    for part in ["single-instance", "session", "update", "help", "mcp"] {
        assert!(err.contains(part), "{part} not named: {err}");
    }
    // the description is there; the program name is nowhere in the host's words
    assert!(!err.to_lowercase().contains("tasks"), "{err}");
}

#[test]
fn a_bare_path_is_the_description_and_a_missing_one_is_not_a_usage_error() {
    let bare = host(&[&tasks(), "--check"], "light");
    assert_eq!(bare.status.code(), Some(0), "{}", text(&bare.stderr));
    let gone = format!("{}/no-such-folder", tasks());
    let out = host(&["--description", &gone, "--check"], "light");
    assert_eq!(
        out.status.code(),
        Some(i32::from(Exit::FAILED.code())),
        "a missing description is a failure (the window would show it), not a contract problem: {}",
        text(&out.stderr)
    );
    assert!(text(&out.stderr).contains("no-such-folder"));
}

#[test]
fn without_a_server_and_without_a_description_the_check_ends_with_the_contract_code() {
    let gone = format!("{}/no-such-forms.sock", tasks());
    let out = host(
        &[
            "--frontend",
            "board",
            "--extends",
            "net.example.Program",
            "--forms-socket",
            &gone,
            "--check",
        ],
        "light",
    );
    assert_eq!(
        out.status.code(),
        Some(i32::from(Exit::CONTRACT.code())),
        "the starter then shows the standard frontend: {}",
        text(&out.stderr)
    );
    assert!(text(&out.stderr).starts_with("fs-starter: skeleton-part-pending"));
    assert!(text(&out.stderr).contains("fs-starter: "), "own name");
}

#[test]
fn the_pending_parts_are_listed_in_one_place() {
    assert_eq!(fs_starter_host::CODE, "skeleton-part-pending");
    assert_eq!(fs_starter_host::PENDING.len(), 5);
    assert!(fs_starter_host::note().contains(fs_starter_host::CODE));
}
