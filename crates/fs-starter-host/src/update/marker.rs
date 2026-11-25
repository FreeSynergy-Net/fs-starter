//! The mark that survives the process: what the next start has to say about the last run (a crash, or a restart without a session).
//!
//! Pattern: Memento (the last run writes its state, the next start reads and says it) + Value object (`Said`). A panic or a signal skips
//! `after_end` (see `shell::around_window`): so the file is written at the START as "running" and removed at a clean end; one that
//! is still there at the next start is a crash. The file is a few `key=value` lines in the state folder of the person, not data of
//! the person: it moves into the database (F10) with the session (SB-12), a card says so.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// What the file says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Started and not ended (a clean end removes the file).
    Running,
    /// Ended with this code that is not 0.
    Exited(i32),
    /// The host ended itself to take over a new file; the session was not restored (no `fs-session`).
    Restarted,
}

/// The mark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mark {
    /// What happened.
    pub state: State,
    /// The version that ran.
    pub version: String,
    /// Seconds since 1970 at the start.
    pub since: u64,
    /// This run was the first after the host took over a new file (then a crash is "crash after update").
    pub after_update: bool,
}

/// What the next start says about the last run: one line with a stable code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Said(pub String);

impl Mark {
    /// A mark for a run that starts now.
    #[must_use]
    pub fn running(version: &str, after_update: bool) -> Self {
        let since = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        Self {
            state: State::Running,
            version: version.to_owned(),
            since,
            after_update,
        }
    }

    fn text(&self) -> String {
        let state = match self.state {
            State::Running => "running".to_owned(),
            State::Exited(c) => format!("exited:{c}"),
            State::Restarted => "restarted".to_owned(),
        };
        format!(
            "state={state}\nversion={}\nsince={}\nafter_update={}\n",
            self.version,
            self.since,
            u8::from(self.after_update)
        )
    }

    fn parse(text: &str) -> Option<Self> {
        let get = |key: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(key)?.strip_prefix('='))
        };
        let state = match get("state")? {
            "running" => State::Running,
            "restarted" => State::Restarted,
            other => State::Exited(other.strip_prefix("exited:")?.parse().ok()?),
        };
        Some(Self {
            state,
            version: get("version")?.chars().take(64).collect(),
            since: get("since")?.parse().ok()?,
            after_update: get("after_update")? == "1",
        })
    }

    /// The line the next start says.
    #[must_use]
    pub fn said(&self, program: &str) -> Said {
        let v = &self.version;
        Said(match (self.state, self.after_update) {
            (State::Restarted, _) => format!(
                "update-session-not-restored program={program} version={v} note=\"restarted for the new file; the session is not restored (fs-session is not there)\""
            ),
            (State::Running, true) => format!(
                "update-crash-after-update program={program} version={v} exit=unknown note=\"the last run ended without a word right after an update (crash or signal)\""
            ),
            (State::Running, false) => format!(
                "update-crash program={program} version={v} exit=unknown note=\"the last run ended without a word (crash or signal)\""
            ),
            (State::Exited(c), true) => format!(
                "update-crash-after-update program={program} version={v} exit={c} note=\"the first run after an update ended with a failure\""
            ),
            (State::Exited(c), false) => format!(
                "update-crash program={program} version={v} exit={c} note=\"the last run ended with a failure\""
            ),
        })
    }
}

/// The place of the mark of one program.
#[derive(Clone, Debug)]
pub struct Marker {
    file: PathBuf,
}

impl Marker {
    /// One mark for each window (`instance`: the slot of `--new-instance`, `None` for the first), so a second window neither tells a crash that
    /// did not happen nor removes the mark of the other.
    /// `<state>/freesynergy/<program>.update`; `state` is `$XDG_STATE_HOME`, else `~/.local/state` (Windows: `%LOCALAPPDATA%`).
    /// `None` without any of them (no fallback below `/tmp`): the host says `update-marker-off` and goes on.
    #[must_use]
    pub fn standard(program: &str, instance: Option<u8>) -> Option<Self> {
        let env = |k: &str| {
            std::env::var_os(k)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        let base = env("XDG_STATE_HOME")
            .or_else(|| env("LOCALAPPDATA"))
            .or_else(|| env("HOME").map(|h| h.join(".local/state")))?;
        let name = instance.map_or_else(|| program.to_owned(), |n| format!("{program}-{n}"));
        Some(Self::in_folder(&base.join("freesynergy"), &name))
    }

    /// In a given folder (tests). The program name is a file name: only letters, digits and `-` `_` stay.
    #[must_use]
    pub fn in_folder(folder: &Path, program: &str) -> Self {
        let name: String = program
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        Self {
            file: folder.join(format!("{name}.update")),
        }
    }

    /// The mark of the last run, if it left one (one that cannot be read is said as a crash with an unknown cause: never silent).
    #[must_use]
    pub fn read(&self) -> Option<Mark> {
        let text = fs::read_to_string(&self.file).ok()?;
        Some(Mark::parse(&text).unwrap_or_else(|| Mark {
            state: State::Running,
            version: "unknown".to_owned(),
            since: 0,
            after_update: false,
        }))
    }

    /// Write the mark (replaces the earlier one).
    ///
    /// # Errors
    /// The folder or the file cannot be written.
    pub fn write(&self, mark: &Mark) -> io::Result<()> {
        if let Some(dir) = self.file.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = self.file.with_extension("update.tmp");
        fs::write(&tmp, mark.text())?;
        fs::rename(&tmp, &self.file)
    }

    /// Remove the mark (a clean end).
    pub fn clear(&self) {
        let _ = fs::remove_file(&self.file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mark_survives_a_write_and_says_what_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let m = Marker::in_folder(dir.path(), "starter");
        assert!(m.read().is_none());
        let mut mark = Mark::running("0.0.8", true);
        m.write(&mark).unwrap();
        assert_eq!(m.read().unwrap(), mark);
        mark.state = State::Exited(3);
        m.write(&mark).unwrap();
        let said = m.read().unwrap().said("starter").0;
        assert!(
            said.starts_with("update-crash-after-update program=starter version=0.0.8 exit=3"),
            "{said}"
        );
        m.clear();
        assert!(m.read().is_none());
    }

    #[test]
    fn the_codes_tell_crash_exit_and_restart_apart() {
        let r = |state, after_update| Mark {
            state,
            version: "1".into(),
            since: 0,
            after_update,
        };
        let code = |m: Mark| m.said("p").0.split(' ').next().unwrap().to_owned();
        assert_eq!(code(r(State::Running, false)), "update-crash");
        assert_eq!(code(r(State::Running, true)), "update-crash-after-update");
        assert_eq!(code(r(State::Exited(1), false)), "update-crash");
        assert_eq!(
            code(r(State::Restarted, false)),
            "update-session-not-restored"
        );
    }

    #[test]
    fn a_file_that_cannot_be_read_is_a_crash_with_an_unknown_cause_not_silence() {
        let dir = tempfile::tempdir().unwrap();
        let m = Marker::in_folder(dir.path(), "a/b");
        fs::write(dir.path().join("a_b.update"), "garbage").unwrap();
        assert_eq!(m.read().unwrap().state, State::Running);
    }
}
