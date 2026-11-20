//! The codes of the starter, with their parameter keys. Message key: `starter-error-` and the name.
//!
//! Pattern: none, an enum with total `match`es (as fs-rpc `code/kind.rs`). A code is never renamed or reused once released;
//! a new one is appended to the enum AND to `ALL` (test K16 of the draft: each variant exactly once).

use fs_types::coded::ErrorCode;

/// What the starter found wrong or must say, without a sentence.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StarterCode {
    /// A word is unknown, repeated, misses its value, or does not fit with another word (parameter `word`). Exit 2.
    Usage,
    /// `mcp` without `--as <actor>`. Exit 2. The actor is never guessed.
    ActorMissing,
    /// No socket: no `--socket` and no default (no `XDG_RUNTIME_DIR`). Exit 1.
    NoSocket,
    /// The server could not be made sure (parameter `cause` = the fs-rpc code name, then its parameters). Not fatal: the
    /// frontend shows the same in its window.
    Server,
    /// A hanging server of this program was saved and ended for the new one (parameters `pid`, `saved`, `how`). A hint.
    ServerTakenOver,
    /// No usable manifest or program file for a frontend (parameters `frontend`, and `fallback` when another opens).
    FrontendMissing,
    /// The frontend's check could not run or ended by a signal (parameters `frontend`, `fallback`).
    FrontendUnstartable,
    /// The frontend does not fit (parameters `frontend`, `field` = `extends`, `requires`, `fs:requires` or `check`,
    /// `fallback` when another opens).
    FrontendContract,
    /// A manifest file was refused (parameters `file`, `field`). Listed, never skipped silently.
    ManifestInvalid,
    /// The manifest was put there by hand and is not checked (parameter `frontend`; ADR 0094 item 7). A hint.
    ManifestUnchecked,
    /// The frontend's program file could not be run (parameters `frontend`, `kind` = fs-rpc kind token). Exit 1.
    OpenFailed,
    /// `mcp` was asked, but this build of the program has no MCP entry. Exit 1.
    McpMissing,
    /// `--version`/`--help` could not be written (parameter `kind`: `broken-pipe` or `other`). Exit 1.
    OutputFailed,
    /// `mcp` was asked, but `fs.mcp.enabled` is off (the default). Exit 1.
    McpDisabled,
}

impl ErrorCode for StarterCode {
    const PREFIX: &'static str = "starter-error-";
    const ALL: &'static [Self] = &[
        Self::Usage,
        Self::ActorMissing,
        Self::NoSocket,
        Self::Server,
        Self::ServerTakenOver,
        Self::FrontendMissing,
        Self::FrontendUnstartable,
        Self::FrontendContract,
        Self::ManifestInvalid,
        Self::ManifestUnchecked,
        Self::OpenFailed,
        Self::McpMissing,
        Self::OutputFailed,
        Self::McpDisabled,
    ];

    fn as_str(self) -> &'static str {
        match self {
            Self::Usage => "usage",
            Self::ActorMissing => "actor-missing",
            Self::NoSocket => "no-socket",
            Self::Server => "server",
            Self::ServerTakenOver => "server-taken-over",
            Self::FrontendMissing => "frontend-missing",
            Self::FrontendUnstartable => "frontend-unstartable",
            Self::FrontendContract => "frontend-contract",
            Self::ManifestInvalid => "manifest-invalid",
            Self::ManifestUnchecked => "manifest-unchecked",
            Self::OpenFailed => "open-failed",
            Self::McpMissing => "mcp-missing",
            Self::OutputFailed => "output-failed",
            Self::McpDisabled => "mcp-disabled",
        }
    }
}

/// The parameter keys of the starter's codes. The keys that come from fs-rpc (`pid`, `kind`, ...) keep their fs-rpc names.
pub mod keys {
    /// The word that was wrong (never the value after `--resumed`).
    pub const WORD: &str = "word";
    /// The fs-rpc code name behind `server` (`server-unresponsive`).
    pub const CAUSE: &str = "cause";
    /// A frontend id.
    pub const FRONTEND: &str = "frontend";
    /// The frontend that opens instead.
    pub const FALLBACK: &str = "fallback";
    /// The manifest field that does not fit or could not be read.
    pub const FIELD: &str = "field";
    /// A manifest file (its path).
    pub const FILE: &str = "file";
    /// What a save before the end came to (`saved`, `nothing`, `failed`; fs-rpc `Saved::token`).
    pub const SAVED: &str = "saved";
    /// How a taken-over server ended (`gone`, `sigterm`, `sigkill`).
    pub const HOW: &str = "how";
    /// A process number.
    pub const PID: &str = "pid";
    /// The class of a system failure (fs-rpc kind token).
    pub const KIND: &str = "kind";
}
