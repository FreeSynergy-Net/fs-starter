//! The words a module brings: `<module>/catalog/texts/` (`manifest.mf2m` + `<tag>.<hash>.mf2b`, packed by `fs-blueprint package module`),
//! loaded by `Mf2Texts::from_packed`. The window asks them after its own words (`ctx.say`: the chain is made by fs-forms-run).
//!
//! Pattern: Port adapter + Factory Method (`ModuleWords::in_locale` makes the words of a language from the folder of the package).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use fs_components_mf2::{DateDir, Mf2Texts};
use fs_components_words::{Args, Locale, Said, TextId, Words};
use fs_forms_run::ModuleWords;

/// The packed texts of one module.
#[derive(Debug)]
pub struct PackedWords {
    dir: PathBuf,
    dates: DateDir,
}

/// No sentence at all: every key is missing, so the next in the chain answers.
struct Silent;

impl Words for Silent {
    fn say(&self, id: &TextId, _: &Args) -> Said {
        Said::missing(id)
    }
}

impl PackedWords {
    /// The words of the module in `folder`, or `None` when it has no `catalog/texts/` (no error) or they cannot be loaded (said on
    /// stderr, never silent: the window then shows the field names).
    #[must_use]
    pub fn of(folder: &Path, module: &str) -> Option<Arc<dyn ModuleWords>> {
        let dir = folder.join("catalog").join("texts");
        if !dir.is_dir() {
            return None;
        }
        let words = Self {
            dir,
            dates: DateDir::new(fs_starter::SHARE_DIR),
        };
        match Mf2Texts::from_packed(&words.dir, &Locale::english(), &words.dates) {
            Ok(_) => Some(Arc::new(words)),
            Err(e) => {
                eprintln!("fs-starter: module-texts-unreadable module={module} why={e:?}");
                None
            }
        }
    }
}

impl ModuleWords for PackedWords {
    fn in_locale(&self, locale: &Locale) -> Arc<dyn Words> {
        match Mf2Texts::from_packed(&self.dir, locale, &self.dates) {
            Ok(texts) => Arc::new(texts),
            Err(_) => Arc::new(Silent),
        }
    }
}
