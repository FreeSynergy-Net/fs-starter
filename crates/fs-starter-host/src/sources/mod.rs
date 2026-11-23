//! The data sources of the host: the adapter `Sources` of `fs-forms-run` for the modules of the starter (plan `PLAN-quelle-und-abgleich`
//! sections 2 to 4, order SQ-4). The window asks for the contract a description names (`groups >=0.1 <0.2`); the host
//!
//! 1. looks the module up in the registry (bundled, or loaded only through the verifier `SystemPackaged`; border 1),
//! 2. compares the contract and the version of the module with the range (border 2: `host-notice-data-contract`, never silent),
//! 3. reads `catalog/forms.json` of the module (what forms it offers, which field fills which channel),
//! 4. starts and connects the service from the module folder only (border 3), on the socket `default_socket(module)` (border 4),
//! 5. reads the pages, with the answers treated as foreign data (border 5), and
//! 6. runs a command only in the own name space of the module (border 6, [`StarterSources::invoke`]).
//!
//! A service that says `provider = "fake"` gives records the window shows as "Sample data" (border 7, done by `FormRecords`).
//!
//! Pattern: Adapter (the port `Sources` of fs-forms-run, the contract `FormSource` of fs-interfaces) + Facade over four small parts.

pub mod answer;
pub mod declared;
mod known;
pub mod range;
mod service;

use std::path::Path;
use std::sync::Arc;

use fs_forms_bind::ModuleForm;
use fs_forms_run::{Coded, FormData, FormRecords, Records, Sources, name_of};

pub use answer::Limits;
pub use known::{Known, Modules};

/// The sources of the starter.
#[derive(Clone, Debug)]
pub struct StarterSources {
    modules: Modules,
    limits: Limits,
}

impl StarterSources {
    /// The modules of this binary and of the system folder, the standard borders.
    #[must_use]
    pub fn standard() -> Self {
        Self::new(Modules::standard(), Limits::default())
    }

    /// Given modules and borders (tests).
    #[must_use]
    pub fn new(modules: Modules, limits: Limits) -> Self {
        Self { modules, limits }
    }

    /// The module and its declared forms for `range`, WITHOUT starting a service: what `--check` shows. The same road the window
    /// takes before it connects.
    ///
    /// # Errors
    /// `host-notice-source-unknown` (no such module), `host-notice-data-contract` (version outside the range, unreadable range or
    /// `forms.json` that cannot be read).
    pub fn declared(&self, range: &str) -> Result<(&Known, Vec<ModuleForm>), Coded> {
        let name = name_of(range);
        let known = self
            .modules
            .find(name)
            .ok_or_else(|| Coded::new("host-notice-source-unknown").with("contract", name))?;
        let contract = |why: &str| {
            Coded::new("host-notice-data-contract")
                .with("contract", name)
                .with("cause", why)
        };
        match range::fits(range, known.version) {
            Some(true) => {}
            Some(false) => {
                return Err(contract("version").with(
                    "version",
                    format!("{}.{}", known.version.0, known.version.1),
                ));
            }
            None => return Err(contract("range")),
        }
        let forms = declared::read(&known.folder).map_err(contract)?;
        Ok((known, forms))
    }

    /// Run a command of a module on a record. Only a command of the OWN name space of the module that is in its set (`commands.json`)
    /// goes through (border 6); the shared `app.*` commands, another module's and a name the module never listed are refused.
    ///
    /// # Errors
    /// `host-notice-invoke-foreign` for a command that is not the module's own, else the codes of the connection.
    pub fn invoke(&self, module: &str, command: &str, record: &str) -> Result<(), Coded> {
        let known = self
            .modules
            .find(module)
            .ok_or_else(|| Coded::new("host-notice-source-unknown").with("contract", module))?;
        let own = command
            .strip_prefix(module)
            .is_some_and(|r| r.starts_with('.'))
            && self
                .modules
                .commands(module)
                .any(|e| e.command.id.as_str() == command);
        if !own {
            return Err(Coded::new("host-notice-invoke-foreign")
                .with("module", module)
                .with("command", command));
        }
        service::call(known, command, record, &self.limits)
    }
}

impl Sources for StarterSources {
    /// The `socket` of the command line is NOT used: a module is reached only on its own socket (border 4).
    fn connect(&self, range: &str, _: Option<&Path>, _: &Path) -> Result<Arc<dyn Records>, Coded> {
        // a source that is not there is said on stderr too (the window shows it as a notice): never silent
        let said = |code: Coded| {
            eprintln!("fs-starter: {}", line(&code));
            code
        };
        let (known, forms) = self.declared(range).map_err(said)?;
        let records = service::fetch(known, &forms, &self.limits).map_err(said)?;
        Ok(Arc::new(records))
    }
}

/// A code with its parameters as one line (`code key=value ...`).
#[must_use]
pub fn line(c: &Coded) -> String {
    let params: Vec<String> = c.params.iter().map(|(k, v)| format!("{k}={v}")).collect();
    format!("{} {}", c.code, params.join(" "))
        .trim_end()
        .to_owned()
}

/// The declared forms as records without rows: what `form_report` matches (no service needed).
#[must_use]
pub fn bare(forms: Vec<ModuleForm>) -> FormRecords {
    FormRecords {
        provider: String::new(),
        forms: forms
            .into_iter()
            .map(|declared| FormData {
                declared,
                rows: Vec::new(),
            })
            .collect(),
    }
}
