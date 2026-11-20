//! E2 host duties: the MCP switch, the session token interface, the update probation.
mod support;

use std::time::Duration;

use fs_starter::{Ended, PROBATION};
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
