//! The command registry of the host (ADR 0097 N9/E7, plan `ENTSCHEID-form` section 3 and 5): the shared commands `app.*` plus the
//! command sets of the bundled modules, read at run time (no fixed list). A module adds its commands under its own name space; it
//! never takes `app.`, never overwrites an id that is there (a conflict is a refusal of the whole set), and a bot never extends the
//! set. Every command a module brought is shown as "new by module X".
//!
//! The set of a module is an exchange file `<modules>/<module>/commands.toml` (the format is a stand-in until the one of
//! `fs-capture` is final): `[module] name, version` and `[[command]] id, label, icon`. The command itself is the one of
//! `fs-components-commands` (one model, no second one).
//!
//! Pattern: Registry (one place that knows every command and who brought it) + Value objects; adding is all or nothing.

use std::fmt;
use std::fs;
use std::path::Path;

use fs_components_commands::standard::PREFIX;
use fs_components_commands::{Command, CommandId, Commands, standard_commands};

/// Who asks to extend the set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Actor {
    /// A person with the right to install.
    Person,
    /// A bot: never extends the command set (rule of the bot rights).
    Bot,
}

/// Where a set comes from. Only these two are accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// A module that comes with the host (the folder of the bundled modules).
    Bundled,
    /// A package loaded later; `signature_ok` is the verdict of the signature check (fs-update). `false` is refused.
    Package {
        /// Did the signature of the package check out?
        signature_ok: bool,
    },
}

/// The command set of one module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleSet {
    /// The name of the module; every id starts with `<name>.`.
    pub module: String,
    /// The version of the set.
    pub version: String,
    /// The commands.
    pub commands: Vec<Command>,
}

/// Why a set was refused (all reasons are given at once; nothing is added then).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A bot asked.
    Bot,
    /// The set is not from a bundled module and has no good signature.
    NotTrusted,
    /// The module has no name, or the name `app` (the name space of the shell).
    ModuleName(String),
    /// A command with the prefix `app.`.
    ReservedPrefix(CommandId),
    /// A command outside the name space of its module.
    OutsideNamespace(CommandId),
    /// The id is there already (the shell's, another module's, or twice in this set).
    Conflict(CommandId),
    /// The file cannot be read as a set (the cause is a short word).
    Unreadable(String),
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bot => write!(f, "bot-may-not-extend-commands"),
            Self::NotTrusted => write!(f, "command-set-not-trusted"),
            Self::ModuleName(n) => write!(f, "command-set-module-name: {n:?}"),
            Self::ReservedPrefix(id) => write!(f, "command-reserved-prefix: {}", id.as_str()),
            Self::OutsideNamespace(id) => write!(f, "command-outside-namespace: {}", id.as_str()),
            Self::Conflict(id) => write!(f, "command-conflict: {}", id.as_str()),
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
            })
            .collect();
        Self { entries }
    }

    /// All entries in order: the shell's first, then the modules' in the order they were added.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Add the set of a module; all or nothing.
    ///
    /// # Errors
    /// Every [`Refusal`] that applies; the registry is unchanged then.
    pub fn add(
        &mut self,
        actor: Actor,
        origin: Origin,
        set: ModuleSet,
    ) -> Result<(), Vec<Refusal>> {
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
        if name.is_empty() || name == PREFIX.trim_end_matches('.') || name.contains('.') {
            bad.push(Refusal::ModuleName(set.module.clone()));
        }
        let mut seen: Commands = self
            .entries
            .iter()
            .fold(Commands::new(), |all, e| all.with(e.command.clone()));
        let space = format!("{name}.");
        for c in &set.commands {
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
        self.entries
            .extend(set.commands.into_iter().map(|command| Entry {
                command,
                module: set.module.clone(),
                version: set.version.clone(),
                new_by: Some(set.module.clone()),
            }));
        Ok(())
    }

    /// Read every `<dir>/<module>/commands.toml` (sorted by module name; a missing folder is no modules) and add the sets as bundled.
    /// A set that is refused is reported and left out; the others are added.
    pub fn load_bundled(&mut self, dir: &Path) -> Vec<(String, Vec<Refusal>)> {
        let mut folders: Vec<_> = fs::read_dir(dir)
            .map(|d| d.filter_map(Result::ok).map(|e| e.path()).collect())
            .unwrap_or_default();
        folders.sort();
        let mut refused = Vec::new();
        for folder in folders {
            let file = folder.join("commands.toml");
            if !file.is_file() {
                continue;
            }
            let label = folder
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            let result = fs::read_to_string(&file)
                .map_err(|_| vec![Refusal::Unreadable("read-failed".to_owned())])
                .and_then(|text| parse(&text).map_err(|r| vec![r]))
                .and_then(|set| self.add(Actor::Person, Origin::Bundled, set));
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

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

/// Read the exchange file of a set.
///
/// # Errors
/// [`Refusal::Unreadable`] with the cause (`toml`, `module-missing`, `command-id-missing`, `command-label-missing`).
pub fn parse(text: &str) -> Result<ModuleSet, Refusal> {
    let bad = |c: &str| Refusal::Unreadable(c.to_owned());
    let doc: toml::Table = text.parse().map_err(|_| bad("toml"))?;
    let module = doc
        .get("module")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| bad("module-missing"))?;
    let word = |t: &toml::Table, k: &str| t.get(k).and_then(toml::Value::as_str).map(str::to_owned);
    let mut commands = Vec::new();
    for item in doc
        .get("command")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
    {
        let t = item.as_table().ok_or_else(|| bad("command-id-missing"))?;
        let id = word(t, "id").ok_or_else(|| bad("command-id-missing"))?;
        let label = word(t, "label").ok_or_else(|| bad("command-label-missing"))?;
        let mut c = Command::new(id.as_str(), label.as_str());
        if let Some(icon) = word(t, "icon") {
            c = c.icon(icon);
        }
        commands.push(c);
    }
    Ok(ModuleSet {
        module: word(module, "name").unwrap_or_default(),
        version: word(module, "version").unwrap_or_default(),
        commands,
    })
}
