//! The checks "is this package trusted": the trait [`Verifier`], the default [`Unbound`] (refuses all) and [`SystemPackaged`]
//! (the package manager brought it). Split from `registry.rs` (file size rule); no logic changed.
//!
//! Pattern: Strategy (the check is a trait with replaceable implementations) + Specification (the system check).

use std::fs::File;
use std::path::{Path, PathBuf};

use fs_starter::{SystemPlaces, system_modules_dir};

use super::model::ModuleSet;
use super::names::word;

/// The signature check of a package loaded later (`fs-update`, not bound yet). The default says no: nothing counts as signed until a
/// real check is bound. There is no public flag a caller could set; only an implementation of this trait can accept.
pub trait Verifier {
    /// Is the package of this set signed by a key we trust? The default refuses.
    fn verify(&self, _set: &ModuleSet) -> bool {
        false
    }

    /// Like [`Verifier::verify`], but for the file that was really opened (`file`, found at `path`): a check that looks at files must
    /// look at this one, not at a path made from the name. The default asks [`Verifier::verify`].
    fn verify_file(&self, set: &ModuleSet, _path: &Path, _file: &File) -> bool {
        self.verify(set)
    }
}

/// The check that is there until `fs-update` is bound: refuses every package.
#[derive(Clone, Copy, Debug, Default)]
pub struct Unbound;

impl Verifier for Unbound {}

/// The check "the package manager brought this": the commands file of the module lies in the system folder of the modules
/// (`/usr/libexec/freesynergy/modules/<module>/catalog/commands.json`), and every part of the way is root's, no symlink, not writable
/// by anyone else (see [`SystemPlaces`]). dnf checked the signature of the package; there is no second check. Everything else stays
/// refused: a module on user level, a folder a user can write, a link, a folder outside the system places.
#[derive(Clone, Debug)]
pub struct SystemPackaged {
    places: SystemPlaces,
    modules: PathBuf,
}

impl SystemPackaged {
    /// The real system places (owner root) and the system folder of the modules (none where there is no package manager).
    #[must_use]
    pub fn new() -> Self {
        Self {
            places: SystemPlaces::standard(),
            modules: system_modules_dir().unwrap_or_default(),
        }
    }

    /// Other places and modules folder (tests only; see [`SystemPlaces::for_test`]).
    #[doc(hidden)]
    #[must_use]
    pub fn for_test(places: SystemPlaces, modules: PathBuf) -> Self {
        Self { places, modules }
    }
}

impl Default for SystemPackaged {
    fn default() -> Self {
        Self::new()
    }
}

impl Verifier for SystemPackaged {
    /// The file that was opened and read must be the file that passes [`SystemPlaces::check`]: the way to `path` is below a system
    /// place, root's, no link, nobody else may write; and the open handle is the very file `path` names now (same device and inode),
    /// so a file swapped for another one between the check and the read is not accepted. Everything the handle gave was read from
    /// that file. The folder the caller named is the caller's: only the file counts, never a path made from the module name.
    fn verify_file(&self, set: &ModuleSet, path: &Path, file: &File) -> bool {
        word(&set.module) && self.places.check(path).is_ok() && same_file(path, file)
    }

    fn verify(&self, set: &ModuleSet) -> bool {
        // the name is a folder name of the modules folder: no way out of it
        word(&set.module)
            && self
                .places
                .check(
                    &self
                        .modules
                        .join(&set.module)
                        .join("catalog")
                        .join("commands.json"),
                )
                .is_ok()
    }
}

/// Is the open `file` the one that `path` names at this moment (no link followed)? The crate `same-file` compares device and inode
/// (Unix) or volume and file index (Windows); a link at `path` is refused first (the crate would follow it).
fn same_file(path: &Path, file: &File) -> bool {
    let plain = std::fs::symlink_metadata(path).is_ok_and(|m| !m.file_type().is_symlink());
    plain
        && file.try_clone().is_ok_and(|open| {
            match (
                same_file::Handle::from_file(open),
                same_file::Handle::from_path(path),
            ) {
                (Ok(a), Ok(b)) => a == b,
                _ => false,
            }
        })
}
