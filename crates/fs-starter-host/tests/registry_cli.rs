//! The registry through the binary (`--list-commands`, `--modules`): run-time reading, refusals named, folder errors never silent.

mod support;
use support::sets::{capture, json, modules_dir};
use support::{host, text};

use fs_starter_host::registry::Registry;

#[test]
fn list_commands_reads_the_bundled_modules_at_run_time() {
    let dir = modules_dir(&[("capture", &capture())]);
    let out = host(
        &["--list-commands", "--modules", dir.path().to_str().unwrap()],
        "light",
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let lines = text(&out.stdout);
    assert!(lines.contains("app.quit\tapp\t"), "{lines}");
    assert!(
        lines.contains("capture.task.create\tcapture\t0.1.0\tnew by module capture"),
        "{lines}"
    );
    // a second module appears without a change of the host
    let more = modules_dir(&[
        ("capture", &capture()),
        ("zip", &json("zip", &["zip.task.create"])),
    ]);
    let out = host(
        &[
            "--list-commands",
            "--modules",
            more.path().to_str().unwrap(),
        ],
        "light",
    );
    assert!(
        text(&out.stdout).contains("zip.task.create\tzip\t0.1"),
        "{}",
        text(&out.stdout)
    );
}

#[test]
fn list_commands_refuses_a_module_that_takes_app_and_names_it() {
    let evil = json("capture", &["app.quit"]);
    let dir = modules_dir(&[("capture", &evil)]);
    let out = host(
        &["--list-commands", "--modules", dir.path().to_str().unwrap()],
        "light",
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("command-reserved-prefix: \"app.quit\""),
        "{}",
        text(&out.stderr)
    );
    assert_eq!(
        text(&out.stdout).matches("app.quit").count(),
        1,
        "the shell's own line only"
    );
}

#[test]
fn list_commands_without_modules_lists_the_shell_only_and_wrong_words_are_usage() {
    let dir = modules_dir(&[]);
    let out = host(
        &["--list-commands", "--modules", dir.path().to_str().unwrap()],
        "light",
    );
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(text(&out.stdout).lines().count(), 4);
    assert_eq!(
        host(&["--list-commands", "--bogus"], "light").status.code(),
        Some(2)
    );
}

#[test]
fn a_missing_or_unreadable_modules_folder_is_an_error_and_never_silent() {
    let out = host(
        &["--list-commands", "--modules", "/nonexistent-fs-modules"],
        "light",
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("modules folder"),
        "{}",
        text(&out.stderr)
    );
    // a file instead of a folder
    let file = tempfile::NamedTempFile::new().unwrap();
    let out = host(
        &[
            "--list-commands",
            "--modules",
            file.path().to_str().unwrap(),
        ],
        "light",
    );
    assert_eq!(out.status.code(), Some(1));
    // the default folder next to the binary may be missing
    assert_eq!(host(&["--list-commands"], "light").status.code(), Some(0));
    let mut r = Registry::new();
    assert!(
        r.load_bundled(std::path::Path::new("/nonexistent-fs-modules"))
            .is_err()
    );
}
