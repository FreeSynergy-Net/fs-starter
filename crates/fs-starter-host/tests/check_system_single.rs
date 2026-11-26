//! `--check-system` and a form of the kind `single-record`: its scope is always one record id, a module refuses any other with 404, so the
//! check does not read it and says it yellow (`single-record-not-probed`). Unix only (the fake module serves on sockets).

mod support;
use support::install::{Install, Service};

fn run(i: &Install) -> (Option<i32>, Vec<String>) {
    let out = i.run(&["--check-system"], "light", &[]);
    let lines = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    (out.status.code(), lines)
}

fn has(lines: &[String], start: &str) -> bool {
    lines.iter().any(|l| l.starts_with(start))
}

/// The groups forms plus a one-record form of the type `Member`: the fake refuses it (404 `unknown-scope`) without a record id, like a module.
const WITH_SINGLE: &str = r#"[
 {"name":"rows","version":"1.0","type":"Member",
  "map":{"items":"Member","label":"person","columns":["role","level"]},
  "fields":{"person":"reference","role":"reference","level":"text"}},
 {"name":"single-record","version":"1.2","type":"Member",
  "map":{"item":"Member","label":"person"},
  "fields":{"person":"reference"}}
]"#;

#[test]
#[cfg_attr(windows, ignore = "unix sockets")]
fn a_single_record_form_is_not_probed_it_is_yellow_never_red() {
    let i = Install::new();
    i.module("groups", "0.1.0", WITH_SINGLE, &Service::Fake);
    let (rc, lines) = run(&i);
    assert_eq!(rc, Some(0), "{lines:?}");
    assert!(has(&lines, "ok module-up module=groups"), "{lines:?}");
    assert!(
        has(&lines, "ok read module=groups form=rows records="),
        "{lines:?}"
    );
    assert!(
        lines.contains(
            &"yellow single-record-not-probed module=groups form=single-record@1.2".to_owned()
        ),
        "{lines:?}"
    );
    assert!(!has(&lines, "red "), "{lines:?}");
    assert!(
        !has(&lines, "ok read module=groups form=single-record"),
        "{lines:?}"
    );
}
