//! The command registry: `app.*` plus the sets of bundled modules, and the rules that keep it safe.
//!
//! Tests: example-based per rule (bot, trust, prefix, name space, conflict, all-or-nothing). Mutants: the `app.` check left out; the
//! name-space check left out; the conflict check left out; the bot check left out; the trust check left out.

mod support;
use support::sets::{ids, set};

use fs_starter_host::registry::{Actor, ModuleSet, Refusal, Registry, Unbound, Verifier};

#[test]
fn the_shell_commands_come_from_the_command_model_and_a_module_adds_its_own() {
    // The pin of fs-components 2ffa1c4d brings `app.undo` / `app.redo` itself: each id stands once (not once from there and once from the host).
    let all = ids(&Registry::new());
    let mut once = all.clone();
    once.sort();
    once.dedup();
    assert_eq!(all.len(), once.len(), "an id twice: {all:?}");
    let mut r = Registry::new();
    assert_eq!(
        ids(&r),
        [
            "app.settings",
            "app.quit",
            "app.help",
            "app.about",
            "app.undo",
            "app.redo"
        ]
    );
    r.add_bundled(
        Actor::Person,
        set("capture", &["capture.shot", "capture.list"]),
    )
    .unwrap();
    assert_eq!(r.entries().len(), 8);
    let lines = r.lines();
    assert_eq!(
        lines[6],
        "capture.shot\tcapture\t0.1.0\tnew by module capture"
    );
    assert!(!lines[0].contains("new by"), "{}", lines[0]);
}

#[test]
fn a_module_may_not_take_the_app_prefix() {
    let mut r = Registry::new();
    let why = r
        .add_bundled(Actor::Person, set("capture", &["app.undo"]))
        .unwrap_err();
    assert!(
        matches!(&why[0], Refusal::ReservedPrefix(id) if id.as_str() == "app.undo"),
        "{why:?}"
    );
    let why = r
        .add_bundled(Actor::Person, set("app", &["app.undo"]))
        .unwrap_err();
    assert!(
        why.iter().any(|w| matches!(w, Refusal::ModuleName(_))),
        "{why:?}"
    );
    assert_eq!(r.entries().len(), 6);
}

#[test]
fn a_module_stays_in_its_own_name_space() {
    let mut r = Registry::new();
    let why = r
        .add_bundled(Actor::Person, set("capture", &["tasks.move"]))
        .unwrap_err();
    assert!(matches!(&why[0], Refusal::OutsideNamespace(_)), "{why:?}");
}

#[test]
fn a_foreign_id_is_never_overwritten_and_the_whole_set_is_refused() {
    let mut r = Registry::new();
    r.add_bundled(Actor::Person, set("capture", &["capture.shot"]))
        .unwrap();
    // the same id again (another set of the same module), and a good one next to it
    let why = r
        .add_bundled(
            Actor::Person,
            set("capture", &["capture.zip", "capture.shot"]),
        )
        .unwrap_err();
    assert_eq!(why, [Refusal::Conflict("capture.shot".into())]);
    // twice inside one set
    let why = r
        .add_bundled(Actor::Person, set("zip", &["zip.a", "zip.a"]))
        .unwrap_err();
    assert_eq!(why, [Refusal::Conflict("zip.a".into())]);
    assert_eq!(
        ids(&r).len(),
        7,
        "nothing of a refused set is added: {:?}",
        ids(&r)
    );
}

#[test]
fn a_bot_never_extends_the_command_set() {
    let mut r = Registry::new();
    let why = r
        .add_bundled(Actor::Bot, set("capture", &["capture.shot"]))
        .unwrap_err();
    assert_eq!(why, [Refusal::Bot]);
    assert_eq!(r.entries().len(), 6);
}

/// A test double of the signature check that accepts (the real one is `fs-update`, not bound yet).
struct Accepting;
impl Verifier for Accepting {
    fn verify(&self, _set: &ModuleSet) -> bool {
        true
    }
}

#[test]
fn a_package_is_refused_by_default_and_accepted_only_by_a_verifier_that_says_yes() {
    let mut r = Registry::new();
    let why = r
        .add_package(Actor::Person, set("capture", &["capture.shot"]), &Unbound)
        .unwrap_err();
    assert_eq!(why, [Refusal::NotTrusted]);
    assert_eq!(r.entries().len(), 6);
    r.add_package(Actor::Person, set("capture", &["capture.shot"]), &Accepting)
        .unwrap();
    assert_eq!(r.entries().len(), 7);
    // a verifier does not lift the other rules
    let why = r
        .add_package(Actor::Person, set("zip", &["app.x"]), &Accepting)
        .unwrap_err();
    assert!(matches!(&why[0], Refusal::ReservedPrefix(_)), "{why:?}");
}
