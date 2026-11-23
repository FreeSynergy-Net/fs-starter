//! N2 and N4 of the review: `ProcessLauncher::find` and the time limit of `--check`.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant};

use fs_starter::{Launcher, ProcessLauncher};

/// `find` looks only in its own folders, in order, never in `PATH`, never at a path.
#[test]
fn find_is_strict() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
    fs::create_dir_all(&a).expect("mkdir");
    fs::create_dir_all(&b).expect("mkdir");
    fs::write(a.join("first"), "a").expect("write");
    fs::write(b.join("first"), "b").expect("write");
    fs::write(b.join("second"), "b").expect("write");
    fs::write(tmp.path().join("outside"), "x").expect("write");
    // a `:` in the word is an alternative data stream on Windows (S-b2): never a word, also where the file exists (Unix allows the name)
    fs::write(a.join("tool:x"), "x").expect("write");
    let l = ProcessLauncher::new(vec![a.clone(), b.clone()]);
    assert_eq!(
        l.find("first"),
        Some(a.join("first")),
        "folder 1 before folder 2"
    );
    assert_eq!(l.find("second"), Some(b.join("second")));
    for bad in [
        "",
        "/",
        "\\",
        "..",
        "../outside",
        "/usr/bin/sh",
        "a\\b",
        "tool:x",
        "C:tool",
        "missing",
    ] {
        assert_eq!(l.find(bad), None, "{bad:?}");
    }
    // `sh` is on PATH, but PATH is not searched.
    assert_eq!(l.find("sh"), None);
}

/// `--check` has a time limit: a hanging frontend is ended and answers `None`; a quick one answers its exit code.
/// One test only: writing an executable and running it must not race with another fork in this binary (ETXTBSY).
#[test]
fn check_has_a_time_limit() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let script = |name: &str, body: &str| {
        let p = tmp.path().join(name);
        fs::write(&p, format!("#!/bin/sh\n{body}\n")).expect("write");
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).expect("chmod");
        p
    };
    let hang = script("hang", "exec sleep 30");
    let quick = script("quick", "exit 3");
    let l = ProcessLauncher::new(vec![tmp.path().to_owned()])
        .with_check_limit(Duration::from_millis(300));
    let began = Instant::now();
    assert_eq!(l.check(&hang, &[]), None);
    assert!(
        began.elapsed() < Duration::from_secs(10),
        "{:?}",
        began.elapsed()
    );
    assert_eq!(l.check(&quick, &[]), Some(3));
    assert_eq!(l.check(&tmp.path().join("missing"), &[]), None);
}
