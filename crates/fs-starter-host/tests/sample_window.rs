//! A window with `--sample-data` is no window of the program: it takes no place of the single instance (a normal start is not handed
//! over to it and does not hand its words to it), starts no update hook and writes no mark `<state>/freesynergy/<program>.update`.
//! The program is named by the blueprint folder (`tests/data/fs-tasks`, program `tasks`); the place of `tasks` is held by this test.
//! The child is killed after a short time: where a graphics adapter exists the window would stay open.
//!
//! Pattern: none - tests of one host behaviour, in the order of the control (a normal start hands over) and the case (sample data do not).
#![cfg(unix)]

mod support;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use fs_starter_host::instance::{Claim, Guard, InstanceTimes, claim};

const LIVE: Duration = Duration::from_secs(4);

fn times() -> InstanceTimes {
    InstanceTimes {
        connect: Duration::from_millis(600),
        answer: Duration::from_millis(600),
    }
}

fn held(runtime: &Path) -> Guard {
    let endpoint: PathBuf = runtime.join("freesynergy").join("tasks.window.sock");
    match claim(Some(&endpoint), &[], times()) {
        Ok(Claim::First(guard)) => guard,
        other => panic!("expected the first place: {:?}", other.map(|_| ())),
    }
}

/// Runs the host with `words` in an own runtime and state folder; ends it after `LIVE`; answers (stderr, exit code or `None` if killed).
fn run(runtime: &Path, words: &[&str]) -> (String, Option<i32>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_fs-starter"))
        .args(words)
        .env("HOME", runtime)
        .env("XDG_DATA_HOME", runtime)
        .env("XDG_CONFIG_HOME", runtime)
        .env("XDG_STATE_HOME", runtime.join("state"))
        .env("XDG_RUNTIME_DIR", runtime)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let began = Instant::now();
    let code = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status.code();
        }
        if began.elapsed() > LIVE {
            child.kill().unwrap();
            child.wait().unwrap();
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let mut err = String::new();
    std::io::Read::read_to_string(&mut child.stderr.take().unwrap(), &mut err).unwrap();
    (err, code)
}

fn marker(runtime: &Path) -> PathBuf {
    runtime
        .join("state")
        .join("freesynergy")
        .join("tasks.update")
}

/// The control: the same start WITHOUT the flag against the held place is handed over (so the setup sees a hand-over when there is one).
#[test]
fn control_a_normal_start_is_handed_over_to_the_held_place() {
    let runtime = tempfile::tempdir().unwrap();
    let guard = held(runtime.path());
    let (err, code) = run(runtime.path(), &[&support::tasks()]);
    assert!(err.contains("instance-handed-over"), "{err}");
    assert_eq!(code, Some(0), "{err}");
    assert!(guard.starts.recv_timeout(Duration::from_secs(2)).is_ok());
}

/// Test data: nothing is handed over, the held place gets no words, no mark is written.
#[test]
fn a_sample_data_window_takes_no_place_hands_nothing_over_and_writes_no_mark() {
    let runtime = tempfile::tempdir().unwrap();
    let guard = held(runtime.path());
    let (err, _) = run(runtime.path(), &[&support::tasks(), "--sample-data"]);
    assert!(!err.contains("instance-handed-over"), "{err}");
    assert!(
        guard.starts.try_recv().is_err(),
        "the held window got no words"
    );
    assert!(!marker(runtime.path()).exists(), "no mark of the program");
    // the held place is still the held one: it was not taken over or removed
    assert!(
        runtime
            .path()
            .join("freesynergy/tasks.window.sock")
            .exists(),
        "{err}"
    );
}
