//! The check "the package manager brought this" (`SystemPlaces`): one red test per border. The test folder plays the system place; its
//! owner is the user running the test, so "wrong owner" is the same folder asked for another owner.
//!
//! Mutants (each must turn a test red): the symlink check left out; the owner check left out; the writable check left out; the
//! "below a place" check left out; the "no `..`" check left out.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use fs_starter::{SystemPlaces, SystemRefusal};

fn mode(p: &Path, m: u32) {
    fs::set_permissions(p, fs::Permissions::from_mode(m)).unwrap();
}

/// A place `root/` with `root/mod/file`, all 0755/0644, owned by the current user. Returns (guard, root, file, our uid).
fn world() -> (tempfile::TempDir, PathBuf, PathBuf, u32) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("place");
    let dir = root.join("capture");
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("commands.json");
    fs::write(&file, "{}").unwrap();
    for d in [tmp.path(), &root, &dir] {
        mode(d, 0o755);
    }
    mode(&file, 0o644);
    let uid = fs::metadata(&file).unwrap().uid();
    (tmp, root, file, uid)
}

fn places(root: &Path, uid: u32) -> SystemPlaces {
    SystemPlaces::for_test(vec![root.to_path_buf()], uid)
}

#[test]
fn a_root_owned_plain_file_in_a_closed_folder_counts_as_packaged() {
    let (_t, root, file, uid) = world();
    assert_eq!(places(&root, uid).check(&file), Ok(()));
    assert_eq!(places(&root, uid).check(&root), Ok(()));
}

#[test]
fn a_file_a_user_can_write_is_refused() {
    let (_t, root, file, uid) = world();
    for m in [0o664, 0o646] {
        mode(&file, m);
        assert_eq!(
            places(&root, uid).check(&file),
            Err(SystemRefusal::Writable)
        );
    }
    mode(&file, 0o644);
    // a folder on the way that others can write: they can swap the file
    let dir = file.parent().unwrap();
    mode(dir, 0o775);
    assert_eq!(
        places(&root, uid).check(&file),
        Err(SystemRefusal::Writable)
    );
    mode(dir, 0o757);
    assert_eq!(
        places(&root, uid).check(&file),
        Err(SystemRefusal::Writable)
    );
    mode(dir, 0o755);
    mode(&root, 0o777);
    assert_eq!(
        places(&root, uid).check(&file),
        Err(SystemRefusal::Writable)
    );
}

#[test]
fn a_symlink_is_refused_as_file_as_folder_and_as_place() {
    let (t, root, file, uid) = world();
    let link = root.join("capture").join("link.json");
    symlink(&file, &link).unwrap();
    assert_eq!(places(&root, uid).check(&link), Err(SystemRefusal::Symlink));
    let dir_link = root.join("linked");
    symlink(root.join("capture"), &dir_link).unwrap();
    assert_eq!(
        places(&root, uid).check(&dir_link.join("commands.json")),
        Err(SystemRefusal::Symlink)
    );
    // a dangling link is no better
    let dangling = root.join("dangling");
    symlink(t.path().join("nothing"), &dangling).unwrap();
    assert_eq!(
        places(&root, uid).check(&dangling),
        Err(SystemRefusal::Symlink)
    );
    // the place itself a link to a good folder
    let alias = t.path().join("alias");
    symlink(&root, &alias).unwrap();
    assert_eq!(
        places(&alias, uid).check(&alias.join("capture")),
        Err(SystemRefusal::Symlink)
    );
}

#[test]
fn another_owner_is_refused() {
    let (_t, root, file, uid) = world();
    // the system owner is root; the test folder is ours (a real user file in a system place)
    if uid != 0 {
        assert_eq!(
            places(&root, 0).check(&file),
            Err(SystemRefusal::WrongOwner)
        );
    }
    assert_eq!(
        places(&root, uid.wrapping_add(1)).check(&file),
        Err(SystemRefusal::WrongOwner)
    );
}

#[test]
fn a_path_outside_the_places_is_refused_even_if_it_is_clean() {
    let (t, root, file, uid) = world();
    let other = t.path().join("elsewhere");
    fs::create_dir(&other).unwrap();
    mode(&other, 0o755);
    let p = places(&root, uid);
    assert_eq!(p.check(&other), Err(SystemRefusal::OutsideSystem));
    assert_eq!(
        p.check(Path::new("/etc/passwd")),
        Err(SystemRefusal::OutsideSystem)
    );
    // a sibling that only shares the name prefix is not below the place
    let sibling = t.path().join("place-evil");
    fs::create_dir(&sibling).unwrap();
    assert_eq!(p.check(&sibling), Err(SystemRefusal::OutsideSystem));
    // the real places: a user folder is outside
    assert_eq!(
        SystemPlaces::standard().check(&file),
        Err(SystemRefusal::OutsideSystem)
    );
}

#[test]
fn dots_and_relative_paths_are_refused() {
    let (_t, root, file, uid) = world();
    let p = places(&root, uid);
    let up = root
        .join("capture")
        .join("..")
        .join("capture")
        .join("commands.json");
    assert_eq!(p.check(&up), Err(SystemRefusal::NotAbsolute));
    assert_eq!(
        p.check(Path::new("place/capture")),
        Err(SystemRefusal::NotAbsolute)
    );
    // a missing file is Unreadable, not a pass
    assert_eq!(
        p.check(&file.with_file_name("nope.json")),
        Err(SystemRefusal::Unreadable)
    );
}

#[test]
fn the_standard_places_are_the_ones_of_the_rpms() {
    assert_eq!(fs_starter::STARTER_DIR, "/usr/bin");
    assert_eq!(fs_starter::MODULES_DIR, "/usr/libexec/freesynergy/modules");
    assert_eq!(fs_starter::PROGRAMS_DIR, "/usr/share/freesynergy/programs");
    // a program name is a plain folder name: no way out of the programs folder
    let std = SystemPlaces::standard();
    for bad in ["", "..", "a/b", "../etc", "A", "x.y", "/abs"] {
        assert_eq!(
            std.program_dir(bad),
            Err(SystemRefusal::NotAbsolute),
            "{bad}"
        );
    }
}
