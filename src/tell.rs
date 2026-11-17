//! Telling the person what happened: every failure and every fallback, as a code (ADR 0065 item 5: errors are always visible).
//!
//! Pattern: Observer/Port (`Tell`; the standard writes to stderr, the program can put its MessageFormat 2 words in front,
//! the suite records). Replaces the `eprintln!` sentences of activities `starter.rs` (:99, :104, :119, :134, :145).

use fs_types::coded::Code;

use crate::names::ProgramName;

/// How heavy a note is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// The start did not do what was asked (a fallback counts as a hint, see the code).
    Error,
    /// Something the person should know, the start goes on (fallback, taken-over server, unchecked manifest).
    Hint,
}

/// One note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    /// How heavy.
    pub level: Level,
    /// What, with parameters.
    pub code: Code,
}

/// Where notes go.
pub trait Tell {
    /// Tells one note of `program`.
    fn tell(&self, program: &ProgramName, note: &Note);
}

/// The standard: one line on stderr, `fs-<program>: <code> (<params>)`, the technical form of `Code`'s `Display`. A program
/// with MessageFormat 2 words (fs-components) gives its own `Tell` in its `-bin`; the window shows the same codes in words.
#[derive(Clone, Copy, Debug, Default)]
pub struct StderrTell;

impl Tell for StderrTell {
    fn tell(&self, program: &ProgramName, note: &Note) {
        eprintln!("fs-{program}: {}", note.code);
    }
}
