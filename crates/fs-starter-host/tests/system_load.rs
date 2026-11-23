//! `Registry::load_packages`, the caller of `add_package` (used by `--list-commands` on the system folder of the modules): a folder
//! comes in only if the verifier accepts it. Mutant: the verifier ignored (loaded as bundled) turns the refusal test red.
#![cfg(unix)]

mod support;
use support::sets::{capture, ids};

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use fs_starter::SystemPlaces;
use fs_starter_host::registry::{Refusal, Registry, SystemPackaged, Unbound};

fn mode(p: &Path, m: u32) {
    fs::set_permissions(p, fs::Permissions::from_mode(m)).unwrap();
}

fn world() -> (tempfile::TempDir, PathBuf, SystemPackaged) {
    let tmp = tempfile::tempdir().unwrap();
    let modules = tmp.path().join("modules");
    let cat = modules.join("capture/catalog");
    fs::create_dir_all(&cat).unwrap();
    fs::write(cat.join("commands.json"), capture()).unwrap();
    for d in [&modules, &modules.join("capture"), &cat] {
        mode(d, 0o755);
    }
    mode(&cat.join("commands.json"), 0o644);
    let uid = fs::metadata(&modules).unwrap().uid();
    let places = SystemPlaces::for_test(vec![modules.clone()], uid);
    let v = SystemPackaged::for_test(places, modules.clone());
    (tmp, modules, v)
}

#[test]
fn a_clean_system_module_is_loaded_as_a_package() {
    let (_t, modules, v) = world();
    let mut r = Registry::new();
    let refused = r.load_packages(&modules, &v).unwrap();
    assert!(refused.is_empty(), "{refused:?}");
    assert!(ids(&r).iter().any(|i| i.starts_with("capture.")));
}

#[test]
fn a_module_the_verifier_does_not_accept_is_refused_and_not_added() {
    let (_t, modules, v) = world();
    mode(&modules.join("capture/catalog/commands.json"), 0o666);
    let mut r = Registry::new();
    let refused = r.load_packages(&modules, &v).unwrap();
    assert_eq!(refused, [("capture".to_owned(), vec![Refusal::NotTrusted])]);
    assert_eq!(r.entries().len(), 4);
    // the default check (nothing bound) refuses a clean one too
    mode(&modules.join("capture/catalog/commands.json"), 0o644);
    let mut r = Registry::new();
    let refused = r.load_packages(&modules, &Unbound).unwrap();
    assert_eq!(refused.len(), 1);
    assert_eq!(r.entries().len(), 4);
}
