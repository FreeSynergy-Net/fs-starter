//! `Trust::Packaged` of a manifest only if the file passes the system check (`SystemPlaces`): `XDG_DATA_DIRS` is set by the user and
//! proves nothing alone. A failing file is still found, shown as `LocalUnchecked` ("local, not checked").
//!
//! Mutants (each must turn a test red): the check left out (`Place::System` alone gives `Packaged`); the check result inverted.
#![cfg(unix)]

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use fs_starter::{ManifestSource, Place, ProgramName, SystemPlaces, Trust, XdgManifests};

const GOOD: &str = r#"<component type="addon"><id>x</id><extends>net.freesynergy.Demo</extends><custom>
<value key="fs:kind">frontend</value><value key="fs:frontend-id">simple</value><value key="fs:exec">fs-demo-simple</value></custom></component>"#;

fn mode(p: &Path, m: u32) {
    fs::set_permissions(p, fs::Permissions::from_mode(m)).unwrap();
}

/// `place/freesynergy/frontends/demo/simple.metainfo.xml`, all closed (0755/0644). Returns (guard, place, frontends folder, uid).
fn world() -> (tempfile::TempDir, PathBuf, PathBuf, u32) {
    let tmp = tempfile::tempdir().unwrap();
    let place = tmp.path().join("place");
    let front = place.join("freesynergy/frontends");
    let dir = front.join("demo");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("simple.metainfo.xml"), GOOD).unwrap();
    for d in [tmp.path(), &place, &place.join("freesynergy"), &front, &dir] {
        mode(d, 0o755);
    }
    mode(&dir.join("simple.metainfo.xml"), 0o644);
    let uid = fs::metadata(&dir).unwrap().uid();
    (tmp, place, front, uid)
}

fn trust(front: &Path, places: SystemPlaces) -> Option<(Place, Trust)> {
    let src = XdgManifests {
        user: None,
        system: vec![front.to_owned()],
        max_bytes: 64 * 1024,
        places,
    };
    let l = src.frontends(&ProgramName::parse("demo").unwrap());
    l.found.first().map(|f| (f.place, f.trust))
}

fn at(place: &Path, uid: u32) -> SystemPlaces {
    SystemPlaces::for_test(vec![place.to_owned()], uid)
}

#[test]
fn a_closed_system_file_is_packaged() {
    let (_t, place, front, uid) = world();
    assert_eq!(
        trust(&front, at(&place, uid)),
        Some((Place::System, Trust::Packaged))
    );
}

#[test]
fn a_folder_the_user_can_write_is_never_packaged() {
    let (_t, place, front, uid) = world();
    mode(&front.join("demo"), 0o775);
    assert_eq!(
        trust(&front, at(&place, uid)),
        Some((Place::System, Trust::LocalUnchecked))
    );
    mode(&front.join("demo"), 0o755);
    mode(&front.join("demo/simple.metainfo.xml"), 0o666);
    assert_eq!(
        trust(&front, at(&place, uid)),
        Some((Place::System, Trust::LocalUnchecked))
    );
}

#[test]
fn a_wrong_owner_is_never_packaged() {
    let (_t, place, front, uid) = world();
    assert_eq!(
        trust(&front, at(&place, uid.wrapping_add(1))),
        Some((Place::System, Trust::LocalUnchecked))
    );
}

#[test]
fn a_symlinked_file_or_folder_is_never_packaged() {
    let (t, place, front, uid) = world();
    let real = front.join("demo/simple.metainfo.xml");
    fs::rename(&real, t.path().join("elsewhere.xml")).unwrap();
    symlink(t.path().join("elsewhere.xml"), &real).unwrap();
    assert_eq!(
        trust(&front, at(&place, uid)),
        Some((Place::System, Trust::LocalUnchecked))
    );
    // the folder is a link
    let (t, place, front, uid) = world();
    fs::rename(front.join("demo"), t.path().join("moved")).unwrap();
    symlink(t.path().join("moved"), front.join("demo")).unwrap();
    assert_eq!(
        trust(&front, at(&place, uid)),
        Some((Place::System, Trust::LocalUnchecked))
    );
}

#[test]
fn a_folder_outside_every_system_place_is_never_packaged() {
    let (t, _place, front, uid) = world();
    let other = t.path().join("other-place");
    fs::create_dir(&other).unwrap();
    mode(&other, 0o755);
    assert_eq!(
        trust(&front, at(&other, uid)),
        Some((Place::System, Trust::LocalUnchecked))
    );
}

/// The way in by the environment: a user who sets `XDG_DATA_DIRS` to a folder of his own gets the standard check, which fails
/// (a temp folder is neither below `/usr/share/freesynergy` nor root's).
#[test]
fn xdg_data_dirs_set_by_the_user_never_gives_packaged() {
    let (t, _place, _front, _uid) = world();
    let src = XdgManifests::from_vars(None, None, Some(OsString::from(t.path().join("place"))));
    let l = src.frontends(&ProgramName::parse("demo").unwrap());
    assert_eq!(l.found.len(), 1, "found, but...");
    assert_eq!(l.found[0].trust, Trust::LocalUnchecked);
}
