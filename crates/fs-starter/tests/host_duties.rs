//! E2 host duties: the MCP switch, the session token interface, the update probation.
mod support;

use std::time::Duration;

use fs_starter::{Ended, McpEntry, McpOff, McpSwitch, PROBATION, Parts, StartHook, run};
use support::{FakeHook, World, os};

/// `fs.mcp.enabled` off (the default): `mcp --as a` is `mcp-disabled`, exit 1, the entry is never served; on, it is served.
#[test]
fn mcp_switch_off_and_on() {
    let mut w = World::new();
    w.mcp_on = false;
    assert_eq!(w.go(&["mcp", "--as", "claude-a"]).code(), 1);
    assert_eq!(w.tell.keys(), ["starter-error-mcp-disabled"]);
    assert!(w.mcp.as_ref().expect("mcp").actors.borrow().is_empty());
    // The switch is asked before the entry exists: off wins over a missing entry.
    w.mcp = None;
    assert_eq!(w.go(&["mcp", "--as", "a"]).code(), 1);
    assert_eq!(
        w.tell.keys().last().map(String::as_str),
        Some("starter-error-mcp-disabled")
    );
    // On: served.
    let w = World::new();
    assert!(w.mcp_on);
    assert_eq!(w.go(&["mcp", "--as", "claude-a"]).code(), 0);
    assert_eq!(w.mcp.as_ref().expect("mcp").actors.borrow().len(), 1);
}

fn with_token(token: &'static str) -> World {
    let mut w = World::new();
    w.hooks.push(FakeHook {
        token: Some(token),
        ..FakeHook::go()
    });
    w
}

/// Session interface: a hook's token goes to the frontend when the person gave none; a given token wins; `--fresh` gets none.
#[test]
fn session_hook_supplies_the_token() {
    let w = with_token("sess7");
    assert_eq!(w.go(&[]).code(), 0);
    assert_eq!(
        w.opened()[0].1,
        os(&["--socket", "/run/demo.sock", "--resumed", "sess7"])
    );
    // `--fresh`: no token from the session.
    assert_eq!(w.go(&["--fresh"]).code(), 0);
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
    // The person's token wins.
    let w = with_token("sess7");
    assert_eq!(w.go(&["--resumed", "mine"]).code(), 0);
    assert_eq!(
        w.opened()[0].1,
        os(&["--socket", "/run/demo.sock", "--resumed", "mine"])
    );
}

/// Two hooks with a token: the FIRST one wins (listing order), and the frontend counts as resumed at the end (K1).
#[test]
fn first_hook_token_wins_and_marks_resumed() {
    let mut w = with_token("first");
    w.hooks.push(FakeHook {
        token: Some("second"),
        ..FakeHook::go()
    });
    assert_eq!(w.go(&[]).code(), 0);
    assert_eq!(
        w.opened()[0].1,
        os(&["--socket", "/run/demo.sock", "--resumed", "first"])
    );
    // `before_open` saw no `--resumed` word; `after_end` sees the token that went to the frontend.
    assert!(!w.hooks[0].before.borrow()[0].1);
    assert!(w.hooks[0].after.borrow()[0].2);
    // Without any token, `resumed` stays false at the end.
    let mut n = World::new();
    n.hooks.push(FakeHook::go());
    assert_eq!(n.go(&[]).code(), 0);
    assert!(!n.hooks[0].after.borrow()[0].2);
}

/// ADR 0097: MCP is off after installation. The standard `McpOff` answers no, and `run` with it is `mcp-disabled`.
#[test]
fn mcp_off_is_the_standard_and_disables() {
    assert!(!McpOff.enabled());
    let w = World::new();
    let hooks: Vec<&dyn StartHook> = Vec::new();
    let parts = Parts {
        program: &w.program,
        gate: &w.gate,
        manifests: &w.manifests,
        preference: &w.pref,
        launcher: &w.launcher,
        tell: &w.tell,
        hooks: &hooks,
        mcp: w.mcp.as_ref().map(|m| m as &dyn McpEntry),
        mcp_switch: &McpOff,
        default_socket: w.default_socket.clone(),
    };
    let exit = run(&parts, &os(&["mcp", "--as", "claude-a"]), &mut Vec::new());
    assert_eq!(exit.code(), 1);
    assert_eq!(w.tell.keys(), ["starter-error-mcp-disabled"]);
    assert!(w.mcp.as_ref().expect("mcp").actors.borrow().is_empty());
}

/// Self-update: an update hook sees how long the frontend ran against the probation (120 s) and can fall back.
#[test]
fn update_hook_sees_the_probation() {
    let mut w = World::new();
    w.hooks.push(FakeHook::go());
    assert_eq!(w.go(&[]).code(), 0);
    assert_eq!(w.hooks[0].probation.borrow().as_slice(), [true]);
    let at = |s| Ended {
        code: 0,
        after: Duration::from_secs(s),
    };
    assert_eq!(PROBATION, Duration::from_secs(120));
    assert!(at(119).within_probation());
    assert!(!at(120).within_probation());
}
