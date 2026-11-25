//! When it is fair to act: the package transaction is over, the file stayed as it is, and (for the own window) the person is calm.
//!
//! Pattern: Strategy (`Transaction`: is a package manager at work; `Calm`: is it fine to end the window) + State (`Settle`: the
//! same new file for long enough). ADR 0069 section 1: restart only when the package transaction is done (file stable, lock of
//! dnf/apt free), else the window starts half updated.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::stamp::Stamp;

/// How long the new file must stay the same before it counts as settled.
pub const SETTLE: Duration = Duration::from_secs(10);

/// Is a package manager at work?
pub trait Transaction: Send {
    /// `true` while a transaction holds its lock.
    fn busy(&self) -> bool;
}

/// Is it fine to end the own window now (the rest rule of ADR 0069 addendum, item 5: no input focus, no modal, nothing running)?
pub trait Calm: Send {
    /// `true` when the person is calm. The host has no such signal yet: see [`NoSignal`].
    fn calm(&self) -> bool;
}

/// The host has no signal of the window for "calm" yet: never calm, so the own window is never ended behind a person's back
/// (the update then comes at the next start, ADR 0069 item 5: no forced restart). A card asks for the signal.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoSignal;

impl Calm for NoSignal {
    fn calm(&self) -> bool {
        false
    }
}

/// The lock files of the package managers of Linux. Which file dnf 5 holds is not proven here (`[nicht belegt]`): the list is
/// data, a name more is one line more.
///
/// `dnf.pid` (dnf 4 / yum): present while dnf runs. `.rpm.lock`: held with `flock` by rpm during a transaction.
/// `lock-frontend`/`lock`: held with `fcntl` by apt/dpkg.
#[derive(Clone, Debug)]
pub struct PackageLocks {
    pid_files: Vec<PathBuf>,
    flock_files: Vec<PathBuf>,
}

impl PackageLocks {
    /// The standard places.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            pid_files: vec!["/run/dnf.pid".into(), "/var/run/dnf.pid".into()],
            flock_files: vec![
                "/var/lib/rpm/.rpm.lock".into(),
                "/var/lib/dpkg/lock-frontend".into(),
            ],
        }
    }

    /// Other places (tests).
    #[must_use]
    pub fn at(pid_files: Vec<PathBuf>, flock_files: Vec<PathBuf>) -> Self {
        Self {
            pid_files,
            flock_files,
        }
    }
}

impl Transaction for PackageLocks {
    fn busy(&self) -> bool {
        self.pid_files.iter().any(|p| p.exists()) || self.flock_files.iter().any(held)
    }
}

/// A lock file that someone else holds: a try-lock fails. Opened read only (never created, never written); a file that is not there
/// or cannot be opened is not held (a reader without rights cannot tell, and says "not busy": the file stability still has to hold).
#[cfg(unix)]
fn held(path: &PathBuf) -> bool {
    use std::os::fd::AsRawFd;
    let Ok(f) = std::fs::File::open(path) else {
        return false;
    };
    // SAFETY: a plain try-lock on a descriptor we own; it is released when `f` is dropped.
    let got = unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_SH | libc::LOCK_NB) };
    got != 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock
}

#[cfg(not(unix))]
fn held(_: &PathBuf) -> bool {
    false
}

/// The new file counts as settled when it did not change for [`SETTLE`].
#[derive(Clone, Copy, Debug, Default)]
pub struct Settle {
    since: Option<(Stamp, Instant)>,
}

impl Settle {
    /// Feed what the file is now; `true` once the same stamp has stood for `after`.
    pub fn settled(&mut self, now: Stamp, at: Instant, after: Duration) -> bool {
        match self.since {
            Some((s, first)) if s == now => at.duration_since(first) >= after,
            _ => {
                self.since = Some((now, at));
                false
            }
        }
    }

    /// Forget (the file was acted upon).
    pub fn reset(&mut self) {
        self.since = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp(dir: &std::path::Path, text: &str) -> Stamp {
        let f = dir.join("f");
        std::fs::write(&f, text).unwrap();
        Stamp::of(&f).unwrap()
    }

    #[test]
    fn a_stamp_must_stand_for_the_whole_time_and_a_change_starts_over() {
        let dir = tempfile::tempdir().unwrap();
        let a = stamp(dir.path(), "a");
        let t0 = Instant::now();
        let mut s = Settle::default();
        assert!(!s.settled(a, t0, SETTLE), "first sight never settles");
        assert!(!s.settled(a, t0 + Duration::from_secs(5), SETTLE));
        assert!(s.settled(a, t0 + SETTLE, SETTLE));
        let b = stamp(dir.path(), "bb");
        assert!(
            !s.settled(b, t0 + SETTLE, SETTLE),
            "another file starts over"
        );
    }

    #[test]
    fn the_pid_file_of_dnf_means_busy_and_no_file_means_not_busy() {
        let dir = tempfile::tempdir().unwrap();
        let pid = dir.path().join("dnf.pid");
        let locks = PackageLocks::at(vec![pid.clone()], vec![dir.path().join("none")]);
        assert!(!locks.busy());
        std::fs::write(&pid, "1").unwrap();
        assert!(locks.busy());
    }

    #[cfg(unix)]
    #[test]
    fn a_flock_held_by_another_handle_means_busy() {
        use std::os::fd::AsRawFd;
        let dir = tempfile::tempdir().unwrap();
        let lock = dir.path().join(".rpm.lock");
        let holder = std::fs::File::create(&lock).unwrap();
        let locks = PackageLocks::at(vec![], vec![lock]);
        assert!(!locks.busy());
        // SAFETY: exclusive lock on our own descriptor, released with it.
        assert_eq!(unsafe { libc::flock(holder.as_raw_fd(), libc::LOCK_EX) }, 0);
        assert!(locks.busy());
    }

    #[test]
    fn without_a_signal_the_window_is_never_calm() {
        assert!(!NoSignal.calm());
    }
}
