//! The system places of the packages and the check "this came from the package manager / the installer" (plan `ENTSCHEID-schnitt`
//! (b) and WIN-HOST (2), (3)): below one of the places, no link anywhere on the way, owned by the system, and writable by nobody else.
//! Unix (rpm world): the starter in `/usr/bin`, the modules in `/usr/libexec/freesynergy/modules`, the data packages (programs) in
//! `/usr/share/freesynergy/programs/<name>`; the owner is root. Windows: ONE place, the anchor `<FOLDERID_ProgramFiles>\FreeSynergy`
//! (the starter, `modules\`, `programs\`, `frontends\`), owned by SYSTEM, Administrators or `TrustedInstaller`. dnf and the installer
//! check the signature of the package; there is no second signature check.
//!
//! The check of ONE part of the way is not written here: it is the module `fs-trusted-path` (one module for all programs, no copy).
//! This file decides only WHICH places count (the policy) and walks the way from the top.
//!
//! Pattern: Specification (a check over the file system metadata; no state) + Value (the refusal says which border was hit).

use std::fmt;
use std::path::{Component, Path, PathBuf};

use fs_trusted_path::{Level, PlaceRefusal, Trust};

/// The folder of the starter binary of the rpm.
#[cfg(unix)]
pub const STARTER_DIR: &str = "/usr/bin";
/// The folder of the module binaries of the rpm (`fs-module-<x>`).
#[cfg(unix)]
pub const MODULES_DIR: &str = "/usr/libexec/freesynergy/modules";
/// The folder of the data packages (`<PROGRAMS_DIR>/<name>/`: `scaffold.toml`, `forms/`, texts).
#[cfg(unix)]
pub const PROGRAMS_DIR: &str = "/usr/share/freesynergy/programs";
/// The folder of the self descriptions and frontends of the system level (`metainfo`, `frontends/`).
#[cfg(unix)]
pub const SHARE_DIR: &str = "/usr/share/freesynergy";

/// The one place of the Windows installer (`<ProgramFiles>\FreeSynergy`); `None` where the shell does not tell. Never from an
/// environment variable.
#[cfg(windows)]
fn anchor() -> Option<PathBuf> {
    crate::known_folder::anchor()
}

/// The system folder of the modules (`<name>/service[.exe]`, `<name>/catalog/`): Unix `/usr/libexec/freesynergy/modules`, Windows
/// `<anchor>\modules`; `None` for a system with neither.
#[must_use]
#[allow(clippy::unnecessary_wraps)] // one signature on every system: Unix always has the folder, Windows only if the shell tells
pub fn modules_dir() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        Some(PathBuf::from(MODULES_DIR))
    }
    #[cfg(windows)]
    {
        anchor().map(|a| a.join("modules"))
    }
    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

/// The folder of the data packages (`<name>/`): Unix `/usr/share/freesynergy/programs`, Windows `<anchor>\programs`.
#[must_use]
#[allow(clippy::unnecessary_wraps)] // one signature on every system: Unix always has the folder, Windows only if the shell tells
pub fn programs_dir() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        Some(PathBuf::from(PROGRAMS_DIR))
    }
    #[cfg(windows)]
    {
        anchor().map(|a| a.join("programs"))
    }
    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

/// The folder of the shared files of the system level (`frontends\`, `metainfo\`) where it is not `SHARE_DIR`: Windows the anchor.
#[cfg(not(unix))]
#[must_use]
#[allow(clippy::unnecessary_wraps)] // one signature on every system: Unix always has the folder, Windows only if the shell tells
pub fn share_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        anchor()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Why a path does not count as packaged (one border each; the first one hit is told).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemRefusal {
    /// The path is not absolute, or has `.` or `..` in it.
    NotAbsolute,
    /// The path is not below one of the system places.
    OutsideSystem,
    /// Windows: a part of the way is not on a local fixed disk (network drive, `subst`, removable media).
    NotLocalDisk,
    /// A part of the way (also the last one) is a link (Unix: symlink; Windows: any reparse point).
    Symlink,
    /// A part of the way is not owned by the system owner.
    WrongOwner,
    /// A part of the way can be written by somebody who is not the system (a user can swap the content).
    Writable,
    /// A part of the way cannot be read, or is not there.
    Unreadable,
    /// A system with neither a Unix nor a Windows form: nothing is packaged in this sense.
    NotSupported,
}

impl From<PlaceRefusal> for SystemRefusal {
    fn from(why: PlaceRefusal) -> Self {
        match why {
            PlaceRefusal::NotAbsolute => Self::NotAbsolute,
            PlaceRefusal::NotLocalDisk => Self::NotLocalDisk,
            PlaceRefusal::Symlink => Self::Symlink,
            PlaceRefusal::WrongOwner => Self::WrongOwner,
            PlaceRefusal::Writable => Self::Writable,
            PlaceRefusal::Unreadable => Self::Unreadable,
            PlaceRefusal::NotSupported => Self::NotSupported,
        }
    }
}

