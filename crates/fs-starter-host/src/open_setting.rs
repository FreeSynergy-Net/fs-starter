//! The first reader of the settings in the host (E3): `fs.open.when-open` ([`WhenOpen`]) from the settings service of the person.
//! The scaffold's own file `catalog/settings.json` is the catalog (one reader, `CatalogFile` of fs-settings); the values come from
//! the service over the contract `settings` (`ensure_service` + `RpcSettingsStore` of fs-settings-rpc). Whose connection it is, the
//! system says (the peer of the socket); this module sends no name of a person.
//!
//! No service (not installed, does not start, does not answer): the value is the default of the declaration (`ask`) and ONE visible
//! line says so (`yellow`), never a crash and never silence. A lock of the administrator is shown with its reason.
//!
//! The value is only read here. What opens with it (`decide`, the question) follows with O4/O5; until then [`OpenSetting`] is
//! handed next to the start.
//!
//! Pattern: Adapter (`Settings` of fs-settings in, the one value of the host out) + Null object (no service: the default).

use std::path::Path;
use std::sync::Arc;

use fs_rpc::{Expected, SingleTimes};
use fs_settings::{Catalog, CatalogFile, Coded, ProgramName, SettingCode, Settings};
use fs_settings::{NoSystem, SettingKey, Value};
use fs_settings_rpc::{ClientTimes, RpcSettingsStore, ensure_service};
use fs_starter::{StarterSettings, WHEN_OPEN, WhenOpen};

/// What the host knows about `fs.open.when-open` at the start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenSetting {
    /// The effective value (the default `Ask` when the service cannot be reached).
    pub when_open: WhenOpen,
    /// The reason of the administrator's lock, when the value is locked.
    pub locked: Option<String>,
    /// Why the value is only the default (`store-unreachable`, ...), as a code with its parameters.
    pub notice: Option<Coded<SettingCode>>,
}

impl OpenSetting {
    /// The value of the declaration's default, with the reason it is only that.
    #[must_use]
    pub fn unavailable(why: Coded<SettingCode>) -> Self {
        Self {
            when_open: WhenOpen::default(),
            locked: None,
            notice: Some(why),
        }
    }

    /// The line the person sees (stderr), or `None` when there is nothing to say.
    #[must_use]
    pub fn line(&self) -> Option<String> {
        if let Some(why) = &self.notice {
            return Some(format!(
                "fs-starter: yellow {} key=fs.open.when-open value={}",
                why.clone().erase(),
                self.when_open.as_choice()
            ));
        }
        self.locked.as_ref().map(|reason| {
            format!(
                "fs-starter: ok locked key=fs.open.when-open value={} reason={reason}",
                self.when_open.as_choice()
            )
        })
    }
}

/// The catalog of the scaffold's own file (the one `Catalog` the host opens its settings with).
///
/// # Errors
/// `catalog-invalid` (never for the file of this crate; a test proves it).
pub fn catalog() -> Result<Catalog, Coded<SettingCode>> {
    CatalogFile::gather(&[&StarterSettings::file()?])
}

/// The key `fs.open.when-open`.
fn key() -> Result<SettingKey, Coded<SettingCode>> {
    StarterSettings::key(WHEN_OPEN)
}

/// The value out of opened settings: the layers decide (`resolve`: lock, program, person, default), not this code.
#[must_use]
pub fn read(settings: &Settings) -> OpenSetting {
    let found = key().ok().and_then(|k| settings.get(&k));
    let Some(e) = found else {
        let gone = Coded::new(SettingCode::NotDeclared).with_text("key", "fs.open.when-open");
        return OpenSetting::unavailable(gone);
    };
    let when_open = match &e.value {
        Value::Choice(word) => WhenOpen::from_choice(word).unwrap_or_default(),
        _ => WhenOpen::default(),
    };
    let unreachable = settings
        .problems()
        .into_iter()
        .find(|p| p.code() == SettingCode::StoreUnreachable);
    OpenSetting {
        when_open,
        locked: e.lock.map(|l| l.reason),
        notice: unreachable,
    }
}

/// Opens the settings of `program` over the service on `socket` and reads the value. `running` is the file of this program
/// (the service program is looked for beside it, then in `/usr/bin`).
#[must_use]
pub fn connect(
    program: &ProgramName,
    socket: &Path,
    running: &Path,
    times: &SingleTimes,
) -> OpenSetting {
    if let Err(why) = ensure_service(socket, &Expected::new(""), running, times) {
        return OpenSetting::unavailable(why);
    }
    let catalog = match catalog() {
        Ok(c) => c,
        Err(why) => return OpenSetting::unavailable(why),
    };
    let store = Arc::new(RpcSettingsStore::open(
        socket,
        program,
        ClientTimes::default(),
    ));
    read(&Settings::open(
        program.clone(),
        catalog,
        store,
        Arc::new(NoSystem),
    ))
}

/// Reads `fs.open.when-open` for `program` over the settings service of this person and gives the line to say, if there is one.
#[must_use]
pub fn line_for(program: &str) -> Option<String> {
    let name = ProgramName::new(program).ok()?;
    let socket = fs_rpc::default_socket("fs-settings")?;
    let running = std::env::current_exe().ok()?;
    connect(&name, &socket, &running, &SingleTimes::default()).line()
}
