//! A window with `--sample-data` is no window of the program: it takes no place of the single instance (a normal start is not handed
//! over to it and does not hand its words to it), starts no update hook, writes no mark in `<state>/freesynergy` and does not ask the settings service of the person.
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

/// The marks of the program the run wrote: the names in `<state>/freesynergy` (the name is read from the run, never guessed: the first
/// window writes `tasks-1.update`, see `Marker::standard`). A missing folder is no mark.
fn marks(runtime: &Path) -> Vec<String> {
    std::fs::read_dir(runtime.join("state").join("freesynergy"))
        .map(|d| {
            d.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// A listener at the socket of the settings service of the person (`<runtime>/freesynergy/settings.sock`, the one `open_setting` asks): it
/// counts who knocks, also a client that closes again.
struct Door(std::os::unix::net::UnixListener);

fn door(runtime: &Path) -> Door {
    let path = runtime.join("freesynergy").join("settings.sock");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let listener = std::os::unix::net::UnixListener::bind(path).unwrap();
    listener.set_nonblocking(true).unwrap();
    Door(listener)
}

impl Door {
    fn callers(&self) -> usize {
        std::iter::from_fn(|| self.0.accept().ok()).count()
    }
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

/// The control of the two checks below: a normal start WITHOUT the flag and without a held place writes the mark of the run (it comes
/// from the update hook, so a mark is also the proof of the hook) and asks the settings service of the person.
#[test]
fn control_a_normal_start_writes_the_mark_and_asks_the_settings() {
    let runtime = tempfile::tempdir().unwrap();
    let (err, _) = run(runtime.path(), &[&support::tasks()]);
    let marks = marks(runtime.path());
    assert!(
        marks
            .iter()
            .any(|m| m.starts_with("tasks") && m.ends_with(".update")),
        "a mark of the program: {marks:?} / {err}"
    );
    // the line of the setting is said only when the service of the person was asked (it says `no-server` here: the door only listens)
    assert!(
        err.contains("fs.open.when-open"),
        "the settings are asked: {err}"
    );
}

/// Test data: nothing is handed over, the held place gets no words, no hook writes a mark (the folder of the marks is empty or missing,
/// whatever the file is called), and the settings service of the person is not asked.
#[test]
fn a_sample_data_window_takes_no_place_asks_no_service_and_writes_no_mark() {
    let runtime = tempfile::tempdir().unwrap();
    let guard = held(runtime.path());
    let settings = door(runtime.path());
    let (err, _) = run(runtime.path(), &[&support::tasks(), "--sample-data"]);
    assert!(!err.contains("instance-handed-over"), "{err}");
    assert!(
        guard.starts.try_recv().is_err(),
        "the held window got no words"
    );
    assert_eq!(marks(runtime.path()), Vec::<String>::new(), "no mark");
    assert!(
        !err.contains("fs.open.when-open"),
        "no settings line: {err}"
    );
    assert_eq!(settings.callers(), 0, "the settings service is not asked");
    // the held place is still the held one: it was not taken over or removed
    assert!(
        runtime
            .path()
            .join("freesynergy/tasks.window.sock")
            .exists(),
        "{err}"
    );
}
