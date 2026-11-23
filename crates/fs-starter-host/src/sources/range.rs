//! The need a description writes for a contract: `name@<req>` (`groups@>=0.1.0, <0.2.0`). The host compares the version of the module
//! with it (ADR 0104 E3).
//!
//! ONE way of writing, read by the types of `fs-types::interface` (`InterfaceReq`, the grammar of `semver::VersionReq`, a comma between
//! comparisons) and the version of a module by `semver::Version` (three parts: `0.1` is refused with a cause, never guessed). The older blank
//! form (`groups >=0.1 <0.2`, `groups 0.1`) and a bare name are NOT read: they are refused with a message that says how a need is written.
//! There is no second notation and no bridge.
//!
//! Pattern: Specification (a pure predicate over two small values; unreadable text is an error, never read as "any" and never dropped).

use fs_types::interface::InterfaceReq;
pub use semver::{Error, Version};

/// Read a module version: `Major.Minor.Patch`, no pre-release and no build part (as `fs-types::interface`).
///
/// # Errors
/// The text is not a version (a two part `0.1` is one of those).
pub fn version(text: &str) -> Result<Version, String> {
    let v = Version::parse(text.trim()).map_err(|e| e.to_string())?;
    if v.pre.is_empty() && v.build.is_empty() {
        Ok(v)
    } else {
        Err("pre-release and build parts are not allowed".to_owned())
    }
}

/// The need of a description (`name@<req>`), every comparison with all three numbers.
///
/// # Errors
/// The text says why it is not a need; a text without `@` (the blank form, a bare name) names the way a need is written.
pub fn need(text: &str) -> Result<InterfaceReq, String> {
    let req: InterfaceReq = text.trim().parse().map_err(|e| {
        format!("{e}; a need is written `name@<req>`, e.g. `groups@>=0.1.0, <0.2.0`")
    })?;
    if req.is_full() {
        Ok(req)
    } else {
        Err("every comparison needs major.minor.patch, e.g. `groups@>=0.1.0, <0.2.0`".to_owned())
    }
}

/// Does `have` meet the need `text` (`name@<req>`)?
///
/// # Errors
/// The need cannot be read (see [`need`]).
pub fn fits(text: &str, have: &Version) -> Result<bool, String> {
    Ok(need(text)?.req().matches(have))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(t: &str) -> Version {
        version(t).unwrap()
    }

    #[test]
    fn a_version_must_be_inside_every_comparison() {
        let r = "groups@>=0.0.1, <0.1.0";
        assert_eq!(fits(r, &v("0.0.1")), Ok(true));
        assert_eq!(fits(r, &v("0.0.9")), Ok(true));
        assert_eq!(fits(r, &v("0.1.0")), Ok(false));
        assert_eq!(fits(r, &v("0.0.0")), Ok(false));
        assert_eq!(fits("groups@>0.1.0", &v("0.1.0")), Ok(false));
        assert_eq!(fits("groups@<=0.1.0", &v("0.1.0")), Ok(true));
        assert_eq!(fits("groups@=1.0.0", &v("1.0.0")), Ok(true));
    }

    #[test]
    fn a_module_version_has_three_parts() {
        assert_eq!(v("0.0.1"), Version::new(0, 0, 1));
        assert!(version("0.1").is_err());
        assert!(version("1").is_err());
        assert!(version("0.1.0-rc.1").is_err());
        assert!(version("0.1.0+b").is_err());
    }

    #[test]
    fn the_blank_form_and_a_bare_name_are_refused_and_say_how_to_write_it() {
        for old in [
            "groups >=0.1 <0.2",
            "groups >=0.0.1, <0.1.0",
            "groups 0.1",
            "groups",
        ] {
            let why = fits(old, &v("0.1.5")).expect_err(old);
            assert!(why.contains("name@<req>"), "{old}: {why}");
        }
    }

    #[test]
    fn a_need_that_is_not_full_or_not_readable_is_an_error() {
        assert!(fits("groups@>=x", &v("0.1.0")).is_err());
        assert!(fits("groups@>=", &v("0.1.0")).is_err());
        assert!(fits("groups@0.1", &v("0.1.0")).is_err(), "partial version");
        assert!(fits("groups@*", &v("0.1.0")).is_err());
        assert!(fits("Groups@>=0.1.0", &v("0.1.0")).is_err());
    }
}
