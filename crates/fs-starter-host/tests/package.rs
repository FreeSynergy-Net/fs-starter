//! `fs-starter <package-path>`: the data package written by `fs-blueprint new` is the argument; the host reads its facts, names the
//! pending parts and hands the layout to `fs-forms-run`. No window is opened here (`--check`); the headless proof is the exit code
//! and the lines. Tests: example-based. Mutants: the package line left out; a broken scaffold.toml opened anyway.

mod support;
use support::{forms_package, host, text};

use fs_starter::Exit;

#[test]
fn the_package_is_read_and_named_before_the_layout_is_fetched() {
    let out = host(&[&forms_package(), "--check"], "light");
    let err = text(&out.stderr);
    assert!(
        err.contains("package forms (skeleton 0.1.0), layout forms/simple.toml"),
        "{err}"
    );
    assert_eq!(err.matches("skeleton-part-pending").count(), 1, "{err}");
}

#[test]
fn a_layout_that_is_only_a_placeholder_is_said_and_never_an_empty_window() {
    // The generated forms/simple.toml is a comment (E4 gap "layout empty"): fs-forms-run refuses it with a code; the host exits
    // with a failure code instead of showing nothing.
    let out = host(&[&forms_package(), "--check"], "light");
    let err = text(&out.stderr);
    assert_ne!(out.status.code(), Some(0), "{err}");
    assert!(err.contains("layout-"), "a coded cause is named: {err}");
}

#[test]
fn a_scaffold_that_is_not_toml_ends_with_a_failure_and_is_not_opened() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("scaffold.toml"), "[program\n").unwrap();
    let out = host(&[dir.path().to_str().unwrap(), "--check"], "light");
    assert_eq!(
        out.status.code(),
        Some(i32::from(Exit::FAILED.code())),
        "{}",
        text(&out.stderr)
    );
    assert!(text(&out.stderr).contains("package-unreadable"));
}

#[test]
fn the_reader_gives_the_three_facts() {
    let p = fs_starter_host::read_package(std::path::Path::new(&forms_package()))
        .unwrap()
        .unwrap();
    assert_eq!(p.program, "forms");
    assert_eq!(p.skeleton, "0.1.0");
    assert_eq!(p.layout.as_deref(), Some("forms/simple.toml"));
    assert!(
        fs_starter_host::read_package(std::path::Path::new("/no/such"))
            .unwrap()
            .is_none()
    );
}
