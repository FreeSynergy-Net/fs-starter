//! K32-K34 (N1 of the review): `fs:default` of a manifest.
mod support;

use std::path::Path;

use fs_starter::{Level, Listing, Trust, read_manifest};
use support::{World, code_text, fid, found};

/// A manifest with `fs:default` = `value` (or none).
fn xml(default: Option<&str>) -> String {
    let d = default.map_or(String::new(), |v| {
        format!(r#"<value key="fs:default">{v}</value>"#)
    });
    format!(
        r#"<component type="addon"><id>x</id><extends>net.freesynergy.Demo</extends><custom>
<value key="fs:kind">frontend</value><value key="fs:frontend-id">simple</value><value key="fs:exec">fs-demo-simple</value>{d}</custom></component>"#
    )
}

/// K32: `true`, `false`, missing, and `yes` (refused with `field=fs:default`).
#[test]
fn k32_read_default() {
    let read = |d| read_manifest(&xml(d), Path::new("/x/simple.metainfo.xml"));
    assert!(read(Some("true")).expect("reads").default);
    assert!(!read(Some("false")).expect("reads").default);
    assert!(!read(None).expect("reads").default);
    let e = read(Some("yes")).expect_err("refused");
    assert_eq!(
        code_text(&e.erase(), "field").as_deref(),
        Some("fs:default")
    );
}

/// A world with frontends `d1` (default), `d2` (default), `p` (the person's choice), `w` (for a word), all fitting.
fn world() -> World {
    let mut w = World::new();
    for id in ["d1", "d2", "p", "w"] {
        w = w.with_frontend(id, Trust::Packaged);
    }
    for f in &mut w.manifests.0.found {
        f.manifest.default = matches!(f.manifest.id.as_str(), "d1" | "d2");
    }
    w
}

/// K33: word > person > default > standard; two defaults: the first plus a hint.
#[test]
fn k33_order() {
    let w = world();
    assert_eq!(w.go(&["--frontend", "w"]).code(), 0);
    assert_eq!(w.opened()[0].0, World::exe("w"));

    let mut w = world();
    w.pref.0 = Some(fid("p"));
    assert_eq!(w.go(&[]).code(), 0);
    assert_eq!(w.opened()[0].0, World::exe("p"), "person beats default");

    let w = world();
    assert_eq!(w.go(&[]).code(), 0);
    assert_eq!(
        w.opened()[0].0,
        World::exe("d1"),
        "first default beats standard"
    );
    let n = w
        .tell
        .note("starter-error-manifest-invalid")
        .expect("second default told");
    assert_eq!(n.level, Level::Hint);
    assert_eq!(code_text(&n.code, "field").as_deref(), Some("fs:default"));
    assert_eq!(
        code_text(&n.code, "file").as_deref(),
        Some("/share/d2.metainfo.xml")
    );

    let mut w = World::new();
    w.manifests.0 = Listing {
        found: vec![found("simple", Trust::Packaged)],
        refused: Vec::new(),
    };
    assert_eq!(w.go(&[]).code(), 0);
    assert_eq!(
        w.opened()[0].0,
        World::exe("simple"),
        "no default: the standard"
    );
}

/// K34: a default frontend that answers `--check` with 3 falls back to the standard, with a hint.
#[test]
fn k34_default_checked() {
    let mut w = world();
    w.launcher.checks.push((World::exe("d1"), Some(3)));
    assert_eq!(w.go(&[]).code(), 0);
    assert_eq!(w.launcher.checked.borrow().as_slice(), [World::exe("d1")]);
    assert_eq!(w.opened()[0].0, World::exe("simple"));
    let n = w
        .tell
        .note("starter-error-frontend-contract")
        .expect("told");
    assert_eq!(code_text(&n.code, "frontend").as_deref(), Some("d1"));
    assert_eq!(code_text(&n.code, "fallback").as_deref(), Some("simple"));
}
