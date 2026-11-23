//! The modules the host may bind (plan section 4, border 1): only what the registry took in. Bundled = the folder `modules/` next to
//! the binary; loaded later = only the system folder of the package manager, through the verifier `SystemPackaged` (it refuses all else).
//! `--modules DIR` never gets here: it is a word of `--list-commands` only.
//!
//! Pattern: Registry view (the registry decides; this lists who brought which module and where its folder is).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use fs_starter::{MODULES_DIR, SystemPlaces};

use super::range::{Version, version};
use crate::registry::{Entry, Refusal, Registry, SystemPackaged};

/// A module the registry knows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Known {
    /// The name of the module = its contract = the name of its folder.
    pub name: String,
    /// The version of its catalog (`commands.json`).
    pub version: Version,
    /// The folder of the module.
    pub folder: PathBuf,
    /// Brought by the package manager (checked as system place); else bundled with the binary.
    pub packaged: bool,
}

/// The modules and their commands.
#[derive(Clone, Debug)]
pub struct Modules {
    registry: Registry,
    known: Vec<Known>,
    /// Folders the registry refused, with the reasons (the window names the cause, not "unknown source").
    refused: Vec<(String, Vec<Refusal>)>,
}

impl Modules {
    /// Bundled modules in `bundled` (the host's own folder), loaded ones in `system` (only if `SystemPackaged` accepts them).
    /// A folder that is missing or refused leaves its modules out; the window says "unknown source" for them, never silently binds.
    #[must_use]
    pub fn load(bundled: &Path, system: &Path, verifier: &SystemPackaged) -> Self {
        let mut me = Self {
            registry: Registry::new(),
            known: Vec::new(),
            refused: Vec::new(),
        };
        let before = me.names();
        let got = me.registry.load_bundled(bundled);
        me.report(bundled, got);
        me.take(&before, bundled, false);
        let before = me.names();
        let got = me.registry.load_packages(system, verifier);
        me.report(system, got);
        me.take(&before, system, true);
        me
    }

    /// The real places: `modules/` next to the binary and the system folder of the modules.
    #[must_use]
    pub fn standard() -> Self {
        let bundled = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("modules")))
            .unwrap_or_default();
        Self::load(&bundled, Path::new(MODULES_DIR), &SystemPackaged::new())
    }

    /// What the registry said about a folder: a missing folder is fine (nothing brought), every other error and every refused module
    /// is kept and written to stderr, never thrown away.
    fn report(&mut self, dir: &Path, got: std::io::Result<Vec<(String, Vec<Refusal>)>>) {
        match got {
            Ok(refused) => {
                for (module, why) in &refused {
                    for r in why {
                        eprintln!(
                            "fs-starter: module {module:?} in {:?}: {r}",
                            dir.to_string_lossy()
                        );
                    }
                }
                self.refused.extend(refused);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                eprintln!(
                    "fs-starter: modules folder {:?}: {e}",
                    dir.to_string_lossy()
                );
                self.refused.push((
                    String::new(),
                    vec![Refusal::Unreadable(format!("folder: {e}"))],
                ));
            }
        }
    }

    /// Why the registry refused the module folder `name`, as a cause word and a reason text; `None` when it was not refused.
    #[must_use]
    pub fn refusal(&self, name: &str) -> Option<(&'static str, String)> {
        let (_, why) = self.refused.iter().find(|(m, _)| m == name)?;
        let text = why
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        let cause = if why.contains(&Refusal::Unreadable("version".to_owned())) {
            "version-unreadable"
        } else {
            "refused"
        };
        Some((cause, text))
    }

    fn names(&self) -> BTreeSet<String> {
        self.registry
            .entries()
            .iter()
            .map(|e| e.module.clone())
            .collect()
    }

    fn take(&mut self, before: &BTreeSet<String>, root: &Path, packaged: bool) {
        for name in self.names().difference(before) {
            let have = self.registry.entries().iter().find(|e| &e.module == name);
            if let Some(v) = have.and_then(|e| version(&e.version).ok()) {
                self.known.push(Known {
                    name: name.clone(),
                    version: v,
                    folder: root.join(name),
                    packaged,
                });
            }
        }
    }

    /// The module that serves the contract `name`.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&Known> {
        self.known.iter().find(|k| k.name == name)
    }

    /// The commands of the module `name` (its own name space).
    pub fn commands<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Entry> {
        self.registry
            .entries()
            .iter()
            .filter(move |e| e.module == name)
    }
}

impl Known {
    /// The program file of the service: `<folder>/service`, never found through PATH or a word of a description. A plain file, no
    /// link on the way; a packaged module also has to pass the system check (owner, not writable by others) for the file itself.
    ///
    /// # Errors
    /// A cause word: `no-service`, `symlink`, `not-a-file`, `not-system`.
    pub fn service(&self) -> Result<PathBuf, &'static str> {
        let file = self.folder.join("service");
        for p in [&self.folder, &file] {
            let m = std::fs::symlink_metadata(p).map_err(|_| "no-service")?;
            if m.file_type().is_symlink() {
                return Err("symlink");
            }
        }
        if !std::fs::symlink_metadata(&file).is_ok_and(|m| m.is_file()) {
            return Err("not-a-file");
        }
        if self.packaged && SystemPlaces::standard().check(&file).is_err() {
            return Err("not-system");
        }
        Ok(file)
    }
}
