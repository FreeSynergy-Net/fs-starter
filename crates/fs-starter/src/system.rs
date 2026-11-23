//! The system places of the rpm packages and the check "this came from the package manager" (plan `ENTSCHEID-schnitt` (b)):
//! the starter itself in `/usr/bin`, the modules in `/usr/libexec/freesynergy/modules`, the data packages (programs) in
//! `/usr/share/freesynergy/programs/<name>`. dnf checks the signature of the package; the starter trusts the system level only if
//! the files are really the package manager's: below one of the places, no symlink anywhere on the way, owned by root, and
//! writable by nobody else. There is no second signature check.
//!
//! Unix only (the rpm world). Elsewhere every path is refused (`NotSupported`): the Windows installer has its own way (cfg split).
//!
//! Pattern: Specification (a pure check over the file system metadata; no state) + Value (the refusal says which border was hit).

use std::fmt;
use std::path::{Component, Path, PathBuf};

/// The folder of the starter binary of the rpm.
pub const STARTER_DIR: &str = "/usr/bin";
/// The folder of the module binaries of the rpm (`fs-module-<x>`).
pub const MODULES_DIR: &str = "/usr/libexec/freesynergy/modules";
/// The folder of the data packages (`<PROGRAMS_DIR>/<name>/`: `scaffold.toml`, `forms/`, texts).
pub const PROGRAMS_DIR: &str = "/usr/share/freesynergy/programs";
/// The folder of the self descriptions and frontends of the system level (`metainfo`, `frontends/`).
pub const SHARE_DIR: &str = "/usr/share/freesynergy";

/// Why a path does not count as packaged (one border each; the first one hit is told).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemRefusal {
    /// The path is not absolute, or has `.` or `..` in it.
    NotAbsolute,
    /// The path is not below one of the system places.
    OutsideSystem,
    /// A part of the way (also the last one) is a symlink.
    Symlink,
    /// A part of the way is not owned by the system owner (root).
    WrongOwner,
    /// A part of the way can be written by group or others (a user can swap the content).
    Writable,
    /// A part of the way cannot be read, or is not there.
    Unreadable,
    /// Not a unix system: nothing is packaged in this sense.
    NotSupported,
}

impl fmt::Display for SystemRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotAbsolute => "not-absolute",
            Self::OutsideSystem => "outside-system",
            Self::Symlink => "symlink",
            Self::WrongOwner => "wrong-owner",
            Self::Writable => "writable",
            Self::Unreadable => "unreadable",
            Self::NotSupported => "not-supported",
        })
    }
}

/// The system places and the owner they must have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemPlaces {
    roots: Vec<PathBuf>,
    owner: u32,
    /// The folder of the data packages (`PROGRAMS_DIR`; below the first place in tests).
    programs: PathBuf,
    /// Check every folder from `/` down (real system); else from the place itself down (a test folder under a user's home).
    from_fs_root: bool,
}

impl SystemPlaces {
    /// The real places, owner root (uid 0), checked from `/`.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            roots: [STARTER_DIR, MODULES_DIR, SHARE_DIR]
                .iter()
                .map(PathBuf::from)
                .collect(),
            owner: 0,
            programs: PathBuf::from(PROGRAMS_DIR),
            from_fs_root: true,
        }
    }

    /// Other places and owner, checked from the place itself down. Only for tests (a test folder is never root's).
    #[doc(hidden)]
    #[must_use]
    pub fn for_test(roots: Vec<PathBuf>, owner: u32) -> Self {
        Self {
            programs: roots
                .first()
                .map_or_else(PathBuf::new, |r| r.join("programs")),
            roots,
            owner,
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

    /// Does `path` count as packaged? Below a system place, no symlink, owned by the owner, not writable by group or others, on every
    /// part of the way (also the folders above, for the real places).
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
        let start = if self.from_fs_root {
            0
        } else {
            root.components().count()
        };
        let mut walk = PathBuf::new();
        for (i, part) in path.components().enumerate() {
            walk.push(part);
            // the places of a test start below the anchor: the anchor itself is checked too (index start - 1)
            if i + 1 >= start.max(1) {
                Self::check_one(self.owner, &walk)?;
            }
        }
        Ok(())
    }

    #[cfg(unix)]
    fn check_one(owner: u32, p: &Path) -> Result<(), SystemRefusal> {
        use std::os::unix::fs::MetadataExt;
        let m = std::fs::symlink_metadata(p).map_err(|_| SystemRefusal::Unreadable)?;
        if m.file_type().is_symlink() {
            return Err(SystemRefusal::Symlink);
        }
        if m.uid() != owner {
            return Err(SystemRefusal::WrongOwner);
        }
        if m.mode() & 0o022 != 0 {
            return Err(SystemRefusal::Writable);
        }
        Ok(())
    }

    #[cfg(not(unix))]
    fn check_one(_owner: u32, _p: &Path) -> Result<(), SystemRefusal> {
        Err(SystemRefusal::NotSupported)
    }
}

/// A plain folder name: `[a-z0-9_-]+` (no dot, no slash: no way out of the folder).
fn plain_name(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}