impl fmt::Display for SystemRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotAbsolute => "not-absolute",
            Self::OutsideSystem => "outside-system",
            Self::NotLocalDisk => "not-local-disk",
            Self::Symlink => "symlink",
            Self::WrongOwner => "wrong-owner",
            Self::Writable => "writable",
            Self::Unreadable => "unreadable",
            Self::NotSupported => "not-supported",
        })
    }
}

/// The system places and who counts as the system.
#[derive(Clone, Debug)]
pub struct SystemPlaces {
    roots: Vec<PathBuf>,
    trust: Trust,
    /// The folder of the data packages (below the first place in tests).
    programs: PathBuf,
    /// Check every folder from the top down (real system; above the first place with the rule "only what can swap the child"); else
    /// from the place itself down (a test folder under a user's home).
    from_fs_root: bool,
}

impl SystemPlaces {
    /// The real places, the real system owners, checked from the top. Where the system has no places (no known folder, other system)
    /// there are none: every path is outside.
    #[must_use]
    pub fn standard() -> Self {
        #[cfg(unix)]
        let roots: Vec<PathBuf> = [STARTER_DIR, MODULES_DIR, SHARE_DIR]
            .iter()
            .map(PathBuf::from)
            .collect();
        #[cfg(not(unix))]
        let roots: Vec<PathBuf> = share_dir().into_iter().collect();
        Self {
            roots,
            trust: Trust::system(),
            programs: programs_dir().unwrap_or_default(),
            from_fs_root: true,
        }
    }

    /// Other places and the owner (a uid), checked from the place itself down. Only for tests (a test folder is never root's); behind
    /// the feature `test-support`, which only dev-dependencies switch on.
    #[cfg(all(unix, feature = "test-support"))]
    #[doc(hidden)]
    #[must_use]
    pub fn for_test(roots: Vec<PathBuf>, owner: u32) -> Self {
        Self::for_test_trust(
            roots,
            Trust::for_test(vec![fs_trusted_path::Owner::uid(owner)]),
        )
    }

    /// Other places, owned by the user who runs the test, checked from the place itself down. Only for tests (as above).
    ///
    /// # Panics
    /// The user of the process is not known (never on a working system).
    #[cfg(all(windows, feature = "test-support"))]
    #[doc(hidden)]
    #[must_use]
    pub fn for_test_as_user(roots: Vec<PathBuf>) -> Self {
        let owners = fs_trusted_path::windows::sid::my_owners().expect("the user of the process");
        Self::for_test_trust(roots, Trust::for_test(owners))
    }

    #[cfg(feature = "test-support")]
    fn for_test_trust(roots: Vec<PathBuf>, trust: Trust) -> Self {
        Self {
            programs: roots
                .first()
                .map_or_else(PathBuf::new, |r| r.join("programs")),
            roots,
            trust,
            from_fs_root: false,
        }
    }

    /// The data package folder of a program: `<programs>/<name>`, only for a plain name, and only if it passes [`Self::check`].
    ///
    /// # Errors
    /// [`SystemRefusal::NotAbsolute`] for a name that is not a plain folder name, else the refusal of [`Self::check`].
    pub fn program_dir(&self, name: &str) -> Result<PathBuf, SystemRefusal> {
        if !plain_name(name) {
            return Err(SystemRefusal::NotAbsolute);
        }
        let dir = self.programs.join(name);
        self.check(&dir).map(|()| dir)
    }

    /// Does `path` count as packaged? Below a system place, no link, owned by the system, not writable by anybody else, on every part of
    /// the way (also the folders above, for the real places; above the place only what can swap the child counts on Windows).
    ///
    /// # Errors
    /// The first border that is hit, see [`SystemRefusal`].
    pub fn check(&self, path: &Path) -> Result<(), SystemRefusal> {
        if !path.is_absolute()
            || path
                .components()
                .any(|c| matches!(c, Component::CurDir | Component::ParentDir))
        {
            return Err(SystemRefusal::NotAbsolute);
        }
        let root = self
            .roots
            .iter()
            .find(|r| path.starts_with(r))
            .ok_or(SystemRefusal::OutsideSystem)?;
        let root_parts = root.components().count();
        let start = if self.from_fs_root { 0 } else { root_parts };
        let mut walk = PathBuf::new();
        for (i, part) in path.components().enumerate() {
            walk.push(part);
            // `C:` alone is no folder (the current folder of the drive): the first part is the root `C:\`
            if !walk.has_root() {
                continue;
            }
            // the places of a test start below the anchor: the anchor itself is checked too (index start - 1)
            if i + 1 >= start.max(1) {
                let level = if i + 1 < root_parts {
                    Level::AboveAnchor
                } else {
                    Level::Anchor
                };
                self.trust.system_owned(&walk, level)?;
            }
        }
        Ok(())
    }
}

/// A plain folder name: `[a-z0-9_-]+` (no dot, no slash: no way out of the folder).
fn plain_name(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}
