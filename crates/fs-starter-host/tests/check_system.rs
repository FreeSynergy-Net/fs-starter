//! SB-2, `--check-system`: one line per finding (`ok|yellow|red code key=value`), exit 0 for ok and yellow, 3 for red. Services are the fake
//! module of this crate (unix sockets), so those tests are unix only. Mutants: a service that is not ended again (the socket stays),
//! a missing heartbeat counted as ok, a pending part counted as ok or red, MCP on counted as ok.

mod support;
use std::time::Duration;
use support::install::{GOOD_FORMS, Install, Service, groups};

fn run(i: &Install, args: &[&str], env: &[(&str, &str)]) -> (Option<i32>, Vec<String>) {
    let mut words = vec!["--check-system"];
    words.extend_from_slice(args);
    let out = i.run(&words, "light", env);
    let lines = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    (out.status.code(), lines)
}

fn has(lines: &[String], start: &str) -> bool {
    lines.iter().any(|l| l.starts_with(start))
}

/// Every line has the shape `ok|yellow|red code ...` (a free text never makes a line of its own).
fn shaped(lines: &[String]) {
    for l in lines {
        let mut w = l.split(' ');
        assert!(matches!(w.next(), Some("ok" | "yellow" | "red")), "{l}");
        assert!(w.next().is_some_and(|c| !c.is_empty()), "{l}");
    }
}

#[test]
fn nothing_installed_is_yellow_and_every_pending_part_is_said_yellow_not_red() {
    let i = Install::new();
    let (rc, lines) = run(&i, &[], &[]);
    assert_eq!(rc, Some(0), "{lines:?}");
    shaped(&lines);
    assert!(has(&lines, "yellow no-module-installed"), "{lines:?}");
    for part in ["session", "update", "help", "mcp"] {
        let want = format!("yellow skeleton-part-pending part={part} note=\"not there yet\"");
        assert!(lines.contains(&want), "{part}: {lines:?}");
    }
    assert!(has(&lines, "yellow update-binding-pending"), "{lines:?}");
    assert!(lines.contains(&"ok mcp-disabled".to_owned()), "{lines:?}");
    assert!(
        lines
            .last()
            .unwrap()
            .starts_with("ok check-system-done red=0 yellow="),
        "{lines:?}"
    );
}

#[test]
fn a_wrong_word_is_a_usage_error() {
    let i = Install::new();
    let out = i.run(&["--check-system", "--bogus"], "light", &[]);
    assert_eq!(out.status.code(), Some(2));
    let out = i.run(&["--check-system", "--modules"], "light", &[]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn a_module_without_a_program_file_is_red() {
    let i = Install::new();
    i.module("groups", "0.1.0", GOOD_FORMS, &Service::None);
    let (rc, lines) = run(&i, &[], &[]);
    assert_eq!(rc, Some(3), "{lines:?}");
    shaped(&lines);
    assert!(has(&lines, "red module-down module=groups"), "{lines:?}");
    assert!(!has(&lines, "ok module-up"), "{lines:?}");
    assert!(has(&lines, "red check-system-done red=1"), "{lines:?}");
}

#[test]
fn a_refused_module_is_red_with_its_reason() {
    let i = Install::new();
    i.module("groups", "0.1", GOOD_FORMS, &Service::None);
    let (rc, lines) = run(&i, &[], &[]);
    assert_eq!(rc, Some(3), "{lines:?}");
    assert!(
        has(&lines, "red module-refused module=\"groups\""),
        "{lines:?}"
    );
}

#[test]
#[cfg_attr(windows, ignore = "unix sockets")]
fn a_running_module_is_ok_with_its_read_line_and_is_ended_again() {
    let i = Install::new();
    i.module("groups", "0.1.0", GOOD_FORMS, &Service::Fake);
    let (rc, lines) = run(&i, &[], &[]);
    assert_eq!(rc, Some(0), "{lines:?}");
    shaped(&lines);
    assert!(
        has(
            &lines,
            "ok module-up module=groups version=0.1.0 heartbeat=ok ms="
        ),
        "{lines:?}"
    );
    assert!(
        has(&lines, "ok read module=groups form=tree records="),
        "{lines:?}"
    );
    assert!(
        has(&lines, "ok read module=groups form=rows records="),
        "{lines:?}"
    );
    // the run started the service itself, so it ended it again (the socket goes away)
    for _ in 0..50 {
        if !i.started("groups") {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        !i.started("groups"),
        "the service this run started is ended"
    );
}

#[test]
#[cfg_attr(windows, ignore = "unix sockets")]
fn a_service_without_a_heartbeat_is_red() {
    let i = Install::new();
    i.module("groups", "0.1.0", GOOD_FORMS, &Service::Fake);
    let (rc, lines) = run(&i, &[], &[("FAKE_MODE", "nobeat")]);
    assert_eq!(rc, Some(3), "{lines:?}");
    assert!(has(&lines, "red module-down module=groups"), "{lines:?}");
    assert!(lines.iter().any(|l| l.contains("heartbeat")), "{lines:?}");
}

#[test]
#[cfg_attr(windows, ignore = "unix sockets")]
fn the_modules_folder_is_read_where_the_word_says() {
    let other = Install::new();
    other.module("groups", "0.1.0", GOOD_FORMS, &Service::Fake);
    let i = Install::new();
    let dir = other.dir.path().join("modules");
    let (rc, lines) = run(&i, &["--modules", dir.to_str().unwrap()], &[]);
    assert_eq!(rc, Some(0), "{lines:?}");
    assert!(has(&lines, "ok module-up module=groups"), "{lines:?}");
}

#[test]
#[cfg_attr(windows, ignore = "unix sockets")]
fn a_program_description_goes_through_the_check_and_a_mismatch_is_red() {
    let i = Install::new();
    i.module("groups", "0.1.0", GOOD_FORMS, &Service::Fake);
    let (rc, lines) = run(&i, &["--description", &groups()], &[]);
    assert_eq!(rc, Some(0), "{lines:?}");
    shaped(&lines);
    assert!(has(&lines, "ok source-checked program="), "{lines:?}");
    assert!(has(&lines, "ok program-checked program="), "{lines:?}");

    let bad = Install::new();
    let forms = GOOD_FORMS.replace(r#""label":"name","#, "");
    bad.module("groups", "0.1.0", &forms, &Service::Fake);
    let (rc, lines) = run(&bad, &["--description", &groups()], &[]);
    assert_eq!(rc, Some(3), "{lines:?}");
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("red program=") && l.contains("form-channel-unmapped")),
        "{lines:?}"
    );
    assert!(!has(&lines, "ok program-checked"), "{lines:?}");
}

#[test]
fn a_program_that_is_not_installed_is_red_and_a_missing_description_is_red() {
    let i = Install::new();
    let (rc, lines) = run(
        &i,
        &["--program", "nope", "--description", "/nonexistent/x"],
        &[],
    );
    assert_eq!(rc, Some(3), "{lines:?}");
    assert!(
        has(&lines, "red program-refused program=\"nope\""),
        "{lines:?}"
    );
    assert!(
        has(&lines, "red program-unreadable program=\"/nonexistent/x\""),
        "{lines:?}"
    );
}
