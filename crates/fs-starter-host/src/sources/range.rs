//! The range a description writes after the name of a contract (`groups >=0.0.1, <0.1`): the host compares the version of the module
//! with it (ADR 0104 E3). No range after the name = any version of that contract.
//!
//! One reader for both: the crate `semver` (cargo's own notation, `Version` = three parts, `VersionReq` = comma separated
//! comparisons). A two part module version (`0.1`) is NOT a version of this notation and is refused with a cause, never guessed. A range
//! of two parts (`0.1`) is read by the crate as a caret requirement (`^0.1` = `>=0.1.0, <0.2.0`).
//!
//! Pattern: Specification (a pure predicate over two small values; unreadable text is an error, never read as "any" and never dropped).

pub use semver::{Error, Version, VersionReq};

/// Read a module version: `Major.Minor.Patch` (pre-release and build parts as the crate knows them).
///
/// # Errors
/// The crate's error: the text is not a version (a two part `0.1` is one of those).
pub fn version(text: &str) -> Result<Version, Error> {
    Version::parse(text.trim())
}

/// The requirement after the name of a contract (`groups >=0.0.1, <0.1` -> `>=0.0.1, <0.1`); no words after the name = any version.
///
/// # Errors
/// The crate's error: the words are not a requirement (a lone `>=` is one of those).
pub fn requirement(range: &str) -> Result<VersionReq, Error> {
    let rest = range
        .trim()
        .split_once(char::is_whitespace)
        .map_or("", |(_, r)| r.trim());
    if rest.is_empty() {
        return Ok(VersionReq::STAR);
    }
    // The description checker of fs-forms (fs-forms-layout v0.0.11) still writes the comparisons separated by blanks only
    // (`>=0.1 <0.2`) and refuses a comma. Until it follows, blank and comma both separate (joined with ", " for the crate).
    let joined = rest
        .split_whitespace()
        .map(|w| w.trim_end_matches(','))
        .collect::<Vec<_>>()
        .join(", ");
    VersionReq::parse(&joined)
}

/// Does `have` fit every comparison of `range` (the words after the name)?
///
/// # Errors
/// The range cannot be read (see [`requirement`]).
pub fn fits(range: &str, have: &Version) -> Result<bool, Error> {
    Ok(requirement(range)?.matches(have))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(t: &str) -> Version {
        version(t).unwrap()
    }

    #[test]
    fn a_version_must_be_inside_every_comparison() {
        let r = "groups >=0.0.1, <0.1";
        assert_eq!(fits(r, &v("0.0.1")).ok(), Some(true));
        assert_eq!(fits(r, &v("0.0.9")).ok(), Some(true));
        assert_eq!(fits(r, &v("0.1.0")).ok(), Some(false));
        assert_eq!(fits(r, &v("0.0.0")).ok(), Some(false));
        assert_eq!(fits("groups >0.1.0", &v("0.1.0")).ok(), Some(false));
        assert_eq!(fits("groups <=0.1.0", &v("0.1.0")).ok(), Some(true));
        assert_eq!(fits("groups =1.0.0", &v("1.0.0")).ok(), Some(true));
    }

    #[test]
    fn a_three_part_version_of_the_catalog_is_read() {
        assert_eq!(v("0.0.1"), Version::new(0, 0, 1));
        assert_eq!(fits("tasks >=0.0.1, <0.1", &v("0.0.1")).ok(), Some(true));
    }

    #[test]
    fn a_two_part_range_is_a_caret_requirement_and_a_two_part_version_is_refused() {
        // belegt: "0.1" = ^0.1 = >=0.1.0, <0.2.0
        assert_eq!(fits("groups 0.1", &v("0.1.5")).ok(), Some(true));
        assert_eq!(fits("groups 0.1", &v("0.2.0")).ok(), Some(false));
        assert!(version("0.1").is_err());
        assert!(version("1").is_err());
    }

    #[test]
    fn no_range_means_any_and_an_unreadable_one_is_an_error() {
        assert_eq!(fits("groups", &v("9.9.9")).ok(), Some(true));
        assert!(fits("groups >=x", &v("0.1.0")).is_err());
        assert!(fits("groups >=", &v("0.1.0")).is_err());
        // the blank separated notation of the description checker (fs-forms-layout) is read too; the comma is optional
        assert_eq!(fits("groups >=0.1 <0.2", &v("0.1.5")).ok(), Some(true));
        assert_eq!(fits("groups >=0.1 <0.2", &v("0.2.0")).ok(), Some(false));
        assert!(fits("groups >=0.1 <", &v("0.1.0")).is_err());
    }
}
