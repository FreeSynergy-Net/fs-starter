//! The command registry of the host (ADR 0097 N9/E7, plan `ENTSCHEID-form` section 3 and 5): the shared commands `app.*` plus the
//! command sets of the bundled modules, read at run time (no fixed list). A module adds its commands under its own name space; it
//! never takes `app.`, never overwrites an id that is there (a conflict is a refusal of the whole set), and a bot never extends the
//! set. Every command a module brought is shown as "new by module X".
//!
//! The set of a module is the file `<modules>/<module>/catalog/commands.json` of the module itself (the format of `fs-capture`, see
//! `read.rs`). The command is the one of `fs-components-commands` (one model, no second one).
//!
//! Pattern: Registry (one place that knows every command and who brought it) + Value objects; adding is all or nothing.

mod model;
mod names;
pub(crate) mod read;
mod read_ids;
mod verify;

use std::fs;
use std::path::Path;

use fs_components_commands::standard::PREFIX;
use fs_components_commands::{Command, Commands, standard_commands};

pub use model::{Access, Help, Item, ModuleSet, Refusal};
use names::{valid_id, valid_version, word};
pub use read::parse;
pub(crate) use verify::is_same_folder;
pub use verify::{SystemPackaged, Unbound, Verifier};

/// Who asks to extend the set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Actor {
    /// A person with the right to install.
    Person,
    /// A bot: never extends the command set (rule of the bot rights).
    Bot,
}

/// Where a set comes from (private: the public ways in are [`Registry::add_bundled`] and [`Registry::add_package`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Origin {
    Bundled,
    Package { signature_ok: bool },
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
    /// Only for the folders of the host (via [`Registry::load_bundled`]); never call it with data that came from outside.
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

    /// Read every `<dir>/<module>/catalog/commands.json` (sorted by module name) and add the sets as bundled. A set that is refused is
    /// reported and left out; the others are added. Hidden folders are skipped; a symlink (folder or file) and a file over 256 KiB are
    /// refused. This is for `--list-commands` and the folder next to the binary; a folder named by `--modules` never feeds the window.
    ///
    /// # Errors
    /// The folder cannot be read (missing, no right): never silent; the caller decides if a missing default folder is fine.
    pub fn load_bundled(&mut self, dir: &Path) -> std::io::Result<Vec<(String, Vec<Refusal>)>> {
        self.load_folders(dir, |r, file, label| {
            read::read_folder(file, label)
                .map_err(|e| vec![e])
                .and_then(|set| r.add_bundled(Actor::Person, set))
        })
    }

    /// Like [`Registry::load_bundled`], but every set comes in as a package: only if `verifier` accepts it (the caller for
    /// [`Registry::add_package`]; the host uses [`SystemPackaged`] on the system folder of the modules).
    ///
    /// # Errors
    /// The folder cannot be read (missing, no right).
    pub fn load_packages(
        &mut self,
        dir: &Path,
        verifier: &dyn Verifier,
    ) -> std::io::Result<Vec<(String, Vec<Refusal>)>> {
        // The file is opened once; the check looks at THAT file (path and handle), and the set is read from the same handle.
        self.load_folders(dir, |r, file, label| {
            let handle = read::open_plain(file)
                .map_err(|_| vec![Refusal::Unreadable("read-failed".to_owned())])?;
            let copy = handle
                .try_clone()
                .map_err(|_| vec![Refusal::Unreadable("read-failed".to_owned())])?;
            let set = read::read_handle(handle, label).map_err(|e| vec![e])?;
            let signature_ok = verifier.verify_file(&set, file, &copy);
            r.add(Actor::Person, Origin::Package { signature_ok }, set)
        })
    }

    fn load_folders(
        &mut self,
        dir: &Path,
        add: impl Fn(&mut Self, &Path, &str) -> Result<(), Vec<Refusal>>,
    ) -> std::io::Result<Vec<(String, Vec<Refusal>)>> {
        let mut folders = Vec::new();
        let mut refused = Vec::new();
        for entry in fs::read_dir(dir)? {
            match entry {
                Ok(e) => folders.push(e.path()),
                Err(_) => refused.push((
                    String::new(),
                    vec![Refusal::Unreadable("entry-failed".to_owned())],
                )),
            }
        }
        folders.sort();
        for folder in folders {
            let label = folder
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            if label.starts_with('.') {
                continue;
            }
            let file = folder.join("catalog").join("commands.json");
            let result = match read::check(&folder, &file) {
                Ok(false) => continue,
                Ok(true) => add(self, &file, &label),
                Err(r) => Err(vec![r]),
            };
            if let Err(why) = result {
                refused.push((label, why));
            }
        }
        Ok(refused)
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
