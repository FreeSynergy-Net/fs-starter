//! K19-K27, K30 of ENTWURF section 5 (and K31, the O4 case: a hand-made manifest is never trusted silently).
mod support;

use std::io::ErrorKind;
use std::time::Duration;

use fs_starter::{Exit, Failure, Level, Listing, Note, StarterCode, TakenOverNote, Trust, Up};
use fs_types::coded::{Coded, Param};
use support::{FailingWriter, OpenAnswer, World, code_text, fid, found, os};

fn code(exit: Exit) -> u8 {
    exit.code()
}

/// `--version` writes `fs-<name> <version>`, exit 0, no server, no frontend.
#[test]
fn version_output() {
    let w = World::new();
    let (exit, out) = w.go_text(&["--version"]);
    assert_eq!((code(exit), out.as_str()), (0, "fs-demo 0.0.1\n"));
    assert_eq!(w.gate.calls.get(), 0);
    assert!(w.opened().is_empty() && w.launcher.checked.borrow().is_empty());
}

/// K19: a hook that stops ends the start with its exit and code; one that goes gets the end and `resumed`.
#[test]
fn k19_hooks() {
    let mut w = World::new();
    let stop = Failure {
        exit: Exit::of_frontend(4),
        code: Coded::new(support::DemoCode::X).erase(),
    };
    w.hooks.push(support::FakeHook {
        stop: Some(stop.clone()),
        ..support::FakeHook::go()
    });
    assert_eq!(code(w.go(&[])), 4);
    assert!(w.opened().is_empty(), "nothing opens after a stop");
    let note = w.tell.note("demo-error-x").expect("told");
    assert_eq!(note.level, Level::Error);
    assert_eq!(note.code, stop.code);

    let mut w = World::new();
    w.hooks.push(support::FakeHook::go());
    assert_eq!(code(w.go(&["--resumed", "tok9"])), 0);
    let before = w.hooks[0].before.borrow().clone();
    assert_eq!(before, vec![(false, true, "simple".to_owned())]);
    let after = w.hooks[0].after.borrow().clone();
    assert_eq!(after.len(), 1);
    assert_eq!((after[0].0, after[0].2), (0, true));
    assert!(after[0].1 < Duration::from_secs(5));
    let opened = w.opened();
    assert_eq!(
        opened[0].1,
        os(&["--socket", "/run/demo.sock", "--resumed", "tok9"])
    );
    // The token is in no note.
    assert!(
        w.tell
            .0
            .borrow()
            .iter()
            .all(|n| !n.code.to_string().contains("tok9"))
    );
}

/// K20: no `--socket` and no default: `no-socket`, exit 1, the gate is not asked.
#[test]
fn k20_no_socket() {
    let mut w = World::new();
    w.default_socket = None;
    assert_eq!(code(w.go(&[])), 1);
    assert_eq!(w.tell.keys(), ["starter-error-no-socket"]);
    assert_eq!(w.gate.calls.get(), 0);
    // `--socket` makes it work again.
    assert_eq!(code(w.go(&["--socket", "/s"])), 0);
    assert_eq!(w.opened()[0].1, os(&["--socket", "/s"]));
}

/// K21: a taken-over server is a hint with `pid`, `saved`, `how`; the start goes on.
#[test]
fn k21_taken_over() {
    let mut w = World::new();
    w.gate.answer = Ok(Up {
        taken_over: Some(TakenOverNote {
            pid: 42,
            saved: "saved",
            how: "sigterm",
        }),
    });
    assert_eq!(code(w.go(&[])), 0);
    let n = w
        .tell
        .note("starter-error-server-taken-over")
        .expect("told");
    assert_eq!(n.level, Level::Hint);
    assert_eq!(n.code.param("pid"), Some(&Param::Number(42)));
    assert_eq!(code_text(&n.code, "saved").as_deref(), Some("saved"));
    assert_eq!(code_text(&n.code, "how").as_deref(), Some("sigterm"));
    assert_eq!(w.opened().len(), 1);
}

/// K22: a refused manifest file is a hint; the standard frontend still opens.
#[test]
fn k22_refused_listing() {
    let mut w = World::new();
    let refused = Coded::new(StarterCode::ManifestInvalid)
        .with_text("file", "/x/bad.metainfo.xml")
        .with_text("field", "xml");
    w.manifests.0 = Listing {
        found: vec![found("simple", Trust::Packaged)],
        refused: vec![refused.clone()],
    };
    assert_eq!(code(w.go(&[])), 0);
    let n = w.tell.note("starter-error-manifest-invalid").expect("told");
    assert_eq!((n.level, n.code), (Level::Hint, refused.erase()));
    assert_eq!(w.opened().len(), 1);
}

