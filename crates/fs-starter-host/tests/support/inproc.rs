//! The host library in THIS test process against the fake module as a service process: the registry reads a made-up `modules/<name>/`
//! folder, and the sources start the fake from it (`ensure_single`), exactly as the window does. The environment (a runtime folder of its
//! own, `FAKE_MODULE`, `FAKE_IDLE_MS`) is set ONCE per test binary, before the first service starts; one binary uses one module name.
//!
//! Pattern: Fixture (a module folder and the sources over it); Unix only (the fake service is).

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::{Once, OnceLock};

use fs_starter_host::registry::SystemPackaged;
use fs_starter_host::sources::{Limits, Modules, StarterSources};
use tempfile::TempDir;

use super::install::fake_module;

static ENV: Once = Once::new();
static RUNTIME: OnceLock<TempDir> = OnceLock::new();

/// `commands.json` of `module` with ONE command of its own name space: `<module>.<capability>` (the key the service gets is the capability).
pub fn commands_of(module: &str, capability: &str) -> String {
    format!(
        r#"{{"module":"{module}","version":"0.1.0","contract":"{module}","scope":"own-contract","commands":[{{"capability":"{capability}","call":"x","access":"write","help":{{"topic":"t","text":{{"en":"e","de":"d"}}}}}}]}}"#
    )
}

/// A bundled module folder with the fake as its service, and the sources of the host over it.
pub struct InProc {
    pub sources: StarterSources,
    /// The socket the service of the module is reached on.
    pub socket: PathBuf,
    _dir: TempDir,
}

/// The module `module` (`forms.json` and `commands.json` as given); the fake ends after `idle_ms` without a connection.
pub fn setup(module: &str, forms: &str, commands: &str, idle_ms: u32) -> InProc {
    let runtime = RUNTIME
        .get_or_init(|| tempfile::Builder::new().prefix("hr").tempdir().unwrap())
        .path()
        .to_owned();
    ENV.call_once(|| {
        // SAFETY: before the first service starts and before anything else of this binary reads the environment; every test of the
        // binary calls `setup` first, and `Once` lets only one of them write.
        unsafe {
            std::env::set_var("XDG_RUNTIME_DIR", &runtime);
            std::env::set_var("FAKE_MODULE", module);
            std::env::set_var("FAKE_IDLE_MS", idle_ms.to_string());
            std::env::remove_var("FAKE_MODE");
        }
    });
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join(module);
    std::fs::create_dir_all(folder.join("catalog")).unwrap();
    std::fs::write(folder.join("catalog/forms.json"), forms).unwrap();
    std::fs::write(folder.join("catalog/commands.json"), commands).unwrap();
    std::fs::copy(
        fake_module(),
        fs_starter::program_file(&folder, fs_starter::SERVICE_STEM),
    )
    .unwrap();
    let modules = Modules::load(
        dir.path(),
        Path::new("/nonexistent"),
        &SystemPackaged::new(),
    );
    assert!(modules.find(module).is_some(), "{:?}", modules.refused());
    InProc {
        sources: StarterSources::new(modules, Limits::default()),
        socket: runtime.join("freesynergy").join(format!("{module}.sock")),
        _dir: dir,
    }
}
