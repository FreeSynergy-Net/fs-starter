//! The rules of characters for names, ids and versions of a command set.
//!
//! Pattern: Specification (small pure predicates over text; no state).

/// A name of a module or a segment of an id: `[a-z0-9_-]+` (ADR 0097 rule of names; no dot, no space, no control character).
pub(super) fn word(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

/// An id: segments of [`word`] split by dots, at least two (`<module>.<name>`).
pub(super) fn valid_id(id: &str) -> bool {
    id.split('.').count() >= 2 && id.split('.').all(word)
}

/// A version: `Major.Minor`, digits only.
pub(super) fn valid_version(v: &str) -> bool {
    let mut p = v.split('.');
    let digits =
        |s: Option<&str>| s.is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()));
    digits(p.next()) && digits(p.next()) && p.next().is_none()
}
