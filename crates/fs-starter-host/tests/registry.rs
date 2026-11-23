//! The command registry: `app.*` plus the sets of bundled modules, read at run time, and the rules that keep it safe.
//!
//! Tests: example-based per rule (bot, trust, prefix, name space, conflict, all-or-nothing, marker) and through the binary
//! (`--list-commands`). Mutants (see the log of the run): the `app.` check left out; the name-space check left out; the conflict
//! check left out; the bot check left out; the trust check left out; the modules not read (fixed list).

mod support;
use support::{host, text};

use fs_starter_host::registry::{Actor, ModuleSet, Origin, Refusal, Registry, parse};

fn set(module: &str, ids: &[&str]) -> ModuleSet {
    let commands: String = ids
        .iter()
        .map(|id| {
            format!(
                "[[command]]\nid = \"{id}\"\nlabel = \"{}\"\n",
                id.replace('.', "-")
            )
        })
        .collect();
    parse(&format!(
        "[module]\nname = \"{module}\"\nversion = \"0.1\"\n{commands}"
    ))
    .unwrap()
}

fn ids(r: &Registry) -> Vec<String> {
    r.entries()
        .iter()
        .map(|e| e.command.id.as_str().to_owned())
        .collect()
}

#[test]
fn the_shell_commands_come_from_the_command_model_and_a_module_adds_its_own() {
    let mut r = Registry::new();
    assert_eq!(
        ids(&r),
        ["app.settings", "app.quit", "app.help", "app.about"]
    );
    r.add(
        Actor::Person,
        Origin::Bundled,
        set("capture", &["capture.shot", "capture.list"]),
    )
    .unwrap();
    assert_eq!(r.entries().len(), 6);
    let lines = r.lines();
    assert_eq!(
        lines[4],
        "capture.shot\tcapture\t0.1\tnew by module capture"
    );
    assert!(!lines[0].contains("new by"), "{}", lines[0]);
}

#[test]
fn a_module_may_not_take_the_app_prefix() {
    let mut r = Registry::new();
    let why = r
        .add(
            Actor::Person,
            Origin::Bundled,
            set("capture", &["app.undo"]),
        )
        .unwrap_err();
    assert!(
        matches!(&why[0], Refusal::ReservedPrefix(id) if id.as_str() == "app.undo"),
        "{why:?}"
    );
    let why = r
        .add(Actor::Person, Origin::Bundled, set("app", &["app.undo"]))
        .unwrap_err();
    assert!(
        why.iter().any(|w| matches!(w, Refusal::ModuleName(_))),
        "{why:?}"
    );
    assert_eq!(r.entries().len(), 4);
}

#[test]
fn a_module_stays_in_its_own_name_space() {
    let mut r = Registry::new();
    let why = r
        .add(
            Actor::Person,
            Origin::Bundled,
            set("capture", &["tasks.move"]),
        )
        .unwrap_err();
    assert!(matches!(&why[0], Refusal::OutsideNamespace(_)), "{why:?}");
}

#[test]
fn a_foreign_id_is_never_overwritten_and_the_whole_set_is_refused() {
    let mut r = Registry::new();
    r.add(
        Actor::Person,
        Origin::Bundled,
        set("capture", &["capture.shot"]),
    )
    .unwrap();
    // the same id again (another set of the same module), and a good one next to it
    let why = r
        .add(
            Actor::Person,
            Origin::Bundled,
            set("capture", &["capture.zip", "capture.shot"]),
        )
        .unwrap_err();
    assert_eq!(why, [Refusal::Conflict("capture.shot".into())]);
    // twice inside one set
    let why = r
        .add(
            Actor::Person,
            Origin::Bundled,
            set("zip", &["zip.a", "zip.a"]),
        )
        .unwrap_err();
    assert_eq!(why, [Refusal::Conflict("zip.a".into())]);
    assert_eq!(
        ids(&r).len(),
        5,
        "nothing of a refused set is added: {:?}",
        ids(&r)
    );
}

#[test]
fn a_bot_never_extends_the_command_set() {
    let mut r = Registry::new();
    let why = r
        .add(
            Actor::Bot,
            Origin::Bundled,
            set("capture", &["capture.shot"]),
        )
        .unwrap_err();
    assert_eq!(why, [Refusal::Bot]);
    assert_eq!(r.entries().len(), 4);
}

#[test]
fn only_a_bundled_module_or_a_signed_package_is_accepted() {
    let mut r = Registry::new();
    let why = r
        .add(
            Actor::Person,
            Origin::Package {
                signature_ok: false,
            },
            set("capture", &["capture.shot"]),
        )
        .unwrap_err();
    assert_eq!(why, [Refusal::NotTrusted]);
    r.add(
        Actor::Person,
        Origin::Package { signature_ok: true },
        set("capture", &["capture.shot"]),
    )
    .unwrap();
    assert_eq!(r.entries().len(), 5);
}

#[test]
fn a_set_file_that_is_not_a_set_is_unreadable() {
    assert!(matches!(parse("not toml ["), Err(Refusal::Unreadable(c)) if c == "toml"));
    assert!(
        matches!(parse("[[command]]\nid = \"a.b\"\nlabel = \"x\"\n"), Err(Refusal::Unreadable(c)) if c == "module-missing")
    );
}

fn modules_dir(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (module, body) in files {
        std::fs::create_dir_all(dir.path().join(module)).unwrap();
        std::fs::write(dir.path().join(module).join("commands.toml"), body).unwrap();
    }
    dir
}

const CAPTURE: &str = "[module]\nname = \"capture\"\nversion = \"0.1\"\n[[command]]\nid = \"capture.shot\"\nlabel = \"capture-shot\"\n";

#[test]
fn list_commands_reads_the_bundled_modules_at_run_time() {
    let dir = modules_dir(&[("capture", CAPTURE)]);
    let out = host(
        &["--list-commands", "--modules", dir.path().to_str().unwrap()],
        "light",
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let lines = text(&out.stdout);
    assert!(lines.contains("app.quit\tapp\t"), "{lines}");
    assert!(
        lines.contains("capture.shot\tcapture\t0.1\tnew by module capture"),
        "{lines}"
    );
    // a second module appears without a change of the host
    let more = modules_dir(&[
        ("capture", CAPTURE),
        ("zip", &CAPTURE.replace("capture", "zip")),
    ]);
    let out = host(
        &[
            "--list-commands",
            "--modules",
            more.path().to_str().unwrap(),
        ],
        "light",
    );
    assert!(
        text(&out.stdout).contains("zip.shot\tzip\t0.1"),
        "{}",
        text(&out.stdout)
    );
}

#[test]
fn list_commands_refuses_a_module_that_takes_app_and_names_it() {
    let evil = CAPTURE.replace("capture.shot", "app.quit");
    let dir = modules_dir(&[("capture", &evil)]);
    let out = host(
        &["--list-commands", "--modules", dir.path().to_str().unwrap()],
        "light",
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("command-reserved-prefix: app.quit"),
        "{}",
        text(&out.stderr)
    );
    assert_eq!(
        text(&out.stdout).matches("app.quit").count(),
        1,
        "the shell's own line only"
    );
}

#[test]
fn list_commands_without_modules_lists_the_shell_only_and_wrong_words_are_usage() {
    let dir = modules_dir(&[]);
    let out = host(
        &["--list-commands", "--modules", dir.path().to_str().unwrap()],
        "light",
    );
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(text(&out.stdout).lines().count(), 4);
    assert_eq!(
        host(&["--list-commands", "--bogus"], "light").status.code(),
        Some(2)
    );
}
