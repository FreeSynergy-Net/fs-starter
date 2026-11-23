//! The range a description writes after the name of a contract (`groups >=0.1 <0.2`): the host compares the version of the module
//! with it (ADR 0104 E3). No range after the name = any version of that contract.
//!
//! Pattern: Specification (a pure predicate over two small values; unreadable text is refused, never read as "any").

/// A version `Major.Minor` (a bare `Major` counts as `Major.0`).
pub type Version = (u32, u32);

/// Read `H.N` or `H`.
#[must_use]
pub fn version(text: &str) -> Option<Version> {
    let (major, minor) = text.split_once('.').unwrap_or((text, "0"));
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    (digits(major) && digits(minor)).then_some((major.parse().ok()?, minor.parse().ok()?))
}

/// Does `have` fit every comparison of `range` (the words after the name)? `None`: the range cannot be read.
///
/// A comparison is `>=`, `>`, `<=`, `<` or `=` (or nothing: equal) and a version.
#[must_use]
pub fn fits(range: &str, have: Version) -> Option<bool> {
    let mut all = true;
    for word in range.split_whitespace().skip(1) {
        let (op, text) = ["<=", ">=", "<", ">", "="]
            .iter()
            .find_map(|op| word.strip_prefix(op).map(|rest| (*op, rest)))
            .unwrap_or(("=", word));
        let want = version(text)?;
        all &= match op {
            ">=" => have >= want,
            ">" => have > want,
            "<=" => have <= want,
            "<" => have < want,
            _ => have == want,
        };
    }
    Some(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_must_be_inside_every_comparison() {
        assert_eq!(fits("groups >=0.1 <0.2", (0, 1)), Some(true));
        assert_eq!(fits("groups >=0.1 <0.2", (0, 2)), Some(false));
        assert_eq!(fits("groups >=0.1 <0.2", (0, 0)), Some(false));
        assert_eq!(fits("groups >0.1", (0, 1)), Some(false));
        assert_eq!(fits("groups <=0.1", (0, 1)), Some(true));
        assert_eq!(fits("groups 0.1", (0, 1)), Some(true));
        assert_eq!(fits("groups =1", (1, 0)), Some(true));
    }

    #[test]
    fn no_range_means_any_and_an_unreadable_one_is_refused() {
        assert_eq!(fits("groups", (9, 9)), Some(true));
        assert_eq!(fits("groups >=x", (0, 1)), None);
        assert_eq!(fits("groups >=0.1.2", (0, 1)), None);
        assert_eq!(fits("groups >=", (0, 1)), None);
    }
}
