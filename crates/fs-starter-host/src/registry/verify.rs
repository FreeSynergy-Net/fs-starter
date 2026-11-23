//! The checks "is this package trusted": the trait [`Verifier`], the default [`Unbound`] (refuses all) and [`SystemPackaged`]
//! (the package manager brought it). Split from `registry.rs` (file size rule); no logic changed.

use std::path::PathBuf;

use fs_starter::{MODULES_DIR, SystemPlaces};

use super::model::ModuleSet;
use super::names::word;

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
    /// The real system places (owner root) and the folder `/usr/libexec/freesynergy/modules`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            places: SystemPlaces::standard(),
            modules: PathBuf::from(MODULES_DIR),
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
