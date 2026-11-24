//! Frontend manifests as the starter reads them, where they were found, and how far they are trusted (ADR 0094 items 2, 3, 7).
//!
//! Pattern: Repository (`ManifestSource`; standard [`crate::XdgManifests`], a fake in the suite) + Value objects.

use std::path::PathBuf;

use fs_types::coded::Coded;
use fs_types::interface::InterfaceReq;

use crate::codes::StarterCode;
use semver::Version;

use crate::names::{FrontendId, ProgramName};

/// The few fields of an AppStream `addon` manifest the starter reads (ADR 0094 O1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontendManifest {
    /// `fs:frontend-id`; equal to the file name `<id>.metainfo.xml`.
    pub id: FrontendId,
    /// `<extends>`: the AppStream id of the program.
    pub extends: String,
    /// `fs:exec`: a bare program file name, no path, no shell.
    pub exec: String,
    /// Every `fs:requires` (`history@>=0.1.0, <0.2.0`, `name@<req>`): the server's contract must meet each.
    pub requires: Vec<InterfaceReq>,
    /// `<requires><id version="X.Y.Z" compare="ge">` of the program, if given.
    pub min_program: Option<Version>,
    /// `fs:default` (`true`/`false`, missing = false): this installation asks for this frontend as the default (ADR 0094 item 4 b).
    pub default: bool,
}

/// Which level a manifest came from; the user level wins over the system level for the same id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// `$XDG_DATA_HOME/freesynergy/frontends/<program>/`.
    User,
    /// `<XDG_DATA_DIRS>/freesynergy/frontends/<program>/` (rpm/deb).
    System,
}

/// How far a manifest is trusted. Shown, never hidden (ADR 0094 item 7, ADR 0074 §9.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trust {
    /// Put there by a package (system level).
    Packaged,
    /// Put there by hand, or not provably by a package: the person is told "local, not checked".
    LocalUnchecked,
}

/// A manifest and where it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    /// The manifest.
    pub manifest: FrontendManifest,
    /// The level.
    pub place: Place,
    /// The trust.
    pub trust: Trust,
    /// The file.
    pub file: PathBuf,
}

/// The manifests of one program: the usable ones (user level first, one per id) and the refused ones (each a code).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Listing {
    /// Usable manifests, one per id.
    pub found: Vec<Found>,
    /// Refused files (`manifest-invalid` with `file`, `field`): the starter tells each one.
    pub refused: Vec<Coded<StarterCode>>,
}

impl Listing {
    /// The first usable manifest that asks to be the default (listing order: user level before system level, files sorted).
    #[must_use]
    pub fn default_frontend(&self) -> Option<&Found> {
        self.found.iter().find(|f| f.manifest.default)
    }

    /// The manifest of `id`, if usable.
    #[must_use]
    pub fn get(&self, id: &FrontendId) -> Option<&Found> {
        self.found.iter().find(|f| &f.manifest.id == id)
    }
}

/// Where frontend manifests come from.
pub trait ManifestSource {
    /// The manifests for `program`, read now (the menu reads again when it opens: hot plug, ADR 0094 item 3).
    fn frontends(&self, program: &ProgramName) -> Listing;
}
