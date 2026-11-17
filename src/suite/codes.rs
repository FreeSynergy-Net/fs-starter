//! The codes of the suite's failures. Message key: `starter-suite-` and the name.
//!
//! In the probe these are `fs_conformance::Coded` (its stand-in `fault`); after the fs-types lift both are `fs_types::coded` (today `coded_stub`).

use fs_conformance::ErrorCode;

/// What a rule of the suite found wrong.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SuiteCode {
    /// Exit `got` instead of `want`.
    WrongExit,
    /// stdout is not what it must be (parameter `stream`).
    WrongOutput,
    /// The server gate was asked `calls` times (0 for words that need no server, 1 for a start).
    ServerCalls,
    /// Something ran that must not (`what`: `frontend`, `check`, `mcp`).
    Launched,
    /// The frontend `frontend` was not the one opened (or more than one opened).
    NotLaunched,
    /// No note with `code` at the right level.
    NotTold,
    /// The note `code` has no parameter `key` with the right value.
    ParamWrong,
    /// The opened frontend did not get the word `word`.
    WordMissing,
    /// The resume token showed up where it must not (`place`: `stdout`, `note`).
    Leaked,
    /// The MCP entry was called with another actor than the one given, or not once.
    ActorChanged,
}

impl ErrorCode for SuiteCode {
    const PREFIX: &'static str = "starter-suite-";
    const ALL: &'static [Self] = &[
        Self::WrongExit,
        Self::WrongOutput,
        Self::ServerCalls,
        Self::Launched,
        Self::NotLaunched,
        Self::NotTold,
        Self::ParamWrong,
        Self::WordMissing,
        Self::Leaked,
        Self::ActorChanged,
    ];

    fn as_str(self) -> &'static str {
        match self {
            Self::WrongExit => "wrong-exit",
            Self::WrongOutput => "wrong-output",
            Self::ServerCalls => "server-calls",
            Self::Launched => "launched",
            Self::NotLaunched => "not-launched",
            Self::NotTold => "not-told",
            Self::ParamWrong => "param-wrong",
            Self::WordMissing => "word-missing",
            Self::Leaked => "leaked",
            Self::ActorChanged => "actor-changed",
        }
    }
}

/// The parameter keys of the suite's codes.
pub mod keys {
    /// The expected exit.
    pub const WANT: &str = "want";
    /// The exit that came.
    pub const GOT: &str = "got";
    /// `stdout`.
    pub const STREAM: &str = "stream";
    /// How often.
    pub const CALLS: &str = "calls";
    /// What ran.
    pub const WHAT: &str = "what";
    /// A frontend id.
    pub const FRONTEND: &str = "frontend";
    /// A message key of a starter code.
    pub const CODE: &str = "code";
    /// A parameter key.
    pub const KEY: &str = "key";
    /// A word.
    pub const WORD: &str = "word";
    /// Where something leaked.
    pub const PLACE: &str = "place";
}
