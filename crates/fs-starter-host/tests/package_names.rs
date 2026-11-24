//! One package name for every format: the deb (`[package.metadata.deb]`) and the rpm (`[package.metadata.generate-rpm]`) are both called `fs-starter`
//! (the name of the program, `name:` of `release.yml`), and both take over the earlier package `fs-starter-host` (deb: Replaces/Conflicts/Provides,
//! rpm: Obsoletes). The built packages are checked the same way by `package-probe.yml`. Tests: example-based. Mutants: the deb `name` left out (cargo-deb
//! takes the crate name again); a table without Replaces, Conflicts, Provides or Obsoletes; `name:` of `release.yml` changed.

use std::fs;

const PROGRAM: &str = "fs-starter";
const OLD: &str = "fs-starter-host";

fn manifest() -> toml::Table {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    fs::read_to_string(path)
        .expect("read Cargo.toml")
        .parse()
        .expect("parse Cargo.toml")
}

fn table<'a>(root: &'a toml::Table, keys: &[&str]) -> &'a toml::Table {
    let mut t = root;
    for k in keys {
        t = t
            .get(*k)
            .and_then(|v| v.as_table())
            .unwrap_or_else(|| panic!("Cargo.toml lacks [{}]", keys.join(".")));
    }
    t
}

fn text<'a>(t: &'a toml::Table, key: &str) -> &'a str {
    t.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("key `{key}` missing or not a string"))
}

#[test]
fn deb_and_rpm_carry_the_same_name_as_the_program() {
    let m = manifest();
    let deb = text(table(&m, &["package", "metadata", "deb"]), "name");
    let rpm = text(table(&m, &["package", "metadata", "generate-rpm"]), "name");
    assert_eq!(deb, PROGRAM, "deb name");
    assert_eq!(rpm, PROGRAM, "rpm name");
}

#[test]
fn both_formats_take_over_the_earlier_package() {
    let m = manifest();
    let deb = table(&m, &["package", "metadata", "deb"]);
    for key in ["replaces", "conflicts", "provides"] {
        assert!(
            text(deb, key).split(',').any(|p| p.trim() == OLD),
            "deb `{key}` must name {OLD}"
        );
    }
    let rpm = table(&m, &["package", "metadata", "generate-rpm", "obsoletes"]);
    assert!(rpm.contains_key(OLD), "rpm obsoletes must name {OLD}");
}

#[test]
fn the_release_caller_uses_the_same_name() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../.github/workflows/release.yml"
    );
    let yml = fs::read_to_string(path).expect("read release.yml");
    assert!(
        yml.lines().any(|l| l.trim() == format!("name: {PROGRAM}")),
        "release.yml: `name: {PROGRAM}` missing"
    );
}
