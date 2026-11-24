//! `--new-instance` (draft N3, package W-1): further windows of a program take the places `window-2 .. window-9`; the lock of
//! `fs_rpc::bind_socket` decides, never a window. Every test has its own temp folder as the runtime folder ([TESTISOLATION]).
//!
//! Test list: a two claims side by side (`#2`, `#3`, the first window keeps its place); b a start without the flag hands over to
//! `.window`; c `--new-instance` with `--picture` takes no place and hands nothing over.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use fs_starter_host::instance::{Claim, InstanceTimes, claim_with};
use fs_starter_host::slots::{claim_next, label};

mod support;

fn at(runtime: &Path, file: &str) -> PathBuf {
    runtime.join("freesynergy").join(format!("{file}.sock"))
}

fn times() -> InstanceTimes {
    InstanceTimes {
        connect: Duration::from_millis(600),
        answer: Duration::from_millis(600),
    }
}

fn any(_: &[String]) -> bool {
    true
}

#[test]
fn a_two_new_windows_live_next_to_the_first() {
    let dir = tempfile::tempdir().unwrap();
    let Ok(Claim::First(first)) =
        claim_with(Some(&at(dir.path(), "tasks.window")), &[], times(), any)
    else {
        panic!("first window")
    };
    let of = |k: u8| Some(at(dir.path(), &format!("tasks.window-{k}")));
    let (k2, w2) = claim_next(&of, any).unwrap().unwrap();
    let (k3, w3) = claim_next(&of, any).unwrap().unwrap();
    assert_eq!((k2, k3), (2, 3));
    assert_eq!(label("tasks", k3), "tasks#3");
    drop(w2);
    // a freed place is the smallest free one again
    assert_eq!(claim_next(&of, any).unwrap().unwrap().0, 2);
    drop((first, w3));
}

#[test]
fn b_a_start_without_the_flag_hands_over_to_the_first_window() {
    let runtime = tempfile::tempdir().unwrap();
    let Ok(Claim::First(first)) =
        claim_with(Some(&at(runtime.path(), "tasks.window")), &[], times(), any)
    else {
        panic!("first window")
    };
    let of = |k: u8| Some(at(runtime.path(), &format!("tasks.window-{k}")));
    let (_, w2) = claim_next(&of, any).unwrap().unwrap();
    let out = host(
        runtime.path(),
        &[&support::tasks(), "--picture", "/nonexistent/never.png"],
    );
    let err = support::text(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{err}");
    assert!(err.contains("instance-handed-over"), "{err}");
    assert!(first.starts.recv_timeout(Duration::from_secs(2)).is_ok());
    assert!(w2.starts.try_recv().is_err(), "window 2 got nothing");
}

#[test]
fn c_new_instance_with_a_picture_takes_no_place() {
    let runtime = tempfile::tempdir().unwrap();
    let Ok(Claim::First(first)) =
        claim_with(Some(&at(runtime.path(), "tasks.window")), &[], times(), any)
    else {
        panic!("first window")
    };
    let png = runtime.path().join("p.png");
    let out = host(
        runtime.path(),
        &[
            &support::tasks(),
            "--new-instance",
            "--picture",
            png.to_str().unwrap(),
        ],
    );
    let err = support::text(&out.stderr);
    assert!(!err.contains("instance-handed-over"), "{err}");
    assert!(first.starts.try_recv().is_err(), "nothing was handed over");
    assert!(
        !at(runtime.path(), "tasks.window-2").exists(),
        "no place taken"
    );
}

fn host(runtime: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fs-starter"))
        .args(args)
        .env("HOME", runtime)
        .env("XDG_DATA_HOME", runtime)
        .env("XDG_CONFIG_HOME", runtime)
        .env("XDG_RUNTIME_DIR", runtime)
        .output()
        .unwrap()
}
