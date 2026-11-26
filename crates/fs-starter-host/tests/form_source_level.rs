//! Contract test. Pattern: Contract Test - the level the rpm offers as `Provides: freesynergy-form-source = N` (`[package.metadata.generate-rpm.provides]`
//! of `Cargo.toml`) equals the level this host is built against (`FORM_SOURCE_LEVEL` of fs-interfaces), so the package never promises another level than the code.
//! Tests: example-based. Mutants: the number in `Cargo.toml` changed alone; the provides entry removed.

use fs_interfaces::forms::FORM_SOURCE_LEVEL;

#[test]
fn rpm_provides_the_form_source_level_of_the_code() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let m: toml::Table = std::fs::read_to_string(path)
        .expect("read Cargo.toml")
        .parse()
        .expect("parse Cargo.toml");
    let value = m["package"]["metadata"]["generate-rpm"]["provides"]["freesynergy-form-source"]
        .as_str()
        .expect("provides entry is a string");
    let level: u32 = value
        .strip_prefix("= ")
        .unwrap_or_else(|| panic!("`{value}` must be `= N`"))
        .parse()
        .expect("N is a whole number");
    assert_eq!(level, FORM_SOURCE_LEVEL);
}
