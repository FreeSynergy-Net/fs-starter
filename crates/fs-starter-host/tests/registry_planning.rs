//! SW-5, acceptance: the REAL, unchanged `catalog/commands.json` and `catalog/forms.json` of fs-modules-planning main 87874b3 (bare
//! contract keys, no `id`) and the SW-7 forms of fs-tasks (52ff9246: `planning@>=0.0.1, <0.1.0`, keys without prefix) through the
//! host: `--list-commands` and `--check`. Both copies are in `tests/data/` and are not edited, with ONE exception: the three `[[action]]` of the
//! board of fs-tasks got `at = "block:board/main"` (fs-forms-layout 4a549f11 refuses a gesture without a place: cause `at-missing`).

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
    // the one yellow that is meant: these planning files have no `catalog/types.bin` yet (pin-13b); nothing else is yellow, nothing red
    assert!(
        text.contains("yellow no-type-catalog module=planning"),
        "{text}"
    );
    let rest = text.replace("yellow no-type-catalog module=planning", "");
    assert!(!rest.contains("red") && !rest.contains("yellow"), "{text}");
}

/// The planning forms with the key of one channel changed: `--check` must say red, never green (SW-5: the keys of the forms are checked
/// against the command set of the module).
fn check_with_key(from: &str, to: &str) -> (Option<i32>, String) {
    let i = install();
    let file = i.folder("planning").join("catalog").join("forms.json");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains(from), "{from}");
    std::fs::write(&file, text.replace(from, to)).unwrap();
    let out = i.run(&["--check", "--description", TASKS], "light", &[]);
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn check_is_red_for_a_form_key_the_module_does_not_have() {
    for key in ["task.steal", "capture.screens", "planning.task.create"] {
        let (rc, text) = check_with_key("\"task.create\"", &format!("\"{key}\""));
        assert_eq!(rc, Some(3), "{key}: {text}");
        assert!(
            text.contains("red form-command-unknown") && text.contains(&format!("command={key}")),
            "{key}: {text}"
        );
    }
}

#[test]
fn a_capability_that_is_not_on_the_contract_list_leaves_out_only_that_command() {
    let i = install();
    let file = i.folder("planning").join("catalog").join("commands.json");
    let text = std::fs::read_to_string(&file).unwrap();
    std::fs::write(&file, text.replace("\"task.delete\"", "\"task.steal\"")).unwrap();
    let dir = i.dir.path().join("modules");
    let out = i.run(
        &["--list-commands", "--modules", dir.to_str().unwrap()],
        "light",
        &[],
    );
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // tolerant reader: the module stays, only the one command is not offered (said by `--check-system`, see tests/tolerant.rs)
    assert!(!all.contains("command-capability-unknown"), "{all}");
    assert!(all.contains("planning.task.create"), "{all}");
    assert!(
        !all.contains("task.steal") && !all.contains("planning.task.delete"),
        "{all}"
    );
}