/// K23: a program file that cannot run is `open-failed`, exit 1, and the end hook is not called.
#[test]
fn k23_open_failed() {
    let mut w = World::new();
    w.hooks.push(support::FakeHook::go());
    w.launcher.open_answer = OpenAnswer::Err(ErrorKind::NotFound);
    assert_eq!(code(w.go(&[])), 1);
    let n = w.tell.note("starter-error-open-failed").expect("told");
    assert_eq!(n.level, Level::Error);
    assert_eq!(code_text(&n.code, "frontend").as_deref(), Some("simple"));
    assert_eq!(code_text(&n.code, "kind").as_deref(), Some("not-found"));
    assert!(w.hooks[0].after.borrow().is_empty(), "no after_end");
}

/// The standard frontend is never asked with `--check` (mutant M7); another one is, once.
#[test]
fn standard_is_not_checked() {
    let w = World::new();
    assert_eq!(code(w.go(&[])), 0);
    assert_eq!(w.launcher.checked.borrow().len(), 0);
    let w = World::new().with_frontend("other", Trust::Packaged);
    assert_eq!(code(w.go(&["--frontend", "other"])), 0);
    assert_eq!(
        w.launcher.checked.borrow().as_slice(),
        [World::exe("other")]
    );
}

/// K24: the frontend's exit is passed on; a signal or an out-of-range value is 1.
#[test]
fn k24_frontend_exit() {
    for (answer, want) in [(7, 7), (-1, 1), (300, 1), (0, 0), (255, 255)] {
        let mut w = World::new();
        w.launcher.open_answer = OpenAnswer::Exit(answer);
        assert_eq!(code(w.go(&[])), want, "{answer}");
    }
}

/// K25: `serve` ends with 0, or with the program's own code and exit, told unchanged.
#[test]
fn k25_serve() {
    let w = World::new();
    assert_eq!(code(w.go(&["serve", "--socket", "/s"])), 0);
    assert!(w.tell.0.borrow().is_empty());
    assert_eq!(w.gate.calls.get(), 0);
    assert_eq!(code(w.go(&["serve", "--fail"])), 2);
    let n: Note = w.tell.note("demo-error-x").expect("told unchanged");
    assert_eq!(n.level, Level::Error);
}

/// K26: `mcp --as a` without an MCP entry is `mcp-missing`, exit 1; with one it gets exactly the actor.
#[test]
fn k26_mcp() {
    let mut w = World::new();
    assert_eq!(code(w.go(&["mcp", "--as", "claude-a"])), 0);
    assert_eq!(
        w.mcp.as_ref().expect("mcp").actors.borrow().as_slice(),
        ["claude-a"]
    );
    w.mcp = None;
    assert_eq!(code(w.go(&["mcp", "--as", "a"])), 1);
    assert_eq!(w.tell.keys(), ["starter-error-mcp-missing"]);
    assert_eq!(code(w.go(&["mcp"])), 2);
    assert!(
        w.tell
            .keys()
            .contains(&"starter-error-actor-missing".to_owned())
    );
}

/// `fs.mcp.enabled` off (the default): `mcp --as a` is `mcp-disabled`, exit 1, the entry is never served; on, it is served.
#[test]
fn mcp_switch_off_and_on() {
    let mut w = World::new();
    w.mcp_on = false;
    assert_eq!(code(w.go(&["mcp", "--as", "claude-a"])), 1);
    assert_eq!(w.tell.keys(), ["starter-error-mcp-disabled"]);
    assert!(w.mcp.as_ref().expect("mcp").actors.borrow().is_empty());
    // The switch is asked before the entry exists: off wins over a missing entry.
    w.mcp = None;
    assert_eq!(code(w.go(&["mcp", "--as", "a"])), 1);
    assert_eq!(
        w.tell.keys().last().map(String::as_str),
        Some("starter-error-mcp-disabled")
    );
    // On: served.
    let w = World::new();
    assert!(w.mcp_on);
    assert_eq!(code(w.go(&["mcp", "--as", "claude-a"])), 0);
    assert_eq!(w.mcp.as_ref().expect("mcp").actors.borrow().len(), 1);
}

