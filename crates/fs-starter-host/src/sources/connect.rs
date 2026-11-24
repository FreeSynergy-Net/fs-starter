//! The port `Sources` of `fs-forms-run` for the modules of the starter: `connect` reads the module (declared forms, pages) and, for a
//! window, then HOLDS it ([`super::held`]). Split from `starter.rs` (the facade) to keep both small.
//!
//! Pattern: Adapter (the port `Sources`) over the facade `StarterSources`; the hold is in [`HeldRecords`].

use std::path::Path;
use std::sync::Arc;

use fs_forms_run::{Coded, FormRecords, Records, Sources};

use super::held::{HeldRecords, hold_module};
use super::starter::{StarterSources, line, read_lines};
use super::writer::ModuleWriter;

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
        // a source that is not there is said on stderr too (the window shows it as a notice): never silent
        let said = |code: Coded| {
            eprintln!("fs-starter: {}", line(&code));
            code
        };
        let (known, forms) = self.declared(range).map_err(said)?;
        let began = std::time::Instant::now();
        let records = self.read(known, &forms).map_err(said)?;
        // what was read, never silent: an empty picture can be told from a service that gave nothing
        for line in read_lines(known.name.as_str(), &records, began.elapsed()) {
            eprintln!("{line}");
        }
        // a window holds its module (the module ends by itself after its idle time once the records are dropped); a one-time command
        // ended the service it started and must not start it again
        let held = if self.end_started {
            None
        } else {
            Some(hold_module(known, &self.limits, say).map_err(said)?)
        };
        // a gesture of the window goes back to this module through the port `Writer`
        let writer = ModuleWriter::new(self.clone(), &known.name, forms);
        Ok(Arc::new(HeldRecords::new(
            FormRecords {
                writer: Some(Arc::new(writer)),
                ..records
            },
            held,
        )))
    }
}

impl Sources for StarterSources {
    /// The `socket` of the command line is NOT used: a module is reached only on its own socket (border 4).
    fn connect(&self, range: &str, _: Option<&Path>, _: &Path) -> Result<Arc<dyn Records>, Coded> {
        self.connect_saying(range, |line| eprintln!("{line}"))
    }
}
