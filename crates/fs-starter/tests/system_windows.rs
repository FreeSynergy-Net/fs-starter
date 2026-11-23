//! The system places on Windows (WIN-HOST WH3): the anchor is the known folder `FOLDERID_ProgramFiles` + `FreeSynergy` (never an
//! environment variable), the check is the module `fs-trusted-path` (no copy here), and a test folder with a closed ACL plays the place.
//! The tests of the check itself (every right, every owner) are those of `fs-trusted-path`; here only what the policy adds.
//!
//! Positive cases need `for_test_as_user` (a test folder is never the system's). The refusing cases need no `for_test`.
#![cfg(windows)]

use std::path::Path;
use std::process::Command;

use fs_starter::{SystemPlaces, SystemRefusal};

fn icacls(path: &Path, args: &[&str]) {
    let out = Command::new("icacls")
        .arg(path)
        .args(args)
        .output()
        .expect("icacls runs");
    assert!(out.status.success(), "icacls {args:?}: {out:?}");
}

/// A folder only Administrators may touch (the runner is elevated, so the folder is its own too), with a file below.
fn closed_place() -> (tempfile::TempDir, std::path::PathBuf, SystemPlaces) {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    icacls(
        dir.path(),
        &["/inheritance:r", "/grant:r", "*S-1-5-32-544:(OI)(CI)F"],
    );
    let file = dir.path().join("groups").join("service.exe");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "x").unwrap();
    let places = SystemPlaces::for_test_as_user(vec![dir.path().to_path_buf()]);
    (dir, file, places)
}

#[test]
fn the_anchor_comes_from_the_known_folder_and_is_one_place() {
    let modules = fs_starter::system_modules_dir().expect("the shell knows Program Files");
    assert!(modules.is_absolute(), "{modules:?}");
    assert!(modules.ends_with(r"FreeSynergy\modules"), "{modules:?}");
    let pf = std::env::var("ProgramFiles").unwrap();
    assert!(
        modules
            .to_string_lossy()
            .to_lowercase()
            .starts_with(&pf.to_lowercase()),
        "{modules:?} is not below {pf} (the CI runner does not change the variable)"
    );
}

#[test]
fn a_path_outside_the_anchor_is_outside_the_system_even_where_the_system_owns_it() {
    let places = SystemPlaces::standard();
    for outside in [
        r"C:\Windows\System32\cmd.exe",
        r"C:\Program Files\other\service.exe",
    ] {
        assert_eq!(
            places.check(Path::new(outside)),
            Err(SystemRefusal::OutsideSystem),
            "{outside}"
        );
    }
    assert_eq!(
        places.check(Path::new(r"modules\x\service.exe")),
        Err(SystemRefusal::NotAbsolute)
    );
    // a program name that is not plain never becomes a folder
    assert_eq!(places.program_dir(r"..\x"), Err(SystemRefusal::NotAbsolute));
}

#[test]
fn a_closed_folder_of_the_place_is_accepted() {
    let (_dir, file, places) = closed_place();
    assert_eq!(places.check(&file), Ok(()));
}

#[test]
fn a_folder_somebody_else_may_write_is_refused_on_every_part_of_the_way() {
    let (dir, file, places) = closed_place();
    // Everyone may write the folder of the module: the file below it is not the system's any more
    icacls(file.parent().unwrap(), &["/grant", "*S-1-1-0:(OI)(CI)W"]);
    assert_eq!(places.check(&file), Err(SystemRefusal::Writable));
    // and Everyone writing the place itself (the anchor) refuses every path in it
    icacls(dir.path(), &["/grant", "*S-1-1-0:(OI)(CI)W"]);
    assert_eq!(places.check(dir.path()), Err(SystemRefusal::Writable));
}

#[test]
fn a_junction_on_the_way_is_refused() {
    let (dir, file, places) = closed_place();
    let link = dir.path().join("linked");
    let out = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(file.parent().unwrap())
        .output()
        .expect("cmd runs");
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        places.check(&link.join("service.exe")),
        Err(SystemRefusal::Symlink)
    );
}
