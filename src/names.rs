//! Checked names: of a program, a frontend, an actor; the resume token; a contract requirement; a version.
//!
//! Pattern: Value objects (a value that exists is well formed; `parse` is the only way in from outside).

use std::ffi::{OsStr, OsString};
use std::fmt;

/// 1 to 64 bytes of `a-z`, `0-9`, `-`; starts with a letter, does not end with `-`, no `--`.
fn is_name(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b.len() <= 64
        && b[0].is_ascii_lowercase()
        && b[b.len() - 1] != b'-'
        && !s.contains("--")
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

macro_rules! name_type {
    ($(#[$doc:meta])* $t:ident) => {
        $(#[$doc])*
        #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $t(String);

        impl $t {
            /// The name as text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

name_type!(
    /// A program's name without `fs-` (`activities`); the binary is `fs-<name>`.
    ProgramName
);
name_type!(
    /// A frontend id (ADR 0094 O2: English, ASCII lower case; `simple`). Never a crate role name.
    FrontendId
);
name_type!(
    /// The name of an acting AI (`claude-strategie`), given with `mcp --as`. Never derived from the person.
    ActorName
);

/// Crate roles a frontend must not be called (ADR 0094 item 6, naming rule).
pub const RESERVED_FRONTENDS: [&str; 8] = [
    "core",
    "server",
    "client",
    "app",
    "bin",
    "cli",
    "engine",
    "testserver",
];

impl ProgramName {
    /// `None` unless [`is_name`] holds and the name does not start with `fs-` (that is the binary's prefix).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        (is_name(s) && !s.starts_with("fs-")).then(|| Self(s.to_owned()))
    }
}

impl FrontendId {
    /// `None` unless the form holds and the id is not reserved.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        (is_name(s) && !RESERVED_FRONTENDS.contains(&s)).then(|| Self(s.to_owned()))
    }
}

impl ActorName {
    /// `None` unless the form holds.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        is_name(s).then(|| Self(s.to_owned()))
    }
}

/// The token of `--resumed` (ADR 0069 addendum, item 5). It has no `Display`, and `Debug` hides it: it reaches the frontend's
/// words and nothing else.
#[derive(Clone, PartialEq, Eq)]
pub struct ResumeToken(OsString);

impl ResumeToken {
    /// Any non-empty word.
    #[must_use]
    pub fn parse(word: &OsStr) -> Option<Self> {
        (!word.is_empty()).then(|| Self(word.to_owned()))
    }

    /// The token as a word for the frontend's command line.
    #[must_use]
    pub fn as_os_str(&self) -> &OsStr {
        &self.0
    }
}

impl fmt::Debug for ResumeToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ResumeToken(..)")
    }
}

/// `major.minor.patch`, numbers only (no `v`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version3(pub u32, pub u32, pub u32);

impl Version3 {
    /// `"0.0.2"` → `Version3(0, 0, 2)`; anything else `None`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let mut it = s.split('.').map(|p| {
            (!p.is_empty() && p.bytes().all(|c| c.is_ascii_digit()))
                .then(|| p.parse::<u32>().ok())
                .flatten()
        });
        let v = Self(it.next()??, it.next()??, it.next()??);
        it.next().is_none().then_some(v)
    }
}

/// A contract and its version as a manifest writes it (`fs:requires`, `fs:contract`: `history 0.1`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractReq {
    /// The capability name (`history`), ADR 0068.
    pub name: String,
    /// The main version: must be equal.
    pub major: u16,
    /// The minor version: the server's must be at least this.
    pub minor: u16,
}

impl ContractReq {
    /// `"history 0.1"`; `None` if the name is not a name or the version is not `major.minor`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let (name, version) = s.trim().split_once(' ')?;
        let (major, minor) = version.trim().split_once('.')?;
        if !is_name(name) {
            return None;
        }
        Some(Self {
            name: name.to_owned(),
            major: major.parse().ok()?,
            minor: minor.parse().ok()?,
        })
    }

    /// Whether a server that offers `self` serves a frontend that requires `required`: same name, same major, minor at least.
    #[must_use]
    pub fn serves(&self, required: &ContractReq) -> bool {
        self.name == required.name && self.major == required.major && self.minor >= required.minor
    }
}

impl fmt::Display for ContractReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}.{}", self.name, self.major, self.minor)
    }
}
