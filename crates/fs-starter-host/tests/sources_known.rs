//! SQ-4, borders 1, 2, 3 and 6 on the library side: which modules are bound (bundled, or a system package that `SystemPackaged`
//! accepts), the contract and version check, the program file, and a command of the own name space only.
//! Mutants: the verifier accepts all; the version check off; the symlink check off; the own-namespace check off.
//!
//! Runs on unix and Windows. Only the part that is unix by nature has `cfg(unix)`: the owner and mode bits of the system places
//! (`SystemPlaces` has no Windows form: it refuses every path there, `NotSupported`, which the first package test checks too).

mod support;
use std::path::Path;

use fs_starter_host::registry::SystemPackaged;
use fs_starter_host::sources::{Limits, Modules, StarterSources};
use support::install::{GOOD_FORMS, commands, link};

fn folder(root: &Path, name: &str, version: &str) {
    let catalog = root.join(name).join("catalog");
    std::fs::create_dir_all(&catalog).unwrap();
    std::fs::write(catalog.join("commands.json"), commands(name, version)).unwrap();
    std::fs::write(catalog.join("forms.json"), GOOD_FORMS).unwrap();
}

fn load(bundled: &Path, system: &Path, v: &SystemPackaged) -> Modules {
    Modules::load(bundled, system, v)
}

#[test]
fn a_bundled_module_is_known_with_its_version_and_folder() {
    let b = tempfile::tempdir().unwrap();
    folder(b.path(), "groups", "0.1");
    let m = load(b.path(), Path::new("/nonexistent"), &SystemPackaged::new());
    let k = m.find("groups").expect("known");
    assert_eq!((k.version, k.packaged), ((0, 1), false));
    assert_eq!(k.folder, b.path().join("groups"));
    assert!(m.find("tasks").is_none());
}

#[test]
fn a_package_is_not_bound_while_the_system_check_says_no() {
    let s = tempfile::tempdir().unwrap();
    folder(s.path(), "groups", "0.1");
    // the real check: a folder of this user is not root's: refused, so the module is not known at all
    let m = load(Path::new("/nonexistent"), s.path(), &SystemPackaged::new());
    assert!(
        m.find("groups").is_none(),
        "a loaded module is refused until the system check says yes"
    );
}

/// The accepting half needs what only unix has: the owner (uid) and the mode bits of the folders, which `SystemPlaces::check` reads.
/// On Windows the system check refuses every path (`NotSupported`), so no module can be bound as a package there; the test above is that case.
#[cfg(unix)]
#[test]
fn a_package_is_bound_when_the_system_check_accepts_it() {
    use fs_starter::SystemPlaces;
    use std::os::unix::fs::MetadataExt;
    let s = tempfile::tempdir().unwrap();
    folder(s.path(), "groups", "0.1");
    // a check that accepts the test places: known, as packaged; its program file must still pass the REAL system places
    let uid = std::fs::metadata(s.path()).unwrap().uid();
    let ok = SystemPackaged::for_test(
        SystemPlaces::for_test(vec![s.path().to_path_buf()], uid),
        s.path().to_path_buf(),
    );
    let m = load(Path::new("/nonexistent"), s.path(), &ok);
    let k = m.find("groups").expect("known");
    assert!(k.packaged);
    std::fs::write(s.path().join("groups/service"), "x").unwrap();
    assert_eq!(
        k.service(),
        Err("not-system"),
        "a program file outside the real system places is not started"
    );
}

#[test]
fn the_program_file_is_a_plain_file_in_the_folder_and_nothing_else() {
    let b = tempfile::tempdir().unwrap();
    folder(b.path(), "groups", "0.1");
    let m = load(b.path(), Path::new("/nonexistent"), &SystemPackaged::new());
    let k = m.find("groups").unwrap();
    assert_eq!(
        k.service(),
        Err("no-service"),
        "no PATH lookup, no fallback"
    );
    let file = b.path().join("groups/service");
    // any real file as the target (a made-up link target is not a symlink test on Windows)
    let target = b.path().join("target");
    std::fs::write(&target, "x").unwrap();
    link(&target, &file).unwrap();
    assert_eq!(k.service(), Err("symlink"));
    std::fs::remove_file(&file).unwrap();
    std::fs::create_dir(&file).unwrap();
    assert_eq!(k.service(), Err("not-a-file"));
    std::fs::remove_dir(&file).unwrap();
    std::fs::write(&file, "x").unwrap();
    assert_eq!(k.service(), Ok(file));
}

