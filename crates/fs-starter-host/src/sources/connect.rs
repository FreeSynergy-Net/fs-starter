//! The port `Sources` of `fs-forms-run` for the modules of the starter: `connect` reads the module (declared forms, pages) and, for a
//! window, then HOLDS it ([`super::held`]). Split from `starter.rs` (the facade) to keep both small.
//!
//! Pattern: Adapter (the port `Sources`) over the facade `StarterSources`; the hold is in [`HeldRecords`].

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use fs_forms_run::{Coded, FormRecords, Records, Sources};

use super::end::EndOnDrop;
use super::entries::{Wanted, select};
use super::held::{Held, HeldRecords, Wish, hold_module};
use super::level;
use super::service::socket_of;
use super::starter::{StarterSources, line, read_lines};
use super::watch::Identity;
use super::writer::ModuleWriter;
use crate::slots;

/// The windows of this process, counted: the second part of the instance of `introduce` (process id + window).
static WINDOWS: AtomicU32 = AtomicU32::new(0);

impl StarterSources {
    /// [`Sources::connect`] with the words of the hold (`fs-starter: hold module=M attached` ...) given to `say` instead of stderr.
    ///
    /// Like `connect` it blocks (the read, then the hold: both within the budget `limits.time`), so a window calls it off its drawing
    /// thread, and it never sets longer waits for the hold.
    ///
    /// # Errors
    /// The codes of [`Self::declared`], of the read, and `host-notice-data-unreachable` (cause `hold`).
    pub fn connect_saying(
        &self,
        range: &str,
        say: impl Fn(String) + Send + 'static,
    ) -> Result<Arc<dyn Records>, Coded> {
        self.connect_held(range, say)
            .map(|held| Arc::new(held) as Arc<dyn Records>)
    }

    /// [`Self::connect_saying`] as the concrete [`HeldRecords`]: what the window draws from, plus the commands of the host itself
    /// (`answer_app`) and the writer.
    ///
    /// # Errors
    /// As [`Self::connect_saying`].
    pub fn connect_held(
        &self,
        range: &str,
        say: impl Fn(String) + Send + 'static,
    ) -> Result<HeldRecords, Coded> {
        self.connect_wanted(range, &[], say)
    }

    /// [`Self::connect_held`] for the sources a layout names: each `(type, scope)` reads the entries of `forms.json` of that type, in
    /// its scope (an entry no source asks for is not read). Without sources every form is read once, in scope `""`.
    ///
    /// # Errors
    /// As [`Self::connect_saying`].
    pub fn connect_wanted(
        &self,
        range: &str,
        wanted: &[Wanted],
        say: impl Fn(String) + Send + 'static,
    ) -> Result<HeldRecords, Coded> {
        // a source that is not there is said on stderr too (the window shows it as a notice): never silent
        let said = |code: Coded| {
            eprintln!("fs-starter: {}", line(&code));
            code
        };
        let (known, entries) = self.declared_entries(range).map_err(said)?;
        let forms = select(&entries, wanted);
        let began = std::time::Instant::now();
        // a one-time run reads WITHOUT ending: it holds the module like a window, and the guard ends what it started itself
        let (got, started) = if self.end_started {
            self.read_keeping(known, &forms)
        } else {
            (self.read_level(known, &forms), false)
        };
        let end = started
            .then(|| socket_of(&known.name).ok().map(EndOnDrop::new))
            .flatten();
        let (records, levels) = got.map_err(said)?;
        // what was read, never silent: an empty picture can be told from a service that gave nothing
        for line in read_lines(known.name.as_str(), &records, began.elapsed()) {
            eprintln!("{line}");
        }
        // the ONE line of the negotiated level (the service says its own; the host works with min(level, its own))
        eprintln!("{}", level::line(&known.name, levels.service, levels.used));
        // every run holds its module (a window until it is dropped; the module ends by itself after its idle time). At level 2 the run
        // also introduces itself and watches, so its writes are in the history (`app.undo`). A failed hold drops `end` and ends the
        // service this run started.
        let me = identity(self.program.as_deref(), self.slot);
        let wish = Wish {
            level: levels.used,
            forms: &forms,
            me,
        };
        let held = hold_module(known, &self.limits, wish, say).map_err(said)?;
        // a gesture of the window goes back to this module through the port `Writer`
        let session = match &held {
            Held::Session(s) => Some(Arc::clone(s)),
            Held::Plain(_) => None,
        };
        // the writer never ends a service: the hold stands and the guard of the records does it
        let mut sources = self.clone();
        sources.end_started = false;
        let writer = ModuleWriter::new(sources, &known.name, forms, session);
        Ok(HeldRecords::new(
            FormRecords {
                writer: Some(Arc::new(writer)),
                ..records
            },
            held,
            end,
        ))
    }
}

impl Sources for StarterSources {
    /// The `socket` of the command line is NOT used: a module is reached only on its own socket (border 4).
    fn connect(&self, range: &str, _: Option<&Path>, _: &Path) -> Result<Arc<dyn Records>, Coded> {
        self.connect_saying(range, |line| eprintln!("{line}"))
    }
}

/// Who a window says it is: the program (or `fs-starter`) and `<program>#<k>` for a numbered window, else process id and counter.
fn identity(program: Option<&str>, slot: Option<u8>) -> Identity {
    let program = program.unwrap_or("fs-starter").to_owned();
    let instance = match slot {
        Some(k) => slots::label(&program, k),
        None => format!(
            "{}/{}",
            std::process::id(),
            WINDOWS.fetch_add(1, Ordering::SeqCst) + 1
        ),
    };
    Identity { program, instance }
}

#[cfg(test)]
mod tests {
    use super::identity;

    #[test]
    fn a_numbered_window_says_program_and_number() {
        let me = identity(Some("tasks"), Some(3));
        assert_eq!(
            (me.program.as_str(), me.instance.as_str()),
            ("tasks", "tasks#3")
        );
        assert!(!identity(Some("tasks"), None).instance.contains('#'));
    }
}
