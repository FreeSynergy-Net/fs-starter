//! Tests of the watcher: a made-up clock, a recording `Act`, files replaced by rename in a temporary folder.
//!
//! Pattern: Observer – the files are watched, what happens is a replaceable strategy.

use super::*;
use crate::update::quiet::NoSignal;
use std::sync::{Arc, Mutex};

struct Quiet(bool);
impl Transaction for Quiet {
    fn busy(&self) -> bool {
        self.0
    }
}
struct Always;
impl Calm for Always {
    fn calm(&self) -> bool {
        true
    }
}
#[derive(Clone, Default)]
struct Rec(Arc<Mutex<Vec<String>>>);
impl Act for Rec {
    fn end_module(&self, m: &str) -> Result<Option<Saved>, String> {
        self.0.lock().unwrap().push(format!("end {m}"));
        Ok(Some(Saved::Saved))
    }
    fn restart_own(&self) -> Result<(), String> {
        self.0.lock().unwrap().push("restart".into());
        Ok(())
    }
}

fn replace(path: &std::path::Path, text: &str) {
    let tmp = path.with_extension("new");
    std::fs::write(&tmp, text).unwrap();
    std::fs::rename(&tmp, path).unwrap();
}

fn setup(
    busy: bool,
    calm: bool,
) -> (
    tempfile::TempDir,
    std::path::PathBuf,
    std::path::PathBuf,
    Rec,
    Watcher,
) {
    let dir = tempfile::tempdir().unwrap();
    let own = dir.path().join("own");
    let svc = dir.path().join("service");
    std::fs::write(&own, "o1").unwrap();
    std::fs::write(&svc, "s1").unwrap();
    let files = vec![
        Watched::begin(own.clone(), Role::Own).unwrap(),
        Watched::begin(svc.clone(), Role::Module("planning".into())).unwrap(),
    ];
    let rec = Rec::default();
    let calm: Box<dyn Calm> = if calm {
        Box::new(Always)
    } else {
        Box::new(NoSignal)
    };
    let w = Watcher::new(
        files,
        Box::new(Quiet(busy)),
        calm,
        Box::new(rec.clone()),
        Duration::from_secs(10),
    );
    (dir, own, svc, rec, w)
}

#[test]
fn a_replaced_module_is_saved_and_ended_once_after_it_settled() {
    let (_d, _own, svc, rec, mut w) = setup(false, false);
    let t0 = Instant::now();
    assert!(w.step(t0).is_empty(), "nothing replaced, nothing said");
    replace(&svc, "s2 longer");
    assert!(
        w.step(t0).is_empty(),
        "first sight of the new file: not settled"
    );
    let lines = w.step(t0 + Duration::from_secs(10));
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("update-module-ended module=planning saved=saved"),
        "{lines:?}"
    );
    assert!(
        w.step(t0 + Duration::from_secs(20)).is_empty(),
        "acted once"
    );
    assert_eq!(*rec.0.lock().unwrap(), ["end planning"]);
}

#[test]
fn nothing_happens_while_a_package_transaction_holds_its_lock() {
    let (_d, _own, svc, rec, mut w) = setup(true, true);
    let t0 = Instant::now();
    replace(&svc, "s2 longer");
    for s in [0, 10, 100] {
        assert_eq!(w.step(t0 + Duration::from_secs(s)), Vec::<String>::new());
    }
    assert!(rec.0.lock().unwrap().is_empty());
}

#[test]
fn the_own_file_is_not_restarted_without_a_calm_signal_and_it_is_said_once() {
    let (_d, own, _svc, rec, mut w) = setup(false, false);
    let t0 = Instant::now();
    replace(&own, "o2 longer");
    w.step(t0);
    let lines = w.step(t0 + Duration::from_secs(10));
    assert!(lines[0].starts_with("update-restart-waits"), "{lines:?}");
    assert!(
        w.step(t0 + Duration::from_secs(20)).is_empty(),
        "said once, not every poll"
    );
    assert!(rec.0.lock().unwrap().is_empty());
}

#[test]
fn the_own_file_is_restarted_when_calm_and_the_missing_session_is_said() {
    let (_d, own, _svc, rec, mut w) = setup(false, true);
    let t0 = Instant::now();
    replace(&own, "o2 longer");
    w.step(t0);
    let lines = w.step(t0 + Duration::from_secs(10));
    assert!(
        lines[0].starts_with("update-restarted") && lines[0].contains("session is not restored"),
        "{lines:?}"
    );
    assert_eq!(*rec.0.lock().unwrap(), ["restart"]);
}

#[test]
fn the_thread_polls_and_stops() {
    let (_d, _own, svc, rec, w) = setup(false, false);
    let mut w = w;
    // a settle time of zero: the second look acts
    w.settle = Duration::ZERO;
    replace(&svc, "s2 longer");
    let run = Running::spawn(w, Duration::from_millis(20), Arc::new(|_| {}));
    std::thread::sleep(Duration::from_millis(300));
    assert!(run.stop(Duration::from_secs(2)));
    assert_eq!(*rec.0.lock().unwrap(), ["end planning"]);
}

struct Hangs;
impl Act for Hangs {
    fn end_module(&self, _: &str) -> Result<Option<Saved>, String> {
        std::thread::sleep(Duration::from_secs(3));
        Ok(None)
    }
    fn restart_own(&self) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn a_watcher_that_hangs_in_a_call_does_not_hold_the_end_of_the_window() {
    let dir = tempfile::tempdir().unwrap();
    let svc = dir.path().join("service");
    std::fs::write(&svc, "s1").unwrap();
    let files = vec![Watched::begin(svc.clone(), Role::Module("m".into())).unwrap()];
    let w = Watcher::new(
        files,
        Box::new(Quiet(false)),
        Box::new(NoSignal),
        Box::new(Hangs),
        Duration::ZERO,
    );
    replace(&svc, "s2 longer");
    let run = Running::spawn(w, Duration::from_millis(10), Arc::new(|_| {}));
    std::thread::sleep(Duration::from_millis(300));
    let began = Instant::now();
    assert!(
        !run.stop(Duration::from_millis(200)),
        "it hangs: left behind"
    );
    assert!(began.elapsed() < Duration::from_secs(2));
}
