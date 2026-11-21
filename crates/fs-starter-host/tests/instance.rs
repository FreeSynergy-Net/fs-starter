//! The single instance of a program window (ADR 0097): the first start holds the place, a second start of the same program hands
//! its words over. Every test has its own temp folder as the runtime folder (never the real one, [TESTISOLATION]).
#![cfg(unix)]

use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use fs_starter_host::instance::{
    Claim, InstanceTimes, claim, claim_with, endpoint_for, words_are_a_start,
};

mod support;

fn words(w: &[&str]) -> Vec<String> {
    w.iter().map(ToString::to_string).collect()
}

fn times() -> InstanceTimes {
    InstanceTimes {
        connect: Duration::from_millis(600),
        answer: Duration::from_millis(600),
    }
}

/// The endpoint `<runtime>/freesynergy/<name>.window.sock` below a temp folder, the shape `fs_rpc::default_socket` builds.
fn at(runtime: &Path, name: &str) -> PathBuf {
    runtime
        .join("freesynergy")
        .join(format!("{name}.window.sock"))
}

/// Everything a start of a test takes: accepts every words (the parser is tested on its own below).
fn any(_: &[String]) -> bool {
    true
}

fn first(c: Result<Claim, String>) -> fs_starter_host::instance::Guard {
    match c {
        Ok(Claim::First(g)) => g,
        Ok(Claim::HandedOver) => panic!("handed over, expected First"),
        Ok(Claim::Off(c)) => panic!("off: {c}"),
        Err(e) => panic!("error: {e}"),
    }
}

#[test]
fn t1_the_second_start_hands_its_words_to_the_first() {
    let dir = tempfile::tempdir().unwrap();
    let path = at(dir.path(), "tasks");
    let guard = first(claim_with(Some(&path), &[], times(), any));
    let second = claim_with(Some(&path), &words(&["--x", "a"]), times(), any);
    assert!(matches!(second, Ok(Claim::HandedOver)));
    let got = guard.starts.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(got, words(&["--x", "a"]));
    assert!(guard.starts.try_recv().is_err(), "exactly one start");
}

#[test]
fn t2_two_programs_are_both_first() {
    let dir = tempfile::tempdir().unwrap();
    let a = first(claim(Some(&at(dir.path(), "tasks")), &[], times()));
    let b = first(claim(Some(&at(dir.path(), "forms")), &[], times()));
    drop((a, b));
}

#[test]
fn t3_after_the_guard_is_dropped_a_new_claim_is_first() {
    let dir = tempfile::tempdir().unwrap();
    let path = at(dir.path(), "tasks");
    drop(first(claim(Some(&path), &[], times())));
    assert!(!path.exists(), "the socket is gone with the guard");
    drop(first(claim(Some(&path), &[], times())));
}

#[test]
fn t4_a_stale_socket_file_without_a_process_is_first() {
    let dir = tempfile::tempdir().unwrap();
    let path = at(dir.path(), "tasks");
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(path.parent().unwrap())
        .unwrap();
    drop(UnixListener::bind(&path).unwrap()); // the file stays, nobody listens
    assert!(path.exists());
    drop(first(claim(Some(&path), &[], times())));
}

#[test]
fn t5_a_name_that_is_a_path_is_refused_and_no_path_is_built() {
    for bad in ["../x", "a/b", "", "fs-tasks", "A", "a.b", "x/../y"] {
        let e = endpoint_for(bad).unwrap_err();
        assert!(e.starts_with("instance-name-invalid"), "{bad}: {e}");
    }
    // a good name is only checked, the endpoint depends on the runtime folder of the process
    assert!(endpoint_for("tasks").is_ok());
}

#[test]
fn t6_a_folder_others_can_write_in_gives_an_error_and_no_first() {
    let dir = tempfile::tempdir().unwrap();
    let open = dir.path().join("open");
    std::fs::create_dir(&open).unwrap();
    std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o777)).unwrap();
    let r = claim(Some(&open.join("tasks.window.sock")), &[], times());
    match r {
        Err(e) => assert!(e.starts_with("instance-endpoint-unusable"), "{e}"),
        Ok(_) => panic!("a folder with mode 0777 must not give a place"),
    }
}

