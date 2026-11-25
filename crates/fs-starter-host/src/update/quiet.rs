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

/// Pattern: Strategy – is a package manager at work?
pub trait Transaction: Send {
    /// `true` while a transaction holds its lock.
    fn busy(&self) -> bool;
}

/// Pattern: Strategy – is the person calm enough that the own window may end?
///
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

/// The locks of the package managers of Linux, read from outside (never taken).
///
/// dnf 4 / yum: `/run/dnf.pid` exists while dnf runs. rpm: `.rpm.lock` is held with `fcntl` (POSIX lock; `lib/rpmlock.c`, read, not
/// measured on a running transaction). dpkg: `lock-frontend` and `lock` with `fcntl` too. dnf 5: `/run/dnf/rpmtransaction.lock`; which kind of lock
/// it is, is not proven (`[nicht belegt]`), so every file is asked both ways (`fcntl` and `flock`: on Linux neither sees the other). The list is
/// data, a name more is one line more.
#[derive(Clone, Debug)]
pub struct PackageLocks {
    pid_files: Vec<PathBuf>,
    lock_files: Vec<PathBuf>,
}

impl PackageLocks {
    /// The standard places.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            pid_files: vec!["/run/dnf.pid".into(), "/var/run/dnf.pid".into()],
            lock_files: vec![
                "/var/lib/rpm/.rpm.lock".into(),
                "/run/dnf/rpmtransaction.lock".into(),
                "/var/lib/dpkg/lock-frontend".into(),
                "/var/lib/dpkg/lock".into(),
            ],
        }
    }

    /// Other places (tests).
    #[must_use]
    pub fn at(pid_files: Vec<PathBuf>, lock_files: Vec<PathBuf>) -> Self {
        Self {
            pid_files,
            lock_files,
        }
    }
}

impl Transaction for PackageLocks {
    fn busy(&self) -> bool {
        self.pid_files.iter().any(|p| p.exists()) || self.lock_files.iter().any(held)
    }
}

/// A lock file that another process holds, as `fcntl` (`F_GETLK`: asks, takes nothing, needs no write right) or as `flock` (try-lock). Opened
/// read only, never created or written; a file that is not there or cannot be opened is not held (the file stability still has to hold).
/// A small constant of `libc` as the `short` of `struct flock`.
#[cfg(unix)]
fn short(v: libc::c_int) -> libc::c_short {
    libc::c_short::try_from(v).unwrap_or(libc::c_short::MAX)
}

#[cfg(unix)]
fn held(path: &PathBuf) -> bool {
    use std::os::fd::AsRawFd;
    let Ok(f) = std::fs::File::open(path) else {
        return false;
    };
    let fd = f.as_raw_fd();
    // SAFETY: `flock` is plain data, zeroed is a valid start; `F_GETLK` only fills it in.
    let mut probe: libc::flock = unsafe { std::mem::zeroed() };
    probe.l_type = short(libc::F_WRLCK);
    probe.l_whence = short(libc::SEEK_SET);
    // SAFETY: a query on a descriptor we own; whole file (start 0, length 0); nothing is taken.
    let by_fcntl = unsafe { libc::fcntl(fd, libc::F_GETLK, &raw mut probe) } == 0
        && probe.l_type != short(libc::F_UNLCK);
    // SAFETY: a try-lock on a descriptor we own; it is released when `f` is dropped.
    let by_flock = unsafe { libc::flock(fd, libc::LOCK_SH | libc::LOCK_NB) } != 0
        && std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock;
    by_fcntl || by_flock
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
        let lock = dir.path().join("flock");
        let holder = std::fs::File::create(&lock).unwrap();
        let locks = PackageLocks::at(vec![], vec![lock]);
        assert!(!locks.busy());
        // SAFETY: exclusive lock on our own descriptor, released with it.
        assert_eq!(unsafe { libc::flock(holder.as_raw_fd(), libc::LOCK_EX) }, 0);
        assert!(locks.busy());
    }

    /// What rpm and dpkg do: a POSIX lock (`fcntl`) of ANOTHER process (such a lock belongs to a process, so the holder is a child).
    #[cfg(unix)]
    #[test]
    fn an_fcntl_lock_of_another_process_means_busy_and_ending_it_frees_it() {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        let dir = tempfile::tempdir().unwrap();
        let lock = dir.path().join(".rpm.lock");
        std::fs::write(&lock, "").unwrap();
        let path = CString::new(lock.as_os_str().as_bytes()).unwrap();
        let locks = PackageLocks::at(vec![], vec![lock]);
        assert!(!locks.busy());
        let (mut up, mut down) = ([0; 2], [0; 2]); // child to parent, parent to child
        // SAFETY: plain system calls; the child only uses async-signal-safe ones (open, fcntl, write, read, _exit).
        unsafe {
            assert_eq!(libc::pipe(up.as_mut_ptr()), 0);
            assert_eq!(libc::pipe(down.as_mut_ptr()), 0);
            let pid = libc::fork();
            assert!(pid >= 0);
            if pid == 0 {
                let fd = libc::open(path.as_ptr(), libc::O_RDWR);
                let mut l: libc::flock = std::mem::zeroed();
                l.l_type = super::short(libc::F_WRLCK);
                l.l_whence = super::short(libc::SEEK_SET);
                let ok = fd >= 0 && libc::fcntl(fd, libc::F_SETLK, &raw mut l) == 0;
                let mark = [u8::from(ok)];
                libc::write(up[1], mark.as_ptr().cast(), 1);
                let mut hold = [0_u8; 1];
                libc::read(down[0], hold.as_mut_ptr().cast(), 1); // until the parent lets go
                libc::_exit(0);
            }
            let mut mark = [0_u8; 1];
            libc::read(up[0], mark.as_mut_ptr().cast(), 1);
            assert_eq!(mark[0], 1, "the child holds the lock");
            let busy = locks.busy();
            let go = [1_u8];
            libc::write(down[1], go.as_ptr().cast(), 1);
            let mut status = 0;
            libc::waitpid(pid, &raw mut status, 0);
            for fd in up.into_iter().chain(down) {
                libc::close(fd);
            }
            assert!(busy, "an fcntl lock of another process must be seen");
        }
        assert!(!locks.busy(), "free again after the holder ended");
    }

    #[test]
    fn without_a_signal_the_window_is_never_calm() {
        assert!(!NoSignal.calm());
    }
}
