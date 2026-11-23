//! The command registry: `app.*` plus the sets of bundled modules, read at run time, and the rules that keep it safe.
//!
//! Tests: example-based per rule (bot, trust, prefix, name space, conflict, all-or-nothing, marker) and through the binary
//! (`--list-commands`). Mutants (see the log of the run): the `app.` check left out; the name-space check left out; the conflict
//! check left out; the bot check left out; the trust check left out; the modules not read (fixed list).

mod support;
use support::{host, text};

use fs_starter_host::registry::{Access, Actor, ModuleSet, Origin, Refusal, Registry, parse};

/// The text of a set file with `module` and one command per id (all fields the format requires).
fn json(module: &str, ids: &[&str]) -> String {
    let commands: Vec<String> = ids
        .iter()
        .map(|id| {
            format!(
                r#"{{"id":{id:?},"call":"x","capability":"{module}.cap","access":"read","help":{{"topic":{:?},"text":{{"en":"e","de":"d"}}}}}}"#,
                id.replace('.', "-")
            )
        })
        .collect();
    format!(
        r#"{{"module":"{module}","version":"0.1","contract":"{module}","scope":"own-contract","commands":[{}]}}"#,
        commands.join(",")
    )
}

fn set(module: &str, ids: &[&str]) -> ModuleSet {
    parse(&json(module, ids), module).unwrap()
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

/// A copy of `catalog/commands.json` of fs-capture @a9ca621.
const REAL: &str = include_str!("data/fs-capture-catalog/capture/catalog/commands.json");

#[test]
fn the_real_file_of_fs_capture_is_read_with_help_and_access() {
    let set = parse(REAL, "capture").unwrap();
    assert_eq!(
        (set.module.as_str(), set.version.as_str()),
        ("capture", "0.1")
    );
    assert_eq!(set.items.len(), 6);
    assert_eq!(set.items[2].access, Access::Write);
    assert_eq!(set.items[4].access, Access::Delete);
    assert_eq!(set.items[0].help.topic, "capture-screens");
    assert!(set.items[0].help.de.starts_with("Listet"));
    let mut r = Registry::new();
    r.add(Actor::Person, Origin::Bundled, set).unwrap();
    let lines = r.lines();
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.contains("\tnew by module capture"))
            .count(),
        6
    );
    assert!(
        lines[4].starts_with("capture.screens\tcapture\t0.1"),
        "{}",
        lines[4]
    );
}

/// The real file with one change; every mandatory field and every rule must turn the test red when it is not kept.
fn changed(from: &str, to: &str) -> String {
    assert!(REAL.contains(from), "{from}");
    REAL.replacen(from, to, 1)
}

#[test]
fn a_file_that_misses_a_mandatory_field_or_breaks_a_rule_is_unreadable() {
    let cause = |text: String, folder: &str| match parse(&text, folder) {
        Err(Refusal::Unreadable(c)) => c,
        other => format!("{other:?}"),
    };
    assert_eq!(cause("not json [".into(), "capture"), "json");
    assert_eq!(
        cause(changed("\"version\": \"0.1\",", ""), "capture"),
        "version"
    );
    assert_eq!(cause(changed("\"0.1\"", "\"0.x\""), "capture"), "version");
    assert_eq!(
        cause(changed("\"contract\": \"capture\",", ""), "capture"),
        "contract"
    );
    assert_eq!(
        cause(
            changed("\"contract\": \"capture\"", "\"contract\": \"zip\""),
            "capture"
        ),
        "contract"
    );
    assert_eq!(
        cause(changed("\"scope\": \"own-contract\",", ""), "capture"),
        "scope"
    );
    assert_eq!(cause(changed("own-contract", "all"), "capture"), "scope");
    assert_eq!(
        cause(
            changed("\"access\": \"read\"", "\"access\": \"admin\""),
            "capture"
        ),
        "command-access"
    );
    assert_eq!(
        cause(changed("\"access\": \"read\",", ""), "capture"),
        "command-access"
    );
    assert_eq!(
        cause(
            changed(
                "\"capability\": \"capture.screens\"",
                "\"capability\": \"tasks.screens\""
            ),
            "capture"
        ),
        "command-capability"
    );
    assert_eq!(
        cause(
            changed(
                "\"de\": \"Listet die Bildschirme (Nummer, Name, Groesse, Skalierung, Hauptbildschirm).\"",
                "\"x\": \"y\""
            ),
            "capture"
        ),
        "command-help"
    );
    assert_eq!(
        cause(changed("\"topic\": \"capture-screens\", ", ""), "capture"),
        "command-help"
    );
    assert_eq!(
        cause(changed("\"id\": \"capture.screens\", ", ""), "capture"),
        "command-id"
    );
    assert_eq!(cause(r#"{"module":"capture","version":"0.1","contract":"capture","scope":"own-contract","commands":[]}"#.into(), "capture"), "commands");
    assert_eq!(
        cause(
            r#"{"module":"capture","version":"0.1","contract":"capture","scope":"own-contract"}"#
                .into(),
            "capture"
        ),
        "commands"
    );
    assert_eq!(
        cause(changed("\"module\": \"capture\",", ""), "capture"),
        "module"
    );
}

#[test]
fn the_module_name_must_be_the_name_of_the_folder() {
    assert!(matches!(parse(REAL, "other"), Err(Refusal::ModuleName(n)) if n == "capture"));
}

fn modules_dir(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (module, body) in files {
        let catalog = dir.path().join(module).join("catalog");
        std::fs::create_dir_all(&catalog).unwrap();
        std::fs::write(catalog.join("commands.json"), body).unwrap();
    }
    dir
}

fn capture() -> String {
    json("capture", &["capture.shot"])
}

#[test]
fn list_commands_reads_the_bundled_modules_at_run_time() {
    let dir = modules_dir(&[("capture", &capture())]);
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
        ("capture", &capture()),
        ("zip", &json("zip", &["zip.shot"])),
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
    let evil = json("capture", &["app.quit"]);
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
