//! K13-K15 of ENTWURF section 5: reading manifests and the two XDG levels.
mod support;

use std::fs;
use std::path::Path;

use fs_starter::{
    InterfaceReq, MAX_MANIFEST_BYTES, ManifestSource, Place, ProgramName, Trust, Version,
    XdgManifests, read_manifest,
};
use support::text;

/// The real manifest of activities (`packaging/frontends/simple.metainfo.xml`, fs-activities da58ed4), except that `fs:contract` and `fs:requires` are written in the one form `name@<req>` (V-5).
const ACTIVITIES_SIMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!-- The manifest of the frontend `simple` (ADR 0094 draft, item 2: AppStream addon, minimal). Stage 1: same release package as the program. -->
<component type="addon">
  <id>net.freesynergy.Activities.simple</id>
  <name>Simple</name>
  <name xml:lang="de">Einfach</name>
  <summary>The standard frontend of FreeSynergy Activities</summary>
  <summary xml:lang="de">Das Standard-Frontend von FreeSynergy Activities</summary>
  <metadata_license>CC0-1.0</metadata_license>
  <project_license>MIT OR Apache-2.0</project_license>
  <url type="homepage">https://github.com/FreeSynergy/fs-activities</url>
  <developer id="net.freesynergy"><name>FreeSynergy</name></developer>
  <extends>net.freesynergy.Activities</extends>
  <requires>
    <id version="0.0.2" compare="ge">net.freesynergy.Activities</id>
  </requires>
  <custom>
    <value key="fs:kind">frontend</value>
    <value key="fs:frontend-id">simple</value>
    <value key="fs:exec">fs-activities-simple</value>
    <value key="fs:contract">history@0.1.0</value>
    <value key="fs:requires">history@^0.1.0</value>
  </custom>
  <releases>
    <release version="0.0.2" date="2026-11-17"/>
  </releases>
</component>
"#;

/// A manifest with the given parts (all valid by default).
fn xml(
    kind: &str,
    id: &str,
    extends: &str,
    exec: &str,
    requires: &str,
    program_req: &str,
) -> String {
    format!(
        r#"<component type="addon"><id>x</id><extends>{extends}</extends>{program_req}<custom>
<value key="fs:kind">{kind}</value><value key="fs:frontend-id">{id}</value><value key="fs:exec">{exec}</value>{requires}</custom></component>"#
    )
}

fn good() -> String {
    xml(
        "frontend",
        "simple",
        "net.freesynergy.Demo",
        "fs-demo-simple",
        r#"<value key="fs:requires">demo@^1.0.0</value>"#,
        "",
    )
}

/// K13: the real manifest of activities is read.
#[test]
fn k13_real_manifest() {
    let m = read_manifest(ACTIVITIES_SIMPLE, Path::new("/x/simple.metainfo.xml")).expect("reads");
    assert_eq!(m.id.as_str(), "simple");
    assert_eq!(m.extends, "net.freesynergy.Activities");
    assert_eq!(m.exec, "fs-activities-simple");
    assert_eq!(
        m.requires,
        vec!["history@^0.1.0".parse::<InterfaceReq>().expect("parses")]
    );
    assert_eq!(m.min_program, Some(Version::new(0, 0, 2)));
}

fn field(xml: &str, file: &str) -> String {
    let e = read_manifest(xml, Path::new(file)).expect_err("must be refused");
    assert_eq!(e.code(), fs_starter::StarterCode::ManifestInvalid);
    assert_eq!(text(&e, "file").as_deref(), Some(file));
    text(&e, "field").expect("field")
}

