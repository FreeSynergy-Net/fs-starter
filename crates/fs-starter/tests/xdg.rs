//! N3 of the review: size limit, DTD, relative `XDG_DATA_HOME`.
mod support;

use std::ffi::OsString;
use std::fs;

use fs_starter::{ManifestSource, ProgramName, XdgManifests};
use support::text;

const GOOD: &str = r#"<component type="addon"><id>x</id><extends>net.freesynergy.Demo</extends><custom>
<value key="fs:kind">frontend</value><value key="fs:frontend-id">simple</value><value key="fs:exec">fs-demo-simple</value></custom></component>"#;

fn level(body: &str) -> (tempfile::TempDir, XdgManifests) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("demo");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("simple.metainfo.xml"), body).expect("write");
    let src = XdgManifests::from_vars(Some(tmp.path().into()), None, Some(OsString::new()));
    // from_vars adds `freesynergy/frontends`; point the user level at the folder directly.
    let src = XdgManifests {
        user: Some(tmp.path().to_owned()),
        system: Vec::new(),
        ..src
    };
    (tmp, src)
}

fn field_of(src: &XdgManifests) -> Option<String> {
    let l = src.frontends(&ProgramName::parse("demo").expect("name"));
    assert_eq!(l.found.len() + l.refused.len(), 1);
    l.refused.first().and_then(|r| text(r, "field"))
}

/// A DOCTYPE with entities (billion laughs) is refused as `xml`.
#[test]
fn dtd_is_refused() {
    let bomb = format!(
        r#"<?xml version="1.0"?><!DOCTYPE c [<!ENTITY a "aaaa"><!ENTITY b "&a;&a;&a;&a;">]>{GOOD}"#
    );
    let (_t, src) = level(&bomb);
    assert_eq!(field_of(&src).as_deref(), Some("xml"));
    let (_t, ok) = level(GOOD);
    assert_eq!(field_of(&ok), None);
}

/// A file above the limit is refused as `size`; at the limit it is read.
#[test]
fn size_limit() {
    let (_t, mut src) = level(GOOD);
    src.max_bytes = GOOD.len() as u64;
    assert_eq!(field_of(&src), None, "exactly at the limit");
    src.max_bytes = GOOD.len() as u64 - 1;
    assert_eq!(field_of(&src).as_deref(), Some("size"));
    let (_t, big) = level(&format!("{GOOD}{}", " ".repeat(70_000)));
    assert_eq!(
        field_of(&big).as_deref(),
        Some("size"),
        "the default is 64 KiB"
    );
}

/// A relative `XDG_DATA_HOME` is ignored (back to `HOME`); an absolute one is used; relative data dirs are dropped.
/// Unix only: the paths are written as Unix paths (`/data` is not absolute on Windows); the XDG variables are a Linux convention.
#[cfg(unix)]
#[test]
fn relative_data_home() {
    let v = |s: &str| Some(OsString::from(s));
    let rel = XdgManifests::from_vars(v("relative/share"), v("/home/u"), None);
    assert_eq!(
        rel.user,
        Some("/home/u/.local/share/freesynergy/frontends".into())
    );
    let abs = XdgManifests::from_vars(v("/data"), v("/home/u"), v("rel:/usr/share"));
    assert_eq!(abs.user, Some("/data/freesynergy/frontends".into()));
    assert_eq!(
        abs.system,
        vec![std::path::PathBuf::from("/usr/share/freesynergy/frontends")]
    );
    assert_eq!(
        XdgManifests::from_vars(v("relative"), None, None).user,
        None
    );
}
