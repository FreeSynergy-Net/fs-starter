//! Telling that a file was replaced: the identity of the file (Unix: device + inode) and its mtime and length.
//!
//! Pattern: Value object (`Stamp`) + Observer (`Watched` remembers what it saw at the start and compares). A package manager replaces a
//! file by a new file (rename), so the inode changes; mtime and length catch an overwrite in place. Polling, no `notify`: the
//! point in time that matters is not the event but the end of the package transaction (see `quiet.rs`), which no event tells.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// What is known of a file at one moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp {
    id: Option<(u64, u64)>,
    mtime: Option<SystemTime>,
    len: u64,
}

impl Stamp {
    /// The stamp of the file `path` points to (a link is followed: the package manager replaces the target, ADR 0069 path A);
    /// `None` when it is not there or not a file.
    #[must_use]
    pub fn of(path: &Path) -> Option<Self> {
        let m = fs::metadata(path).ok().filter(fs::Metadata::is_file)?;
        #[cfg(unix)]
        let id = {
            use std::os::unix::fs::MetadataExt;
            Some((m.dev(), m.ino()))
        };
        #[cfg(not(unix))]
        let id = None;
        Some(Self {
            id,
            mtime: m.modified().ok(),
            len: m.len(),
        })
    }
}

/// What the file is now, compared with the start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// The same file as at the start.
    Same,
    /// Another file stands there (new inode, mtime or length).
    Replaced(Stamp),
    /// The file is gone (a transaction is half way: no decision yet).
    Gone,
}

/// What a watched file is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Role {
    /// The own program file of the host.
    Own,
    /// The `service` of the module of this name.
    Module(String),
}

/// A file with the stamp it had when the watch began.
#[derive(Clone, Debug)]
pub struct Watched {
    /// The file.
    pub path: PathBuf,
    /// What it is for.
    pub role: Role,
    seen: Stamp,
}

impl Watched {
    /// Begin to watch; `None` when the file cannot be stamped now (a module without a service file is not ours to watch).
    #[must_use]
    pub fn begin(path: PathBuf, role: Role) -> Option<Self> {
        let seen = Stamp::of(&path)?;
        Some(Self { path, role, seen })
    }

    /// Compare with the start.
    #[must_use]
    pub fn check(&self) -> Change {
        match Stamp::of(&self.path) {
            None => Change::Gone,
            Some(now) if now == self.seen => Change::Same,
            Some(now) => Change::Replaced(now),
        }
    }

    /// Take the new file as the one seen (after it was acted upon).
    pub fn accept(&mut self, now: Stamp) {
        self.seen = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_replaced_by_rename_is_replaced_and_an_untouched_one_is_the_same() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("service");
        fs::write(&file, b"old").unwrap();
        let w = Watched::begin(file.clone(), Role::Own).unwrap();
        assert_eq!(w.check(), Change::Same);
        let next = dir.path().join("service.new");
        fs::write(&next, b"new, longer").unwrap();
        fs::rename(&next, &file).unwrap();
        assert!(matches!(w.check(), Change::Replaced(_)));
        fs::remove_file(&file).unwrap();
        assert_eq!(w.check(), Change::Gone);
    }

    #[test]
    fn nothing_to_watch_when_the_file_is_not_there() {
        let dir = tempfile::tempdir().unwrap();
        assert!(Watched::begin(dir.path().join("nope"), Role::Own).is_none());
        assert!(Watched::begin(dir.path().to_path_buf(), Role::Own).is_none());
    }
}
