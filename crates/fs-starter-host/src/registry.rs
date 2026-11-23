//! The command registry of the host (ADR 0097 N9/E7, plan `ENTSCHEID-form` section 3 and 5): the shared commands `app.*` plus the
//! command sets of the bundled modules, read at run time (no fixed list). A module adds its commands under its own name space; it
//! never takes `app.`, never overwrites an id that is there (a conflict is a refusal of the whole set), and a bot never extends the
//! set. Every command a module brought is shown as "new by module X".
//!
//! The set of a module is the file `<modules>/<module>/catalog/commands.json` of the module itself (the format of `fs-capture`, see
//! `read.rs`). The command is the one of `fs-components-commands` (one model, no second one).
//!
//! Pattern: Registry (one place that knows every command and who brought it) + Value objects; adding is all or nothing.

mod read;

use std::fmt;
use std::fs;
use std::path::Path;

use fs_components_commands::standard::PREFIX;
use fs_components_commands::{Command, CommandId, Commands, standard_commands};

pub use read::parse;

/// Who asks to extend the set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Actor {
    /// A person with the right to install.
    Person,
    /// A bot: never extends the command set (rule of the bot rights).
    Bot,
}

/// The signature check of a package loaded later (`fs-update`, not bound yet). The default says no: nothing counts as signed until a
/// real check is bound. There is no public flag a caller could set; only an implementation of this trait can accept.
pub trait Verifier {
    /// Is the package of this set signed by a key we trust? The default refuses.
    fn verify(&self, _set: &ModuleSet) -> bool {
        false
    }
}

/// The check that is there until `fs-update` is bound: refuses every package.
#[derive(Clone, Copy, Debug, Default)]
pub struct Unbound;

impl Verifier for Unbound {}

/// Where a set comes from (private: the public ways in are [`Registry::add_bundled`] and [`Registry::add_package`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Origin {
    Bundled,
    Package { signature_ok: bool },
}

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
    fn parse(word: &str) -> Option<Self> {
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

/// One command and who brought it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The command.
    pub command: Command,
    /// `app` for the shell, else the module.
    pub module: String,
    /// The version of the shell (host) or of the module's set.
    pub version: String,
    /// `Some(module)` for a command a module added: shown as "new by module X".
    pub new_by: Option<String>,
    /// What the command may do (`None` for the shell's own).
    pub access: Option<Access>,
    /// The help of the module's command (`None` for the shell's own).
    pub help: Option<Help>,
}

/// All commands the host knows.
#[derive(Clone, Debug)]
pub struct Registry {
    entries: Vec<Entry>,
}

impl Registry {
    /// The shared commands `app.*` of the shell, read from the command model (not a list of the host).
    #[must_use]
    pub fn new() -> Self {
        let entries = standard_commands()
            .iter()
            .map(|c| Entry {
                command: c.clone(),
                module: "app".to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                new_by: None,
                access: None,
                help: None,
            })
            .collect();
        Self { entries }
    }

    /// All entries in order: the shell's first, then the modules' in the order they were added.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Add the set of a module that comes with the host (the host's own folder of modules); all or nothing.
    ///
    /// # Errors
    /// Every [`Refusal`] that applies; the registry is unchanged then.
    pub fn add_bundled(&mut self, actor: Actor, set: ModuleSet) -> Result<(), Vec<Refusal>> {
        self.add(actor, Origin::Bundled, set)
    }

    /// Add the set of a package loaded later: only if `verifier` accepts it (use [`Unbound`] until `fs-update` is bound: it refuses).
    ///
    /// # Errors
    /// Every [`Refusal`] that applies, [`Refusal::NotTrusted`] among them when the check says no; the registry is unchanged then.
    pub fn add_package(
        &mut self,
        actor: Actor,
        set: ModuleSet,
        verifier: &dyn Verifier,
    ) -> Result<(), Vec<Refusal>> {
        let signature_ok = verifier.verify(&set);
        self.add(actor, Origin::Package { signature_ok }, set)
    }

