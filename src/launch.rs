//! Finding and running a frontend's program file.
//!
//! Pattern: Strategy / Port (`Launcher`; the standard runs processes, the suite's fake records). Taken from activities
//! `starter.rs` `exe_of` (:81), `code_of` (:90), the `--check` call (:122) and the open (:138). Not the store's launcher
//! (fs-packages `fs-packages-app/src/launcher.rs:15`): that one starts other programs detached with a log pump and reports later;
//! this one runs the frontend of its own program and waits for its answer.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Runs frontends.
pub trait Launcher {
    /// The program file for `exec` (a bare name from `fs:exec`), looked up only in the launcher's own folders, never through
    /// `PATH` and never relative to the manifest (ADR 0094 item 3).
    fn find(&self, exec: &str) -> Option<PathBuf>;

    /// Runs `<exe> --check <words>` and waits: its exit code, or `None` when it could not run or a signal ended it.
    fn check(&self, exe: &Path, words: &[OsString]) -> Option<i32>;

    /// Runs `<exe> <words>` with the starter's terminal and waits for its end: its exit code (a signal is 1).
    ///
    /// # Errors
    /// The program file could not be run.
    fn open(&self, exe: &Path, words: &[OsString]) -> io::Result<i32>;
}

/// The standard launcher: plain processes, no shell. Folders: next to the running program (stage 1 bundles every binary,
/// ADR 0094 item 6), then `/usr/bin`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessLauncher {
    dirs: Vec<PathBuf>,
}

impl ProcessLauncher {
    /// Looks in `dirs`, in this order.
    #[must_use]
    pub fn new(dirs: Vec<PathBuf>) -> Self {
        Self { dirs }
    }

    /// The folder of the running program, then `/usr/bin`. The store's `current/<frontend package>/` comes in with
    /// fs-packages stage 2 (ADR 0094 O4).
    #[must_use]
    pub fn standard() -> Self {
        let beside = std::env::current_exe()
            .ok()
            .and_then(|me| me.parent().map(Path::to_path_buf));
        Self::new(
            beside
                .into_iter()
                .chain([PathBuf::from("/usr/bin")])
                .collect(),
        )
    }
}

impl Launcher for ProcessLauncher {
    fn find(&self, exec: &str) -> Option<PathBuf> {
        if exec.is_empty() || exec.contains(['/', '\\']) {
            return None;
        }
        self.dirs.iter().map(|d| d.join(exec)).find(|p| p.is_file())
    }

    fn check(&self, exe: &Path, words: &[OsString]) -> Option<i32> {
        let quiet = std::process::Stdio::null;
        let status = Command::new(exe)
            .arg("--check")
            .args(words)
            .stdin(quiet())
            .stdout(quiet())
            .stderr(quiet())
            .status();
        status.ok().and_then(|s| s.code())
    }

    fn open(&self, exe: &Path, words: &[OsString]) -> io::Result<i32> {
        Ok(Command::new(exe).args(words).status()?.code().unwrap_or(1))
    }
}
