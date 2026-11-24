//! SQ-4, borders 1, 2, 3 and 6 on the library side: which modules are bound (bundled, or a system package that `SystemPackaged`
//! accepts), the contract and version check, the program file, and a command of the own name space only.
//! Mutants: the verifier accepts all; the version check off; the symlink check off; the own-namespace check off.
//!
//! Runs on unix and Windows. Only the part that is unix by nature has `cfg(unix)`: the owner and mode bits of the system places
//! (the test places need the owner (uid) of a folder; the Windows acceptance has its own test, `fs-starter/tests/system_windows.rs`).

mod support;
use std::path::Path;

use fs_starter_host::registry::SystemPackaged;
use fs_starter_host::sources::range::Version;
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
    folder(b.path(), "groups", "0.1.0");
    let m = load(b.path(), Path::new("/nonexistent"), &SystemPackaged::new());
    let k = m.find("groups").expect("known");
    assert_eq!(
        (k.version.clone(), k.packaged),
        (Version::new(0, 1, 0), false)
    );
    assert_eq!(k.folder, b.path().join("groups"));
    assert!(m.find("tasks").is_none());
}

#[test]
fn a_package_is_not_bound_while_the_system_check_says_no() {
    let s = tempfile::tempdir().unwrap();
    folder(s.path(), "groups", "0.1.0");
    // the real check: a folder of this user is not root's: refused, so the module is not known at all
    let m = load(Path::new("/nonexistent"), s.path(), &SystemPackaged::new());
    assert!(
        m.find("groups").is_none(),
        "a loaded module is refused until the system check says yes"
    );
}

/// The accepting half needs what only unix has: the owner (uid) and the mode bits of the folders, which `SystemPlaces::check` reads.
/// On Windows a folder of a test is outside the real place (the anchor), so no module is bound as a package there; the test above is that case.
#[cfg(unix)]
#[test]
fn a_package_is_bound_when_the_system_check_accepts_it() {
    use fs_starter::SystemPlaces;
    use std::os::unix::fs::MetadataExt;
    let s = tempfile::tempdir().unwrap();
    folder(s.path(), "groups", "0.1.0");
    // a check that accepts the test places: known, as packaged; its program file must still pass the REAL system places
    let uid = std::fs::metadata(s.path()).unwrap().uid();
    let ok = SystemPackaged::for_test(
        SystemPlaces::for_test(vec![s.path().to_path_buf()], uid),
        s.path().to_path_buf(),
    );
    let m = load(Path::new("/nonexistent"), s.path(), &ok);
    let k = m.find("groups").expect("known");
    assert!(k.packaged);
    std::fs::write(
        fs_starter::program_file(&s.path().join("groups"), "service"),
        "x",
    )
    .unwrap();
    assert_eq!(
        k.service(),
        Err("not-system"),
        "a program file outside the real system places is not started"
    );
}

#[test]
fn the_program_file_is_a_plain_file_in_the_folder_and_nothing_else() {
    let b = tempfile::tempdir().unwrap();
    folder(b.path(), "groups", "0.1.0");
    let m = load(b.path(), Path::new("/nonexistent"), &SystemPackaged::new());
    let k = m.find("groups").unwrap();
    assert_eq!(
        k.service(),
        Err("no-service"),
        "no PATH lookup, no fallback"
    );
    let file = fs_starter::program_file(&b.path().join("groups"), "service");
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
    let (_b, s) = sources("0.1.0");
    assert!(s.declared("groups@>=0.1.0, <0.2.0").is_ok());
    assert_eq!(
        code(s.declared("tasks@>=0.1.0, <0.2.0")),
        "host-notice-source-unknown"
    );
    for range in [
        "groups@>=0.2.0",
        "groups@<0.1.0",
        "groups@>0.1.0",
        "groups@=1.0.0",
    ] {
        assert_eq!(
            code(s.declared(range)),
            "host-notice-data-contract",
            "{range}"
        );
    }
    // unreadable: a letter, an incomplete version, and the older ways of writing (see tests/range_form.rs)
    for range in [
        "groups@>=x",
        "groups@>=0.1",
        "groups >=0.1.0, <0.2.0",
        "groups",
    ] {
        let c = s.declared(range).err().unwrap();
        assert!(
            c.code == "host-notice-data-contract"
                && c.params.contains(&("cause".into(), "range".into())),
            "{range}"
        );
    }
}

