//! What the host does with an answer of a module service: it is FOREIGN data (plan section 4, border 5). Its size is limited, control
//! characters are masked before anything is drawn, and the time for the whole call is limited.
//!
//! Pattern: Value object (`Limits`) + pure functions (no I/O here, so every border has a test).

use std::time::Duration;

/// The borders for one connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// The whole call (start, handshake, all pages): 10 s.
    pub time: Duration,
    /// Records per form: asked for as `limit`; more records than that in the answer refuse the answer.
    pub rows: u32,
    /// Bytes of one text value; a longer text is cut at a character border.
    pub text: usize,
    /// Values per channel of one record (the rest is dropped).
    pub values: usize,
    /// Bytes of all text of the answers together; more refuses the answer.
    pub bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            time: Duration::from_secs(10),
            rows: 500,
            text: 1024,
            values: 16,
            bytes: 2 * 1024 * 1024,
        }
    }
}

/// A character that is shown as `U+FFFD`: every control character (also newline and tab: a label is one line) and the characters
/// that turn the direction of the text around (a name must not read as another one).
fn masked(c: char) -> bool {
    c.is_control() || matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

/// The text as it may be drawn: masked, then cut to `max` bytes at a character border.
#[must_use]
pub fn clean(text: &str, max: usize) -> String {
    let mut out = String::new();
    for c in text.chars() {
        let c = if masked(c) { '\u{FFFD}' } else { c };
        if out.len() + c.len_utf8() > max {
            break;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_and_direction_characters_are_masked() {
        assert_eq!(
            clean("a\u{1b}[31mb\nc\td", 100),
            "a\u{FFFD}[31mb\u{FFFD}c\u{FFFD}d"
        );
        assert_eq!(clean("x\u{202E}y", 100), "x\u{FFFD}y");
        assert_eq!(clean("plain Ärger", 100), "plain Ärger");
    }

    #[test]
    fn a_long_text_is_cut_at_a_character_border() {
        assert_eq!(clean("aaaa", 3), "aaa");
        assert_eq!(clean("aÄb", 2), "a");
        assert_eq!(clean("", 0), "");
    }
}