/// Session interface: a hook's token goes to the frontend when the person gave none; a given token wins; `--fresh` gets none.
#[test]
fn session_hook_supplies_the_token() {
    let mut w = World::new();
    w.hooks.push(support::FakeHook {
        token: Some("sess7"),
        ..support::FakeHook::go()
    });
    assert_eq!(code(w.go(&[])), 0);
    assert_eq!(
        w.opened()[0].1,
        os(&["--socket", "/run/demo.sock", "--resumed", "sess7"])
    );
    // The person's token wins.
    let w2 = World::new();
    let mut w2 = w2;
    w2.hooks.push(support::FakeHook {
        token: Some("sess7"),
        ..support::FakeHook::go()
    });
    assert_eq!(code(w2.go(&["--resumed", "mine"])), 0);
    assert_eq!(
        w2.opened()[0].1,
        os(&["--socket", "/run/demo.sock", "--resumed", "mine"])
    );
    // `--fresh`: no token from the session.
    assert_eq!(code(w.go(&["--fresh"])), 0);
    assert_eq!(
        w.opened()[1].1,
        os(&["--socket", "/run/demo.sock", "--fresh"])
    );
    // The token is in no note.
    assert!(
        w.tell
            .0
            .borrow()
            .iter()
            .all(|n| !n.code.to_string().contains("sess7"))
    );
}

/// Self-update: an update hook sees how long the frontend ran against the probation (120 s) and can fall back.
#[test]
fn update_hook_sees_the_probation() {
    use fs_starter::{Ended, PROBATION};
    let mut w = World::new();
    w.hooks.push(support::FakeHook::go());
    assert_eq!(code(w.go(&[])), 0);
    assert_eq!(w.hooks[0].probation.borrow().as_slice(), [true]);
    let at = |s| Ended {
        code: 0,
        after: Duration::from_secs(s),
    };
    assert_eq!(PROBATION, Duration::from_secs(120));
    assert!(at(119).within_probation());
    assert!(!at(120).within_probation());
}

/// K27: the person's choice is checked and opened.
#[test]
fn k27_preference() {
    let mut w = World::new().with_frontend("other", Trust::Packaged);
    w.pref.0 = Some(fid("other"));
    assert_eq!(code(w.go(&[])), 0);
    assert_eq!(
        w.launcher.checked.borrow().as_slice(),
        [World::exe("other")]
    );
    assert_eq!(w.opened()[0].0, World::exe("other"));
    // A word beats the choice.
    let w2 = World::new()
        .with_frontend("other", Trust::Packaged)
        .with_frontend("third", Trust::Packaged);
    let mut w2 = w2;
    w2.pref.0 = Some(fid("other"));
    assert_eq!(code(w2.go(&["--frontend", "third"])), 0);
    assert_eq!(w2.opened()[0].0, World::exe("third"));
}

/// K30: a failing stdout is `output-failed` with the class, exit 1.
#[test]
fn k30_output_failed() {
    let w = World::new();
    let exit = w.go_out(&["--version"], &mut FailingWriter(ErrorKind::BrokenPipe));
    assert_eq!(code(exit), 1);
    let n = w.tell.note("starter-error-output-failed").expect("told");
    assert_eq!(code_text(&n.code, "kind").as_deref(), Some("broken-pipe"));
    let w = World::new();
    assert_eq!(
        code(w.go_out(&["--help"], &mut FailingWriter(ErrorKind::Other))),
        1
    );
    let n = w.tell.note("starter-error-output-failed").expect("told");
    assert_eq!(code_text(&n.code, "kind").as_deref(), Some("other"));
}

/// K31 (O4): a manifest from the user level (`LocalUnchecked`) is told as `manifest-unchecked`, a hint, and still opens.
#[test]
fn k31_local_unchecked_is_visible() {
    let mut w = World::new();
    w.manifests.0.found = vec![found("simple", Trust::LocalUnchecked)];
    assert_eq!(code(w.go(&[])), 0);
    let n = w
        .tell
        .note("starter-error-manifest-unchecked")
        .expect("never silent");
    assert_eq!(n.level, Level::Hint);
    assert_eq!(code_text(&n.code, "frontend").as_deref(), Some("simple"));
    assert_eq!(w.opened().len(), 1);
    // Packaged: no such note.
    let w = World::new();
    assert_eq!(code(w.go(&[])), 0);
    assert!(w.tell.note("starter-error-manifest-unchecked").is_none());
}

/// Standard frontend missing: `frontend-missing`, exit 1, nothing opens (fallback table, end to end); a gate failure does not stop.
#[test]
fn missing_and_gate_failure() {
    let mut w = World::new();
    w.launcher.known.clear();
    assert_eq!(code(w.go(&[])), 1);
    assert!(
        w.tell
            .keys()
            .contains(&"starter-error-frontend-missing".to_owned())
    );
    assert_eq!(w.opened().len(), 0);

    let mut w = World::new();
    w.gate.answer = Err(Coded::new(StarterCode::Server).with_text("cause", "server-unresponsive"));
    assert_eq!(code(w.go(&[])), 0);
    assert_eq!(
        w.tell.note("starter-error-server").map(|n| n.level),
        Some(Level::Error)
    );
    assert_eq!(w.gate.calls.get(), 1);
}