#[test]
fn t8_a_first_instance_that_never_answers_ends_with_not_answering() {
    let dir = tempfile::tempdir().unwrap();
    let path = at(dir.path(), "tasks");
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(path.parent().unwrap())
        .unwrap();
    let _hung = UnixListener::bind(&path).unwrap(); // listens, never accepts
    let t = InstanceTimes {
        connect: Duration::from_millis(300),
        answer: Duration::from_millis(300),
    };
    let started = std::time::Instant::now();
    let r = claim(Some(&path), &words(&["--picture", "x.png"]), t);
    match r {
        Err(e) => assert_eq!(e, "instance-not-answering"),
        Ok(_) => panic!("a hung instance must not give First or HandedOver"),
    }
    assert!(started.elapsed() >= Duration::from_millis(300));
    assert!(path.exists(), "nothing is killed or removed");
}

#[test]
fn t10_the_first_instance_refuses_words_the_parser_does_not_know() {
    let dir = tempfile::tempdir().unwrap();
    let path = at(dir.path(), "tasks");
    let guard = first(claim(Some(&path), &[], times()));
    let r = claim(Some(&path), &words(&["--x", "a"]), times());
    match r {
        Err(e) => assert!(e.starts_with("instance-words-refused"), "{e}"),
        Ok(_) => panic!("unknown words must be refused"),
    }
    assert!(guard.starts.try_recv().is_err(), "nothing was queued");
    let good = words(&[&support::tasks(), "--sample-data"]);
    assert!(words_are_a_start(&good));
    assert!(matches!(
        claim(Some(&path), &good, times()),
        Ok(Claim::HandedOver)
    ));
    assert_eq!(
        guard.starts.recv_timeout(Duration::from_secs(2)).unwrap(),
        good
    );
}

#[test]
fn t11_a_word_too_long_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = at(dir.path(), "tasks");
    let guard = first(claim_with(Some(&path), &[], times(), any));
    let long = "x".repeat(5000);
    let r = claim_with(Some(&path), &[long], times(), any);
    assert!(r.is_err());
    assert!(guard.starts.try_recv().is_err());
}

#[test]
fn t7_without_a_runtime_folder_the_program_starts_and_says_single_instance_off() {
    let home = tempfile::tempdir().unwrap();
    let png = home.path().join("a.png");
    let out = Command::new(env!("CARGO_BIN_EXE_fs-starter"))
        .args([&support::tasks(), "--picture", png.to_str().unwrap()])
        .env("HOME", home.path())
        .env("XDG_DATA_HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .env_remove("XDG_RUNTIME_DIR")
        .output()
        .unwrap();
    let err = support::text(&out.stderr);
    assert!(err.contains("single-instance-off"), "{err}");
    assert!(!err.contains("instance-handed-over"), "{err}");
    assert!(
        !Path::new("/tmp/freesynergy")
            .join("tasks.window.sock")
            .exists()
    );
}

#[test]
fn t9_a_second_start_of_the_picture_against_a_held_package_hands_over_and_exits_0() {
    let runtime = tempfile::tempdir().unwrap();
    let guard = first(claim(Some(&at(runtime.path(), "tasks")), &[], times()));
    let png = runtime.path().join("never.png");
    let out = Command::new(env!("CARGO_BIN_EXE_fs-starter"))
        .args([&support::tasks(), "--picture", png.to_str().unwrap()])
        .env("HOME", runtime.path())
        .env("XDG_DATA_HOME", runtime.path())
        .env("XDG_CONFIG_HOME", runtime.path())
        .env("XDG_RUNTIME_DIR", runtime.path())
        .output()
        .unwrap();
    let err = support::text(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{err}");
    assert!(err.contains("instance-handed-over"), "{err}");
    assert!(!png.exists(), "no picture: the second start did not draw");
    let got = guard.starts.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(got.contains(&"--picture".to_owned()), "{got:?}");
}
