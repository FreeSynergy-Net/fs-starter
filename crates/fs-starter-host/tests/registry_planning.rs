//! SW-5, acceptance: the REAL, unchanged `catalog/commands.json` and `catalog/forms.json` of fs-modules-planning main 87874b3 (bare
//! contract keys, no `id`) and the SW-7 forms of fs-tasks (52ff9246: `planning@>=0.0.1, <0.1.0`, keys without prefix) through the
//! host: `--list-commands` and `--check`. Both copies are in `tests/data/` and must never be edited (a change there is a new copy).

mod support;
use support::install::Install;

const PLANNING: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/data/planning-main-87874b3"
);
const TASKS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/data/fs-tasks-sw7-52ff9246"
);

/// An installation with the real files of the module `planning` in `modules/planning/catalog/` (no service: `--check` starts none).
fn install() -> Install {
    let i = Install::new();
    let catalog = i.folder("planning").join("catalog");
    std::fs::create_dir_all(&catalog).unwrap();
    for file in ["commands.json", "forms.json"] {
        std::fs::copy(format!("{PLANNING}/catalog/{file}"), catalog.join(file)).unwrap();
    }
    i
}

#[test]
fn the_real_commands_of_planning_get_the_prefix_from_the_host() {
    let i = install();
    let dir = i.dir.path().join("modules");
    let out = i.run(
        &["--list-commands", "--modules", dir.to_str().unwrap()],
        "light",
        &[],
    );
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{text} {}",
        String::from_utf8_lossy(&out.stderr)
    );
    for id in [
        "planning.card.move",
        "planning.task.create",
        "planning.task.delete",
    ] {
        assert!(
            text.lines()
                .any(|l| l.starts_with(&format!("{id}\tplanning\t0.0.2\tnew by module planning"))),
            "{id}: {text}"
        );
    }
    assert_eq!(
        text.lines().filter(|l| l.contains("new by module")).count(),
        3,
        "{text}"
    );
}

#[test]
fn check_of_the_sw7_forms_against_the_real_planning_files_is_green() {
    let i = install();
    let out = i.run(&["--check", "--description", TASKS], "light", &[]);
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    println!("rc={:?}\n{text}", out.status.code());
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(text.contains("source planning 0.0.2 checked"), "{text}");
    assert!(!text.contains("red") && !text.contains("yellow"), "{text}");
}
