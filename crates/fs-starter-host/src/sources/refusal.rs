//! What a refusal of a module service becomes in the host: a CODE the person can be told, never the foreign text. The service names the
//! kind of its refusal as a `Common.Problem` type `form-source-<kind>` (`formsource.capnp`, Level 1); only that fixed shape is taken over
//! as the code, anything else is the fallback of the caller (the line or the service is broken, or it speaks something unknown).
//!
//! Pattern: Adapter (`Common.Problem` of the service -> `Coded` of the host) + pure function (the shape check has a test).

use fs_forms_run::Coded;
use fs_rpc::Problem;

use super::answer::clean;
use super::service::cause;

const PREFIX: &str = "form-source-";
/// The longest reason taken over from a problem (bytes); the rest is cut.
const REASON_MAX: usize = 120;

/// The problem type as a code, when it has exactly the shape `form-source-<kind>` (kind: lower case letters and `-`, at most 32 bytes).
#[must_use]
pub(super) fn kind_code(problem_type: &str) -> Option<String> {
    let kind = problem_type.strip_prefix(PREFIX)?;
    let fits = !kind.is_empty()
        && kind.len() <= 32
        && kind.bytes().all(|b| b.is_ascii_lowercase() || b == b'-');
    fits.then(|| problem_type.to_owned())
}

/// The reason a problem names, as clean short text for the data of the line: the detail (what happened this time), else the title;
/// control characters are masked and the text is cut. `None` when the problem says nothing. Never a code, never trusted as one.
#[must_use]
pub(super) fn reason_of(title: &str, detail: &str) -> Option<String> {
    let said = if detail.trim().is_empty() {
        title
    } else {
        detail
    };
    let said = clean(said.trim(), REASON_MAX);
    (!said.is_empty()).then_some(said)
}

/// `coded` with the reason and the status of a problem (`reason=` as text, `status=` as a number).
#[must_use]
pub(super) fn with_reason(coded: Coded, title: &str, detail: &str, status: u16) -> Coded {
    let coded = match reason_of(title, detail) {
        Some(r) => coded.with("reason", r),
        None => coded,
    };
    if status == 0 {
        coded
    } else {
        coded.with("status", status.to_string())
    }
}

/// The refusal of a failed call: the code of the problem the error carries (`form-source-unknown-form`), else `fallback`.
pub(super) fn refusal_or(error: &capnp::Error, module: &str, why: &str, fallback: &str) -> Coded {
    let problem = Problem::from_error(error);
    let code = problem
        .as_ref()
        .and_then(|p| kind_code(&p.problem_type))
        .unwrap_or_else(|| fallback.to_owned());
    let said = cause(&code, module, why);
    match problem {
        Some(p) => with_reason(said, &p.title, &p.detail, p.status),
        None => said,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_fixed_shape_of_a_kind_becomes_a_code() {
        assert_eq!(
            kind_code("form-source-unknown-form").as_deref(),
            Some("form-source-unknown-form")
        );
        for bad in [
            "form-source-",
            "form-source-Evil",
            "form-source-a b",
            "https://freesynergy.net/problems/unsupported",
            "host-notice-data-unreachable",
        ] {
            assert_eq!(kind_code(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_reason_is_the_detail_else_the_title_clean_and_cut() {
        assert_eq!(reason_of("t", "d").as_deref(), Some("d"));
        assert_eq!(reason_of("title", "  ").as_deref(), Some("title"));
        assert_eq!(reason_of("", ""), None);
        let long = format!("a\u{7}b\n{}", "y".repeat(400));
        let got = reason_of("t", &long).unwrap();
        assert_eq!(got.len(), REASON_MAX);
        assert!(got.starts_with("a\u{FFFD}b\u{FFFD}yy"));
    }

    #[test]
    fn a_problem_in_the_error_text_gives_its_kind_and_a_plain_error_the_fallback() {
        let problem = Problem::new("form-source-unknown-form", "t", "d", 404);
        let said = refusal_or(&problem.into_error(), "groups", "page", "fallback");
        assert_eq!(said.code, "form-source-unknown-form");
        let plain = refusal_or(
            &capnp::Error::failed("boom".into()),
            "groups",
            "page",
            "fallback",
        );
        assert_eq!(plain.code, "fallback");
        assert!(plain.params.iter().all(|(k, _)| k != "reason"));
    }
}