    fn add(&mut self, actor: Actor, origin: Origin, set: ModuleSet) -> Result<(), Vec<Refusal>> {
        let mut bad = Vec::new();
        if actor == Actor::Bot {
            bad.push(Refusal::Bot);
        }
        if matches!(
            origin,
            Origin::Package {
                signature_ok: false
            }
        ) {
            bad.push(Refusal::NotTrusted);
        }
        let name = set.module.as_str();
        if !word(name) || name == PREFIX.trim_end_matches('.') {
            bad.push(Refusal::ModuleName(set.module.clone()));
        }
        if !valid_version(&set.version) {
            bad.push(Refusal::Unreadable("version".to_owned()));
        }
        let mut seen: Commands = self
            .entries
            .iter()
            .fold(Commands::new(), |all, e| all.with(e.command.clone()));
        let space = format!("{name}.");
        for i in &set.items {
            let c = &i.command;
            if !valid_id(c.id.as_str()) {
                bad.push(Refusal::BadId(c.id.as_str().to_owned()));
                continue;
            }
            if c.id.as_str().starts_with(PREFIX) {
                bad.push(Refusal::ReservedPrefix(c.id.clone()));
            } else if !c.id.as_str().starts_with(&space) {
                bad.push(Refusal::OutsideNamespace(c.id.clone()));
            } else if seen.get(&c.id).is_some() {
                bad.push(Refusal::Conflict(c.id.clone()));
            }
            seen = seen.with(c.clone());
        }
        if !bad.is_empty() {
            return Err(bad);
        }
        self.entries.extend(set.items.into_iter().map(|i| Entry {
            command: i.command,
            module: set.module.clone(),
            version: set.version.clone(),
            new_by: Some(set.module.clone()),
            access: Some(i.access),
            help: Some(i.help),
        }));
        Ok(())
    }

    /// Read every `<dir>/<module>/catalog/commands.json` (sorted by module name; a missing folder is no modules) and add the sets as
    /// bundled. A set that is refused is reported and left out; the others are added.
    pub fn load_bundled(&mut self, dir: &Path) -> Vec<(String, Vec<Refusal>)> {
        let mut folders: Vec<_> = fs::read_dir(dir)
            .map(|d| d.filter_map(Result::ok).map(|e| e.path()).collect())
            .unwrap_or_default();
        folders.sort();
        let mut refused = Vec::new();
        for folder in folders {
            if !folder.join("catalog").join("commands.json").is_file() {
                continue;
            }
            let label = folder
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            let result = read::read_folder(&folder, &label)
                .map_err(|r| vec![r])
                .and_then(|set| self.add_bundled(Actor::Person, set));
            if let Err(why) = result {
                refused.push((label, why));
            }
        }
        refused
    }

    /// The lines of `--list-commands`: `id`, module, version and, for a command a module added, `new by module X`; separated by tabs.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|e| {
                let note = e
                    .new_by
                    .as_ref()
                    .map_or(String::new(), |m| format!("\tnew by module {m}"));
                format!(
                    "{}\t{}\t{}{note}",
                    e.command.id.as_str(),
                    e.module,
                    e.version
                )
            })
            .collect()
    }
}

/// A name of a module or a segment of an id: `[a-z0-9_-]+` (ADR 0097 rule of names; no dot, no space, no control character).
fn word(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

/// An id: segments of [`word`] split by dots, at least two (`<module>.<name>`).
fn valid_id(id: &str) -> bool {
    id.split('.').count() >= 2 && id.split('.').all(word)
}

/// A version: `Major.Minor`, digits only.
fn valid_version(v: &str) -> bool {
    let mut p = v.split('.');
    let digits =
        |s: Option<&str>| s.is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()));
    digits(p.next()) && digits(p.next()) && p.next().is_none()
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}
