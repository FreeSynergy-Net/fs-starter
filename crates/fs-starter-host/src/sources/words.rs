//! The words a module brings: `<module>/catalog/texts/` (`manifest.mf2m` + `<tag>.<hash>.mf2b`, packed by `fs-blueprint package module`),
//! loaded by `Mf2Texts::from_packed`. The window asks them after its own words (`ctx.say`: the chain is made by fs-forms-run).
//!
//! Pattern: Port adapter + Factory Method (`ModuleWords::in_locale` makes the words of a language from the folder of the package).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use fs_components_mf2::{Mf2Texts, NoDates};
use fs_components_words::{Args, Locale, Said, TextId, Words};
use fs_forms_run::ModuleWords;

/// The packed texts of one module. Dates stay neutral (`NoDates`): the date tables are language packages, not part of a module.
#[derive(Debug)]
pub struct PackedWords {
    dir: PathBuf,
}

/// No sentence at all: every key is missing, so the next in the chain answers (the window then shows the bare word id, e.g. `person.emails`).
struct Silent;

impl Words for Silent {
    fn say(&self, id: &TextId, _: &Args) -> Said {
        Said::missing(id)
    }
}

/// Whether the module in `folder` brings words that load: `Ok(false)` = no `catalog/texts/`, `Err` = the reader's words (English is tried:
/// the language every package must have).
///
/// # Errors
/// What `Mf2Texts::from_packed` says (manifest or catalog unreadable, no language).
pub fn check(folder: &Path) -> Result<bool, String> {
    let dir = folder.join("catalog").join("texts");
    if !dir.is_dir() {
        return Ok(false);
    }
    Mf2Texts::from_packed(&dir, &Locale::english(), &NoDates)
        .map(|_| true)
        .map_err(|e| format!("{e:?}"))
}

impl PackedWords {
    /// The words of the module in `folder`, or `None` when it has none or they cannot be loaded (said on stderr; `--check-system` says it
    /// as `no-module-texts` / `module-texts-broken`). Without words a head is the bare word id of the type catalog.
    #[must_use]
    pub fn of(folder: &Path, module: &str) -> Option<Arc<dyn ModuleWords>> {
        match check(folder) {
            Ok(true) => Some(Arc::new(Self {
                dir: folder.join("catalog").join("texts"),
            })),
            Ok(false) => None,
            Err(e) => {
                eprintln!("fs-starter: module-texts-broken module={module} why={e}");
                None
            }
        }
    }
}

impl ModuleWords for PackedWords {
    fn in_locale(&self, locale: &Locale) -> Arc<dyn Words> {
        match Mf2Texts::from_packed(&self.dir, locale, &NoDates) {
            Ok(texts) => Arc::new(texts),
            Err(_) => Arc::new(Silent),
        }
    }
}
