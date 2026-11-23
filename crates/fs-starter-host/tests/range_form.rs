//! V-5: the host reads a need of a contract in ONE way only, `name@<req>` (cargo grammar, comma between comparisons; the types of
//! `fs-types::interface`). The blank form (`groups >=0.1 <0.2`, `groups 0.1`) is refused with a clear message, never read, never
//! taken for "any version". No second notation (decision 23.11.2026).

mod support;
use std::path::Path;

use fs_starter_host::registry::SystemPackaged;
use fs_starter_host::sources::{Limits, Modules, StarterSources};
use support::install::{GOOD_FORMS, commands};

/// The version text of the module in this test (the shape the registry reads today).
const MODULE_VERSION: &str = "0.1.0";

fn sources() -> (tempfile::TempDir, StarterSources) {
    let b = tempfile::tempdir().unwrap();
    let catalog = b.path().join("groups").join("catalog");
    std::fs::create_dir_all(&catalog).unwrap();
    std::fs::write(
        catalog.join("commands.json"),
        commands("groups", MODULE_VERSION),
    )
    .unwrap();
    std::fs::write(catalog.join("forms.json"), GOOD_FORMS).unwrap();
    let m = Modules::load(b.path(), Path::new("/nonexistent"), &SystemPackaged::new());
    (b, StarterSources::new(m, Limits::default()))
}

#[test]
fn the_blank_form_is_refused_with_a_clear_message() {
    let (_b, s) = sources();
    for text in [
        "groups >=0.1 <0.2",
        "groups >=0.1.0 <0.2.0",
        "groups >=0.1.0, <0.2.0",
        "groups 0.1",
        "groups",
    ] {
        let c = s
            .declared(text)
            .err()
            .unwrap_or_else(|| panic!("`{text}` was read: the blank form must be refused"));
        assert_eq!(c.code, "host-notice-data-contract", "{text}");
        let param = |k: &str| {
            c.params
                .iter()
                .find(|(n, _)| n == k)
                .map(|(_, v)| v.clone())
        };
        assert_eq!(param("cause").as_deref(), Some("range"), "{text}");
        let reason = param("reason").unwrap_or_default();
        assert!(
            reason.contains('@'),
            "`{text}`: the message must name the way it is written (`name@<req>`), got `{reason}`"
        );
    }
}

#[test]
fn the_one_form_is_read_and_compared() {
    let (_b, s) = sources();
    assert!(s.declared("groups@>=0.1.0, <0.2.0").is_ok());
    for text in [
        "groups@>=0.2.0, <0.3.0",
        "groups@>=0.0.1, <0.1.0",
        "groups@>0.1.0",
    ] {
        let c = s.declared(text).expect_err("outside the need");
        assert_eq!(c.code, "host-notice-data-contract", "{text}");
    }
    assert_eq!(
        s.declared("tasks@>=0.1.0, <0.2.0").err().unwrap().code,
        "host-notice-source-unknown"
    );
}
