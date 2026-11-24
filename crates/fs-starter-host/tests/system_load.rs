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
    assert_eq!(r.entries().len(), 6);
    // the default check (nothing bound) refuses a clean one too
    mode(&modules.join("capture/catalog/commands.json"), 0o644);
    let mut r = Registry::new();
    let refused = r.load_packages(&modules, &Unbound).unwrap();
    assert_eq!(refused.len(), 1);
    assert_eq!(r.entries().len(), 6);
}

/// The check looks at the file that is really read, not at a path made from the module name (review S1): a world-writable folder
/// of someone else, named to `load_packages`, is refused although the verifier's own modules folder is clean.
#[test]
fn a_foreign_world_writable_folder_is_not_accepted_as_a_package() {
    let (t, _modules, v) = world();
    let foreign = t.path().join("foreign");
    let cat = foreign.join("capture/catalog");
    fs::create_dir_all(&cat).unwrap();
    fs::write(cat.join("commands.json"), capture()).unwrap();
    mode(&cat.join("commands.json"), 0o644);
    for d in [&cat, &foreign.join("capture")] {
        mode(d, 0o755);
    }
    mode(&foreign, 0o777);
    let mut r = Registry::new();
    let refused = r.load_packages(&foreign, &v).unwrap();
    assert_eq!(refused, [("capture".to_owned(), vec![Refusal::NotTrusted])]);
    assert_eq!(r.entries().len(), 6);
    // the same folder, closed, is still no system place of this verifier
    mode(&foreign, 0o755);
    let mut r = Registry::new();
    assert_eq!(r.load_packages(&foreign, &v).unwrap().len(), 1);
}

/// A link in the way (here the modules place itself) refuses the file; a file swapped after the open is not the one that was checked.
#[test]
fn a_link_in_the_way_and_a_swapped_file_are_refused() {
    use fs_starter_host::registry::Verifier;
    let (t, modules, _v) = world();
    let uid = fs::metadata(&modules).unwrap().uid();
    let set = support::sets::set("capture", &["capture.shot"]);
    let file = modules.join("capture/catalog/commands.json");
    let verifier = |root: &Path| {
        let places = SystemPlaces::for_test(vec![root.to_path_buf()], uid);
        SystemPackaged::for_test(places, root.to_path_buf())
    };
    // clean: accepted through the handle
    let open = fs::File::open(&file).unwrap();
    assert!(verifier(&modules).verify_file(&set, &file, &open));
    // swapped after the open: the handle is no longer the file at the path
    let other = modules.join("capture/catalog/other.json");
    fs::write(&other, capture()).unwrap();
    mode(&other, 0o644);
    fs::rename(&other, &file).unwrap();
    assert!(!verifier(&modules).verify_file(&set, &file, &open));
    // a link as the place: refused
    let link = t.path().join("link");
    std::os::unix::fs::symlink(&modules, &link).unwrap();
    let through = link.join("capture/catalog/commands.json");
    let open = fs::File::open(&through).unwrap();
    assert!(!verifier(&link).verify_file(&set, &through, &open));
}