#[test]
fn a_command_of_another_name_space_is_never_run() {
    let (_b, s) = sources("0.1.0");
    for command in [
        "app.undo",
        "tasks.task.create",
        "groups",
        "groups.unlisted",
        "groupsx.task.create",
        "groups.",
        ".task.create",
    ] {
        assert_eq!(
            code(s.invoke("groups", command, "r1", &[])),
            "host-notice-invoke-foreign",
            "{command}"
        );
    }
    // a module that is not there: unknown source, not a command
    assert_eq!(
        code(s.invoke("tasks", "tasks.task.create", "r1", &[])),
        "host-notice-source-unknown"
    );
    // the own, listed command passes the border (it then fails for another reason: no program file here)
    assert_ne!(
        code(s.invoke("groups", "groups.task.create", "r1", &[])),
        "host-notice-invoke-foreign"
    );
}

/// A module with a version of two parts (`0.1`: not a `semver` version) is refused by the registry: the window says THAT, not "unknown source".
#[test]
fn a_module_version_the_registry_cannot_read_is_named_not_called_unknown() {
    let (_b, s) = sources("0.1");
    let c = s.declared("groups@>=0.1.0, <0.2.0").err().unwrap();
    assert_eq!(c.code, "host-notice-data-contract");
    assert!(
        c.params
            .contains(&("cause".into(), "version-unreadable".into())),
        "{c:?}"
    );
    // a module that is not there at all stays unknown
    assert_eq!(
        code(s.declared("tasks@>=0.1.0, <0.2.0")),
        "host-notice-source-unknown"
    );
}

/// The refusal of a folder is kept, not thrown away (`Modules::refusal`), and a good module beside it is still bound.
#[test]
fn a_refused_folder_is_kept_and_a_good_one_beside_it_is_bound() {
    let b = tempfile::tempdir().unwrap();
    folder(b.path(), "groups", "0.1");
    folder(b.path(), "tasks", "0.1.0");
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

/// K5 (WH0 P2): where program files have an ending, `service` next to `service.exe` is a second file the system could start instead
/// of the checked one. Such a module is refused with its own cause `ambiguous-service`; only the file with the ending counts.
/// Runs with the Windows ending `.exe` on every system (the rule is the same, the real ending is `fs_starter::EXE_SUFFIX`).
/// Mutant: the `is_ambiguous` check removed from `Known::service_with` makes the second assertion red.
#[test]
fn a_bare_service_next_to_service_exe_refuses_the_module_with_its_own_cause() {
    let b = tempfile::tempdir().unwrap();
    folder(b.path(), "groups", "0.1.0");
    let m = load(b.path(), Path::new("/nonexistent"), &SystemPackaged::new());
    let k = m.find("groups").unwrap();
    let dir = b.path().join("groups");
    // only `service.exe`: the file that is checked and started
    std::fs::write(dir.join("service.exe"), "x").unwrap();
    assert_eq!(k.service_with(".exe"), Ok(dir.join("service.exe")));
    // a bare `service` next to it: never accepted, whatever it is (file, link target, folder)
    std::fs::write(dir.join("service"), "y").unwrap();
    assert_eq!(k.service_with(".exe"), Err("ambiguous-service"));
    // only the bare `service`: not the program file where files have an ending, and refused as well (never started in its place)
    std::fs::remove_file(dir.join("service.exe")).unwrap();
    assert_eq!(
        k.service_with(".exe"),
        Err("ambiguous-service"),
        "a file without the ending is never accepted"
    );
    // without an ending (Unix) the bare name IS the program file: nothing is ambiguous
    std::fs::write(dir.join("service.exe"), "x").unwrap();
    assert_eq!(k.service_with(""), Ok(dir.join("service")));
}
