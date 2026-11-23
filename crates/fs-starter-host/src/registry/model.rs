//! The value types of the registry: what a command may do, its help, an item, the set of a module and why a set is refused.
//!
//! Pattern: Value objects (plain data, no behaviour beyond reading a word and showing a refusal as a coded line).

use std::fmt;

use fs_components_commands::{Command, CommandId};

/// What a command may do with the data (the `access` of the exchange file).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// Only reads.
    Read,
    /// Changes data.
    Write,
    /// Deletes data.
    Delete,
}

impl Access {
    pub(super) fn parse(word: &str) -> Option<Self> {
        match word {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            "delete" => Some(Self::Delete),
            _ => None,
        }
    }
}

/// The help of a command: a topic and the text in both languages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Help {
    /// The help topic (also the label of the command).
    pub topic: String,
    /// English text.
    pub en: String,
    /// German text.
    pub de: String,
}

/// One command of a module with what the file says about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// The command (its label is the help topic).
    pub command: Command,
    /// What it may do.
    pub access: Access,
    /// Its help.
    pub help: Help,
    /// The call as written in the file; kept, never run.
    pub call: Option<String>,
}

/// The command set of one module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleSet {
    /// The name of the module; every id starts with `<name>.`.
    pub module: String,
    /// The version of the set (`Major.Minor`).
    pub version: String,
    /// The commands.
    pub items: Vec<Item>,
}

/// Why a set was refused (all reasons are given at once; nothing is added then).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A bot asked.
    Bot,
    /// The set is not from a bundled module and has no good signature.
    NotTrusted,
    /// The module has no name, the name `app` (the name space of the shell), or not the name of its folder.
    ModuleName(String),
    /// A command with the prefix `app.`.
    ReservedPrefix(CommandId),
    /// A command outside the name space of its module.
    OutsideNamespace(CommandId),
    /// The id is there already (the shell's, another module's, or twice in this set).
    Conflict(CommandId),
    /// An id with a character outside `[a-z0-9_-]` (segments split by dots) or not `<module>.<name>`: the whole set is refused.
    BadId(String),
    /// The file cannot be read as a set (the cause is a short word).
    Unreadable(String),
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bot => write!(f, "bot-may-not-extend-commands"),
            Self::NotTrusted => write!(f, "command-set-not-trusted"),
            Self::ModuleName(n) => write!(f, "command-set-module-name: {n:?}"),
            Self::ReservedPrefix(id) => write!(f, "command-reserved-prefix: {:?}", id.as_str()),
            Self::OutsideNamespace(id) => write!(f, "command-outside-namespace: {:?}", id.as_str()),
            Self::Conflict(id) => write!(f, "command-conflict: {:?}", id.as_str()),
            Self::BadId(id) => write!(f, "command-bad-id: {id:?}"),
            Self::Unreadable(c) => write!(f, "command-set-unreadable: {c}"),
        }
    }
}
