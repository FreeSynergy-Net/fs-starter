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
fn the_valid_seed_layout_of_the_generator_is_accepted() {
    // `fs-blueprint new` seeds a real, readable forms layout (schema 1, kind frontend): `--check` succeeds.
    let out = host(&[&forms_package(), "--check"], "light");
    let err = text(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{err}");
    assert!(!err.contains("layout-"), "no layout error: {err}");
}

/// A copy of the fixture in a temp folder whose `forms/simple.toml` is `layout`.
fn package_with_layout(layout: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let from = std::path::PathBuf::from(forms_package());
    std::fs::copy(from.join("scaffold.toml"), dir.path().join("scaffold.toml")).unwrap();
    std::fs::create_dir(dir.path().join("forms")).unwrap();
    std::fs::write(dir.path().join("forms/simple.toml"), layout).unwrap();
    dir
}

#[test]
fn a_layout_that_is_only_a_placeholder_comment_is_said_and_never_an_empty_window() {
    // The old generated forms/simple.toml was only a comment: fs-forms-run refuses it with a code; the host exits with a failure
    // code instead of showing nothing.
    let dir = package_with_layout("# the forms description of fs-forms (placeholder)\n");
    let out = host(&[dir.path().to_str().unwrap(), "--check"], "light");
    let err = text(&out.stderr);
    assert_ne!(out.status.code(), Some(0), "{err}");
    assert!(err.contains("layout-"), "a coded cause is named: {err}");
}

#[test]
#[ignore = "needs FS_BLUEPRINT_BIN (a built fs-blueprint of fs-release main dcbf81b or newer)"]
fn what_fs_blueprint_new_writes_is_accepted_by_the_host() {
    // The chain: scaffold.toml -> `fs-blueprint new` -> data package -> `fs-starter <package> --check`. Needs the built generator;
    // its path comes in `FS_BLUEPRINT_BIN`. The test is `#[ignore]`d, so the skip shows in every run; `cargo test -- --ignored`
    // runs it, and then a missing `FS_BLUEPRINT_BIN` is a failure.
    let bin =
        std::env::var_os("FS_BLUEPRINT_BIN").expect("FS_BLUEPRINT_BIN names a built fs-blueprint");
    let work = tempfile::tempdir().unwrap();
    let from = std::path::PathBuf::from(forms_package());
    std::fs::copy(
        from.join("scaffold.toml"),
        work.path().join("scaffold.toml"),
    )
    .unwrap();
    let made = work.path().join("made");
    let generated = std::process::Command::new(bin)
        .current_dir(work.path())
        .args(["new", "scaffold.toml", "--out"])
        .arg(&made)
        .output()
        .unwrap();
    assert_eq!(
        generated.status.code(),
        Some(0),
        "{}",
        text(&generated.stderr)
    );
    let out = host(&[made.to_str().unwrap(), "--check"], "light");
    let err = text(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{err}");
    assert!(err.contains("package forms (skeleton 0.1.0)"), "{err}");
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
