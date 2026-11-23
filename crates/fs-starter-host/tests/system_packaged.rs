//! `SystemPackaged`: the module set counts as signed only if the package manager brought it (system folder of the modules, root's, no
//! link, not writable). Everything else stays refused, as `Unbound` does.
//!
//! Mutants: verify returns true always; the module-name check left out; the path check left out (each turns a test red).
#![cfg(unix)]

mod support;
use support::sets::set;

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use fs_starter::SystemPlaces;
use fs_starter_host::registry::{Actor, Refusal, Registry, SystemPackaged, Verifier};

fn mode(p: &Path, m: u32) {
    fs::set_permissions(p, fs::Permissions::from_mode(m)).unwrap();
}

/// `modules/capture/catalog/commands.json`; returns (guard, modules, uid).
fn world() -> (tempfile::TempDir, PathBuf, u32) {
    let tmp = tempfile::tempdir().unwrap();
    let modules = tmp.path().join("modules");
    let cat = modules.join("capture").join("catalog");
    fs::create_dir_all(&cat).unwrap();
    fs::write(cat.join("commands.json"), "{}").unwrap();
    for d in [&modules, &modules.join("capture"), &cat] {
        mode(d, 0o755);
    }
    mode(&cat.join("commands.json"), 0o644);
    let uid = fs::metadata(&modules).unwrap().uid();
    (tmp, modules, uid)
}

fn verifier(modules: &Path, uid: u32) -> SystemPackaged {
    let places = SystemPlaces::for_test(vec![modules.to_path_buf()], uid);
    SystemPackaged::for_test(places, modules.to_path_buf())
}

#[test]
fn a_clean_system_module_is_accepted_and_registered() {
    let (_t, modules, uid) = world();
    let v = verifier(&modules, uid);
    assert!(v.verify(&set("capture", &["capture.shot"])));
    let mut r = Registry::new();
    r.add_package(Actor::Person, set("capture", &["capture.shot"]), &v)
        .unwrap();
    assert_eq!(r.entries().len(), 5);
}

#[test]
fn a_user_writable_folder_a_link_and_a_wrong_owner_are_refused() {
    let (_t, modules, uid) = world();
    let file = modules.join("capture/catalog/commands.json");
    // wrong owner
    assert!(!verifier(&modules, uid.wrapping_add(1)).verify(&set("capture", &["capture.shot"])));
    // writable file
    mode(&file, 0o666);
    assert!(!verifier(&modules, uid).verify(&set("capture", &["capture.shot"])));
    mode(&file, 0o644);
    // writable catalog folder
    mode(&modules.join("capture/catalog"), 0o777);
    assert!(!verifier(&modules, uid).verify(&set("capture", &["capture.shot"])));
    mode(&modules.join("capture/catalog"), 0o755);
    // symlinked module folder
    symlink(modules.join("capture"), modules.join("zip")).unwrap();
    assert!(!verifier(&modules, uid).verify(&set("zip", &["zip.x"])));
    // the registry then says NotTrusted, nothing added
    let mut r = Registry::new();
    let why = r
        .add_package(
            Actor::Person,
            set("zip", &["zip.x"]),
            &verifier(&modules, uid),
        )
        .unwrap_err();
    assert_eq!(why, [Refusal::NotTrusted]);
    assert_eq!(r.entries().len(), 4);
}

#[test]
fn a_module_that_is_not_in_the_system_folder_is_refused() {
    let (t, modules, uid) = world();
    let v = verifier(&modules, uid);
    assert!(!v.verify(&set("missing", &["missing.x"])));
    // a clean module folder outside the system folder is not accepted by the module name
    let outside = t.path().join("user");
    fs::create_dir_all(outside.join("evil/catalog")).unwrap();
    for name in ["../user/evil", "..", "a/b", ""] {
        let mut forged = set("capture", &["capture.shot"]);
        forged.module = name.to_owned();
        assert!(!v.verify(&forged), "{name}");
    }
    // clean nested folders that only the name check keeps out (`.` and `a/b` are no module names)
    for nested in ["catalog", "a/b/catalog"] {
        let d = modules.join(nested);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("commands.json"), "{}").unwrap();
        let mut walk = d.clone();
        while walk != modules {
            mode(&walk, 0o755);
            walk.pop();
        }
        mode(&d.join("commands.json"), 0o644);
    }
    for name in [".", "a/b"] {
        let mut forged = set("capture", &["capture.shot"]);
        forged.module = name.to_owned();
        assert!(!v.verify(&forged), "{name}");
    }
    // the real system folder: a test module is not there
    assert!(!SystemPackaged::new().verify(&set("capture", &["capture.shot"])));
}