fn sources(version: &str) -> (tempfile::TempDir, StarterSources) {
    let b = tempfile::tempdir().unwrap();
    folder(b.path(), "groups", version);
    let m = load(b.path(), Path::new("/nonexistent"), &SystemPackaged::new());
    (b, StarterSources::new(m, Limits::default()))
}

fn code<T>(r: Result<T, fs_forms_run::Coded>) -> String {
    r.err().expect("refused").code
}

#[test]
fn the_contract_and_its_range_are_compared_and_never_skipped() {
    let (_b, s) = sources("0.1");
    assert!(s.declared("groups >=0.1 <0.2").is_ok());
    assert!(s.declared("groups").is_ok(), "no range: any version");
    assert_eq!(
        code(s.declared("tasks >=0.1")),
        "host-notice-source-unknown"
    );
    for range in ["groups >=0.2", "groups <0.1", "groups >0.1", "groups =1.0"] {
        assert_eq!(
            code(s.declared(range)),
            "host-notice-data-contract",
            "{range}"
        );
    }
    let c = s.declared("groups >=x").err().unwrap();
    assert!(
        c.code == "host-notice-data-contract"
            && c.params.contains(&("cause".into(), "range".into()))
    );
}

#[test]
fn a_command_of_another_name_space_is_never_run() {
    let (_b, s) = sources("0.1");
    for command in [
        "app.undo",
        "tasks.add",
        "groups",
        "groups.unlisted",
        "groupsx.add",
        "groups.",
        ".add",
    ] {
        assert_eq!(
            code(s.invoke("groups", command, "r1")),
            "host-notice-invoke-foreign",
            "{command}"
        );
    }
    // a module that is not there: unknown source, not a command
    assert_eq!(
        code(s.invoke("tasks", "tasks.add", "r1")),
        "host-notice-source-unknown"
    );
    // the own, listed command passes the border (it then fails for another reason: no program file here)
    assert_ne!(
        code(s.invoke("groups", "groups.add", "r1")),
        "host-notice-invoke-foreign"
    );
}

/// A module with a version of three parts (`0.1.0`) is refused by the registry: the window says THAT, not "unknown source".
#[test]
fn a_module_version_the_registry_cannot_read_is_named_not_called_unknown() {
    let (_b, s) = sources("0.1.0");
    let c = s.declared("groups").err().unwrap();
    assert_eq!(c.code, "host-notice-data-contract");
    assert!(
        c.params
            .contains(&("cause".into(), "version-unreadable".into())),
        "{c:?}"
    );
    // a module that is not there at all stays unknown
    assert_eq!(code(s.declared("tasks")), "host-notice-source-unknown");
}

/// The refusal of a folder is kept, not thrown away (`Modules::refusal`), and a good module beside it is still bound.
#[test]
fn a_refused_folder_is_kept_and_a_good_one_beside_it_is_bound() {
    let b = tempfile::tempdir().unwrap();
    folder(b.path(), "groups", "0.1.0");
    folder(b.path(), "tasks", "0.1");
    let m = load(b.path(), Path::new("/nonexistent"), &SystemPackaged::new());
    assert!(m.find("groups").is_none());
    assert!(m.find("tasks").is_some());
    assert_eq!(m.refusal("groups").map(|r| r.0), Some("version-unreadable"));
    assert!(m.refusal("tasks").is_none());
}

/// A folder that exists but cannot be read is a refusal too (only a MISSING folder is fine).
#[test]
fn a_folder_that_cannot_be_read_is_not_silently_empty() {
    let b = tempfile::tempdir().unwrap();
    let file = b.path().join("not-a-folder");
    std::fs::write(&file, "x").unwrap();
    let m = load(&file, Path::new("/nonexistent"), &SystemPackaged::new());
    assert!(m.refusal("").is_some(), "the error of the folder is kept");
    let missing = load(
        Path::new("/nonexistent"),
        Path::new("/nonexistent"),
        &SystemPackaged::new(),
    );
    assert!(missing.refusal("").is_none(), "a missing folder is fine");
}
