//! The modules the host may bind (plan section 4, border 1): only what the registry took in. Bundled = the folder `modules/` next to
//! the binary; loaded later = only the system folder of the package manager, through the verifier `SystemPackaged` (it refuses all else).
//! `--modules DIR` never gets here: it is a word of `--list-commands` only.
//!
//! Pattern: Registry view (the registry decides; this lists who brought which module and where its folder is).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use fs_forms_run::Coded;
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
            if let Some(e) = have {
                let text = e.version.clone();
                if let Err(c) = self.admit(name, &text, root, packaged) {
                    eprintln!("fs-starter: red {}", super::line(&c));
                }
            }
        }
    }

    /// Take one module of the registry in with its version text.
    ///
    /// # Errors
    /// `host-notice-module-version-unreadable`: the version text is not one (the module is not bound, the refusal is kept).
    fn admit(&mut self, name: &str, text: &str, root: &Path, packaged: bool) -> Result<(), Coded> {
        match version(text) {
            Ok(v) => {
                self.known.push(Known {
                    name: name.to_owned(),
                    version: v,
                    folder: root.join(name),
                    packaged,
                });
                Ok(())
            }
            Err(e) => {
                let reason = e.to_string();
                self.refused.push((
                    name.to_owned(),
                    vec![Refusal::Unreadable("version".to_owned())],
                ));
                Err(Coded::new("host-notice-module-version-unreadable")
                    .with("contract", name)
                    .with("version", text)
                    .with("reason", reason))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> Modules {
        Modules {
            registry: Registry::new(),
            known: Vec::new(),
            refused: Vec::new(),
        }
    }

    /// A version the host cannot read is never dropped without a word: the module is not bound, the refusal is kept and the caller
    /// gets its own code (mutant: the message left out, the refusal not kept).
    #[test]
    fn a_dropped_version_is_said_with_its_own_code() {
        let mut m = empty();
        let c = m
            .admit("groups", "0.1", Path::new("/x"), false)
            .expect_err("a two part version is not one");
        assert_eq!(c.code, "host-notice-module-version-unreadable");
        assert!(m.find("groups").is_none());
        assert_eq!(m.refusal("groups").map(|r| r.0), Some("version-unreadable"));
    }

    #[test]
    fn a_three_part_version_of_the_catalog_is_taken_in() {
        let mut m = empty();
        m.admit("tasks", "0.0.1", Path::new("/x"), false).unwrap();
        assert_eq!(m.find("tasks").unwrap().version, Version::new(0, 0, 1));
    }
}
