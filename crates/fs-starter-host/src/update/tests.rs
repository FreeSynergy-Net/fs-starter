//! The hook as a whole: the marker over two starts, and a real replacement of a file in a temporary folder.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use fs_starter::{Ended, FrontendId, HookOutcome, ProgramName, StartCx, StartHook};

use super::*;

struct Quiet;
impl Transaction for Quiet {
    fn busy(&self) -> bool {
        false
    }
}

#[derive(Clone, Default)]
struct Rec(Arc<Mutex<Vec<String>>>);
impl Act for Rec {
    fn end_module(&self, m: &str) -> Result<Option<fs_rpc::Saved>, String> {
        self.0.lock().unwrap().push(format!("end {m}"));
        Ok(Some(fs_rpc::Saved::Saved))
    }
    fn restart_own(&self) -> Result<(), String> {
        self.0.lock().unwrap().push("restart".into());
        Ok(())
    }
}

fn lines() -> (Say, Arc<Mutex<Vec<String>>>) {
    let got = Arc::new(Mutex::new(Vec::new()));
    let g = Arc::clone(&got);
    (
        Arc::new(move |l: &str| g.lock().unwrap().push(l.to_owned())),
        got,
    )
}

fn hook(dir: &std::path::Path, files: Vec<Watched>, rec: &Rec, say: Say) -> UpdateHook {
    let w = Watcher::new(
        files,
        Box::new(Quiet),
        Box::new(NoSignal),
        Box::new(rec.clone()),
        Duration::ZERO,
    );
    UpdateHook::with("starter", Some(Marker::in_folder(dir, "starter")), w, say)
}

fn cx<'a>(p: &'a ProgramName, f: &'a FrontendId) -> StartCx<'a> {
    StartCx {
        program: p,
        version: "0.0.8",
        frontend: f,
        fresh: false,
        resumed: false,
    }
}

#[test]
fn a_clean_end_leaves_no_mark_and_a_failed_end_is_told_at_the_next_start() {
    let dir = tempfile::tempdir().unwrap();
    let (p, f) = (
        ProgramName::parse("starter").unwrap(),
        FrontendId::parse("simple").unwrap(),
    );
    let rec = Rec::default();

    let (say, got) = lines();
    let h = hook(dir.path(), vec![], &rec, say);
    assert_eq!(h.before_open(&cx(&p, &f)), HookOutcome::Go);
    h.after_end(
        &cx(&p, &f),
        &Ended {
            code: 0,
            after: Duration::from_secs(1),
        },
    );
    assert!(Marker::in_folder(dir.path(), "starter").read().is_none());
    assert!(
        got.lock()
            .unwrap()
            .iter()
            .all(|l| !l.starts_with("update-crash")),
        "{:?}",
        got.lock().unwrap()
    );

    let h = hook(dir.path(), vec![], &rec, lines().0);
    h.before_open(&cx(&p, &f));
    h.after_end(
        &cx(&p, &f),
        &Ended {
            code: 3,
            after: Duration::from_secs(1),
        },
    );
    let (say, got) = lines();
    hook(dir.path(), vec![], &rec, say).before_open(&cx(&p, &f));
    let said = got.lock().unwrap().clone();
    assert!(
        said.iter()
            .any(|l| l.starts_with("update-crash program=starter version=0.0.8 exit=3")),
        "{said:?}"
    );
}

#[test]
fn a_run_that_never_reached_after_end_is_a_crash_at_the_next_start() {
    let dir = tempfile::tempdir().unwrap();
    let (p, f) = (
        ProgramName::parse("starter").unwrap(),
        FrontendId::parse("simple").unwrap(),
    );
    hook(dir.path(), vec![], &Rec::default(), lines().0).before_open(&cx(&p, &f));
    // no after_end: a panic or a signal
    let (say, got) = lines();
    hook(dir.path(), vec![], &Rec::default(), say).before_open(&cx(&p, &f));
    assert!(
        got.lock()
            .unwrap()
            .iter()
            .any(|l| l.starts_with("update-crash program=starter version=0.0.8 exit=unknown"))
    );
}

#[test]
fn a_restart_without_session_is_told_and_a_crash_after_it_is_a_crash_after_update() {
    let dir = tempfile::tempdir().unwrap();
    let (p, f) = (
        ProgramName::parse("starter").unwrap(),
        FrontendId::parse("simple").unwrap(),
    );
    let m = Marker::in_folder(dir.path(), "starter");
    let mut mark = Mark::running("0.0.7", true);
    mark.state = State::Restarted;
    m.write(&mark).unwrap();
    let (say, got) = lines();
    hook(dir.path(), vec![], &Rec::default(), say).before_open(&cx(&p, &f));
    assert!(
        got.lock()
            .unwrap()
            .iter()
            .any(|l| l.starts_with("update-session-not-restored program=starter"))
    );
    // this run (the first after the update) dies without a word
    let (say, got) = lines();
    hook(dir.path(), vec![], &Rec::default(), say).before_open(&cx(&p, &f));
    assert!(
        got.lock()
            .unwrap()
            .iter()
            .any(|l| l.starts_with("update-crash-after-update program=starter")),
        "{:?}",
        got.lock().unwrap()
    );
}

#[test]
fn a_service_file_replaced_during_the_window_is_ended_through_the_hook() {
    let dir = tempfile::tempdir().unwrap();
    let svc = dir.path().join("service");
    std::fs::write(&svc, "v1").unwrap();
    let rec = Rec::default();
    let (say, got) = lines();
    let files = vec![Watched::begin(svc.clone(), Role::Module("planning".into())).unwrap()];
    let w = Watcher::new(
        files,
        Box::new(Quiet),
        Box::new(NoSignal),
        Box::new(rec.clone()),
        Duration::ZERO,
    );
    let h = UpdateHook::with(
        "starter",
        Some(Marker::in_folder(dir.path(), "starter")),
        w,
        say,
    );
    let (p, f) = (
        ProgramName::parse("starter").unwrap(),
        FrontendId::parse("simple").unwrap(),
    );
    h.before_open(&cx(&p, &f));
    let new = dir.path().join("service.new");
    std::fs::write(&new, "v2 is longer").unwrap();
    std::fs::rename(&new, &svc).unwrap();
    // the thread polls every POLL (5 s): wait for one look, then the second acts (settle is zero)
    std::thread::sleep(POLL * 2 + Duration::from_secs(1));
    h.after_end(
        &cx(&p, &f),
        &Ended {
            code: 0,
            after: Duration::from_secs(1),
        },
    );
    assert_eq!(
        *rec.0.lock().unwrap(),
        ["end planning"],
        "{:?}",
        got.lock().unwrap()
    );
    assert!(
        got.lock()
            .unwrap()
            .iter()
            .any(|l| l.starts_with("update-module-ended module=planning saved=saved"))
    );
}
