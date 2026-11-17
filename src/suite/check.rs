//! Helpers of the rules: each check is a `Result`, the first failure is the verdict.
//!
//! Pattern: Specification (small predicates over an [`Outcome`]) + a macro that makes a rule of a body.

use std::ffi::OsString;
use std::path::Path;

use fs_conformance::{Coded, Cx, Need, Verdict};
use fs_types::coded::message_key;

use super::codes::{SuiteCode, keys};
use super::fixture::{Arrange, Outcome, StarterFixture};
use crate::codes::StarterCode;
use crate::program::Exit;
use crate::tell::{Level, Note};

/// The result of one check.
pub(crate) type Checked = Result<(), Coded<SuiteCode>>;

/// Why a rule body stopped early: a failure of the suite, or a verdict of the frame (a missing need).
pub(crate) enum Stop {
    /// A failure with the suite's code.
    Fail(Coded<SuiteCode>),
    /// A verdict as it is (`Cx::missing`).
    Verdict(Verdict),
}

impl From<Coded<SuiteCode>> for Stop {
    fn from(c: Coded<SuiteCode>) -> Self {
        Self::Fail(c)
    }
}

/// The body of a rule: `?` works on every [`Checked`].
pub(crate) type Body = Result<(), Stop>;

/// Arranges, or stops with the need `fixture.arrange` (skipped under `partial`, else `capability-missing`).
pub(crate) fn arrange<F: StarterFixture + ?Sized>(f: &mut F, cx: &Cx, setup: &Arrange) -> Body {
    if f.arrange(setup) {
        Ok(())
    } else {
        Err(Stop::Verdict(cx.missing(Need::new("fixture.arrange"))))
    }
}

/// A rule: `rule!(Type, "id", |fixture, cx| { ...; Ok(()) })`. The body is a [`Body`]; the fixture is reset first.
macro_rules! rule {
    ($(#[$doc:meta])* $t:ident, $id:literal, |$f:ident, $cx:ident| $body:block) => {
        $(#[$doc])*
        pub(crate) struct $t;

        #[async_trait::async_trait(?Send)]
        impl<F: $crate::suite::StarterFixture + ?Sized> fs_conformance::Rule<F> for $t {
            fn id(&self) -> fs_conformance::RuleId {
                fs_conformance::RuleId::new($id)
            }

            async fn check(&self, $f: &mut F, $cx: &fs_conformance::Cx) -> fs_conformance::Verdict {
                // A nested function, not a closure: `?` works, and nothing is captured.
                fn body<F: $crate::suite::StarterFixture + ?Sized>($f: &mut F, $cx: &fs_conformance::Cx) -> $crate::suite::check::Body $body
                $f.reset();
                match body($f, $cx) {
                    Ok(()) => fs_conformance::Verdict::Pass,
                    Err($crate::suite::check::Stop::Fail(code)) => fs_conformance::Verdict::Fail(code.erase()),
                    Err($crate::suite::check::Stop::Verdict(verdict)) => verdict,
                }
            }
        }
    };
}
pub(crate) use rule;

fn code(c: SuiteCode) -> Coded<SuiteCode> {
    Coded::new(c)
}

/// The exit must be `want`.
pub(crate) fn exit(o: &Outcome, want: Exit) -> Checked {
    if o.exit == want {
        return Ok(());
    }
    Err(code(SuiteCode::WrongExit)
        .with_number(keys::WANT, want.code().into())
        .with_number(keys::GOT, o.exit.code().into()))
}

/// stdout must be exactly `want`.
pub(crate) fn stdout(o: &Outcome, want: &str) -> Checked {
    if o.stdout == want {
        Ok(())
    } else {
        Err(code(SuiteCode::WrongOutput).with_text(keys::STREAM, "stdout"))
    }
}

/// The server gate must have been asked exactly `calls` times.
pub(crate) fn gate(o: &Outcome, calls: u32) -> Checked {
    if o.gate_calls == calls {
        Ok(())
    } else {
        Err(code(SuiteCode::ServerCalls).with_number(keys::CALLS, o.gate_calls.into()))
    }
}

/// No frontend was asked or opened.
pub(crate) fn nothing_opened(o: &Outcome) -> Checked {
    match (o.checked.is_empty(), o.opened.is_empty()) {
        (true, true) => Ok(()),
        (false, _) => Err(code(SuiteCode::Launched).with_text(keys::WHAT, "check")),
        (true, false) => Err(code(SuiteCode::Launched).with_text(keys::WHAT, "frontend")),
    }
}

/// Exactly one frontend was opened, at `exe`; its words.
pub(crate) fn opened<'o>(
    o: &'o Outcome,
    exe: &Path,
    id: &str,
) -> Result<&'o [OsString], Coded<SuiteCode>> {
    match o.opened.as_slice() {
        [(path, words)] if path == exe => Ok(words),
        _ => Err(code(SuiteCode::NotLaunched).with_text(keys::FRONTEND, id)),
    }
}

/// `want` stands in `words` in this order, side by side.
pub(crate) fn words_have(words: &[OsString], want: &[&str]) -> Checked {
    let found = words
        .windows(want.len())
        .any(|w| w.iter().zip(want).all(|(a, b)| a == b));
    if found {
        Ok(())
    } else {
        Err(code(SuiteCode::WordMissing).with_text(keys::WORD, want[0]))
    }
}

/// The first note of `c` at `level`.
pub(crate) fn told(o: &Outcome, level: Level, c: StarterCode) -> Result<&Note, Coded<SuiteCode>> {
    let key = message_key(c);
    o.told
        .iter()
        .find(|n| n.level == level && n.code.message_key() == key)
        .ok_or_else(|| code(SuiteCode::NotTold).with_text(keys::CODE, key))
}

/// The note has the text parameter `key` = `value`.
pub(crate) fn param(note: &Note, key: &str, value: &str) -> Checked {
    let ok = matches!(note.code.param(key), Some(fs_types::coded::Param::Text(t)) if t == value);
    if ok {
        Ok(())
    } else {
        Err(code(SuiteCode::ParamWrong)
            .with_text(keys::CODE, note.code.message_key())
            .with_text(keys::KEY, key))
    }
}
