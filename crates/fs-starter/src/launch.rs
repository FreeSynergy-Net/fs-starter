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
use std::time::{Duration, Instant};

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
    check_limit: Duration,
}

/// How long `--check` may run before the frontend is ended and counts as not startable (changeable: [`ProcessLauncher::with_check_limit`]).
pub const CHECK_LIMIT: Duration = Duration::from_secs(10);

impl ProcessLauncher {
    /// Looks in `dirs`, in this order.
    #[must_use]
    pub fn new(dirs: Vec<PathBuf>) -> Self {
        Self {
            dirs,
            check_limit: CHECK_LIMIT,
        }
    }

    /// The same launcher with another time limit for `--check`.
    #[must_use]
    pub fn with_check_limit(mut self, limit: Duration) -> Self {
        self.check_limit = limit;
        self
    }

    /// The folder of the running program, then (Unix) `/usr/bin`. The store's `current/<frontend package>/` comes in with
    /// fs-packages stage 2 (ADR 0094 O4).
    #[must_use]
    pub fn standard() -> Self {
        let beside = std::env::current_exe()
            .ok()
            .and_then(|me| me.parent().map(Path::to_path_buf));
        #[cfg(unix)]
        let system = [PathBuf::from(crate::system::STARTER_DIR)];
        #[cfg(not(unix))]
        let system: [PathBuf; 0] = [];
        Self::new(beside.into_iter().chain(system).collect())
    }
}

impl Launcher for ProcessLauncher {
    fn find(&self, exec: &str) -> Option<PathBuf> {
        // a bare name: no folder part and no ending (the ending is set only by `program_file`)
        if exec.is_empty()
            || exec.contains(['/', '\\'])
            || crate::layout::has_suffix(exec, crate::layout::EXE_SUFFIX)
        {
            return None;
        }
        self.dirs
            .iter()
            .map(|d| crate::layout::program_file(d, exec))
            .find(|p| p.is_file())
    }

    fn check(&self, exe: &Path, words: &[OsString]) -> Option<i32> {
        let quiet = std::process::Stdio::null;
        let status = Command::new(exe)
            .arg("--check")
            .args(words)
            .stdin(quiet())
            .stdout(quiet())
            .stderr(quiet())
            .spawn();
        let mut child = status.ok()?;
        let end = Instant::now() + self.check_limit;
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return status.code(),
                Ok(None) if Instant::now() < end => std::thread::sleep(Duration::from_millis(10)),
                // Too slow, or not askable any more: the frontend is ended and counts as not startable.
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
            }
        }
    }

    fn open(&self, exe: &Path, words: &[OsString]) -> io::Result<i32> {
        Ok(Command::new(exe).args(words).status()?.code().unwrap_or(1))
    }
}