/// K14: every broken field names itself.
#[test]
fn k14_broken_fields() {
    let f = "/x/simple.metainfo.xml";
    assert!(read_manifest(&good(), Path::new(f)).is_ok());
    assert_eq!(field("not xml <", f), "xml");
    assert_eq!(
        field(
            &good().replace(r#"type="addon""#, r#"type="desktop-application""#),
            f
        ),
        "component"
    );
    assert_eq!(
        field(&good().replace(">frontend<", ">backend<"), f),
        "fs:kind"
    );
    assert_eq!(
        field(&good().replace(">simple<", ">Bad_Id<"), f),
        "fs:frontend-id"
    );
    assert_eq!(
        field(
            &good().replace(">simple<", ">core<"),
            "/x/core.metainfo.xml"
        ),
        "fs:frontend-id"
    );
    assert_eq!(field(&good(), "/x/other.metainfo.xml"), "file-name");
    assert_eq!(
        field(
            &good().replace("<extends>net.freesynergy.Demo</extends>", ""),
            f
        ),
        "extends"
    );
    assert_eq!(
        field(&good().replace(">fs-demo-simple<", ">/bin/sh<"), f),
        "fs:exec"
    );
    // ONE way of writing a need (`name@<req>`): the bare name and the blank form of before are refused
    for bad in [
        "demo",
        "demo 1.0",
        "demo@^1.0",
        "demo@*",
        "demo@>=x",
        "Demo@^1.0.0",
    ] {
        assert_eq!(
            field(&good().replace(">demo@^1.0.0<", &format!(">{bad}<")), f),
            "fs:requires",
            "{bad}"
        );
    }
    let bad_cmp =
        r#"<requires><id version="0.0.2" compare="eq">net.freesynergy.Demo</id></requires>"#;
    assert_eq!(
        field(
            &xml(
                "frontend",
                "simple",
                "net.freesynergy.Demo",
                "fs-demo-simple",
                "",
                bad_cmp
            ),
            f
        ),
        "requires"
    );
    let ok_cmp =
        r#"<requires><id version="0.0.2" compare="ge">net.freesynergy.Demo</id></requires>"#;
    let m = read_manifest(
        &xml(
            "frontend",
            "simple",
            "net.freesynergy.Demo",
            "fs-demo-simple",
            "",
            ok_cmp,
        ),
        Path::new(f),
    )
    .expect("reads");
    assert_eq!(m.min_program, Some(Version::new(0, 0, 2)));
}

/// The test folder plays the system place (its owner is the user running the test). Without unix nothing is packaged.
fn places(root: &Path) -> fs_starter::SystemPlaces {
    #[cfg(unix)]
    let uid = std::os::unix::fs::MetadataExt::uid(&fs::metadata(root).expect("meta"));
    #[cfg(not(unix))]
    let uid = 0;
    fs_starter::SystemPlaces::for_test(vec![root.to_path_buf()], uid)
}

fn packaged() -> Trust {
    if cfg!(unix) {
        Trust::Packaged
    } else {
        Trust::LocalUnchecked
    }
}

fn put(dir: &Path, name: &str, body: &str) {
    fs::create_dir_all(dir).expect("mkdir");
    fs::write(dir.join(name), body).expect("write");
}

/// K15: the user level wins over the system level; trust follows the level; a bad file is listed, the rest is found.
#[test]
fn k15_xdg_levels() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (user, system) = (tmp.path().join("user"), tmp.path().join("system"));
    let program = ProgramName::parse("demo").expect("name");
    let both = good();
    put(
        &user.join("demo"),
        "simple.metainfo.xml",
        &both.replace("fs-demo-simple", "fs-demo-user"),
    );
    put(&system.join("demo"), "simple.metainfo.xml", &both);
    put(
        &system.join("demo"),
        "classic.metainfo.xml",
        &both.replace(">simple<", ">classic<"),
    );
    put(&system.join("demo"), "broken.metainfo.xml", "not xml <");
    put(&system.join("demo"), "readme.txt", "ignored");

    let src = XdgManifests {
        max_bytes: MAX_MANIFEST_BYTES,
        user: Some(user.clone()),
        system: vec![system.clone()],
        places: places(&system),
    };
    let l = src.frontends(&program);
    let simple = l
        .found
        .iter()
        .find(|f| f.manifest.id.as_str() == "simple")
        .expect("simple");
    assert_eq!(
        (simple.place, simple.trust, simple.manifest.exec.as_str()),
        (Place::User, Trust::LocalUnchecked, "fs-demo-user")
    );
    let classic = l
        .found
        .iter()
        .find(|f| f.manifest.id.as_str() == "classic")
        .expect("classic");
    assert_eq!((classic.place, classic.trust), (Place::System, packaged()));
    assert_eq!(l.found.len(), 2, "one per id");
    assert_eq!(l.refused.len(), 1);
    assert_eq!(text(&l.refused[0], "field").as_deref(), Some("xml"));

    // Only the system level.
    let only = XdgManifests {
        max_bytes: MAX_MANIFEST_BYTES,
        user: None,
        system: vec![system.clone()],
        places: places(&system),
    }
    .frontends(&program);
    assert!(
        only.found
            .iter()
            .all(|f| f.trust == packaged() && f.place == Place::System)
    );
    // Missing folders: empty, no refusal.
    let none = XdgManifests {
        max_bytes: MAX_MANIFEST_BYTES,
        user: Some(tmp.path().join("nope")),
        system: vec![tmp.path().join("nada")],
        places: places(tmp.path()),
    }
    .frontends(&program);
    assert!(none.found.is_empty() && none.refused.is_empty());
}
