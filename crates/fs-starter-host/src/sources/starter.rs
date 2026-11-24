//! The data sources of the host: the adapter `Sources` of `fs-forms-run` for the modules of the starter (plan `PLAN-quelle-und-abgleich`
//! sections 2 to 4, order SQ-4). The window asks for the contract a description names (`groups@>=0.1.0, <0.2.0`); the host
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

use super::{
    answer::Limits,
    declared, invoke,
    known::{Known, Modules},
    range, service,
};

use fs_forms_bind::ModuleForm;
use fs_forms_run::{Coded, FormData, FormRecords, name_of};

use super::invoke::Written;

/// The sources of the starter.
#[derive(Clone, Debug)]
pub struct StarterSources {
    pub(super) modules: Modules,
    pub(super) limits: Limits,
    /// A one-time command (`--picture`, `--check-system`): a service this run started is ended when the run is done.
    pub(super) end_started: bool,
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
        Self {
            modules,
            limits,
            end_started: false,
        }
    }

    /// For a one-time command: the services this run started itself are ended again after the read (never one that was up before).
    #[must_use]
    pub fn ending_what_it_started(mut self) -> Self {
        self.end_started = true;
        self
    }

    /// The module and its declared forms for `range`, WITHOUT starting a service: what `--check` shows. The same road the window
    /// takes before it connects.
    ///
    /// # Errors
    /// `host-notice-source-unknown` (no such module), `host-notice-data-contract` (version outside the range, unreadable range or
    /// `forms.json` that cannot be read).
    pub fn declared(&self, range: &str) -> Result<(&Known, Vec<ModuleForm>), Coded> {
        // the blank form (`groups >=0.1 <0.2`) has no `@`: its first word still names the module, so the refusal below says how a need is
        // written instead of "unknown source"
        let name = name_of(range).split_whitespace().next().unwrap_or("");
        let contract = |why: &str| {
            Coded::new("host-notice-data-contract")
                .with("contract", name)
                .with("cause", why)
        };
        let Some(known) = self.modules.find(name) else {
            // the folder is there but the registry refused it (a version like `0.1.0`, a bad name ...): say THAT, not "unknown"
            return Err(match self.modules.refusal(name) {
                Some((cause, reason)) => contract(cause).with("reason", reason),
                None => Coded::new("host-notice-source-unknown").with("contract", name),
            });
        };
        match range::fits(range, &known.version) {
            Ok(true) => {}
            Ok(false) => {
                return Err(contract("version").with("version", known.version.to_string()));
            }
            Err(reason) => return Err(contract("range").with("reason", reason)),
        }
        let forms = declared::read(&known.folder).map_err(contract)?;
        Ok((known, forms))
    }

    /// The modules this host knows (what the registry took in, and what it refused).
    #[must_use]
    pub fn modules(&self) -> &Modules {
        &self.modules
    }

    /// Start the service of `module` (the one road of the window: `ensure_single`, version handshake, heartbeat), read every form it
    /// declares, and answer what was read as `read_lines`. A one-time command builds the sources with
    /// [`Self::ending_what_it_started`], so a service this call started is ended again.
    ///
    /// # Errors
    /// `host-notice-source-unknown` (no such module), `host-notice-data-contract` (the forms cannot be read), else the
    /// `host-notice-data-*` codes of the connection.
    pub fn probe(&self, module: &str) -> Result<Vec<String>, Coded> {
        let known = self
            .modules
            .find(module)
            .ok_or_else(|| Coded::new("host-notice-source-unknown").with("contract", module))?;
        let forms = declared::read(&known.folder).map_err(|why| {
            Coded::new("host-notice-data-contract")
                .with("contract", module)
                .with("cause", why)
        })?;
        let began = std::time::Instant::now();
        let records = self.read(known, &forms)?;
        Ok(read_lines(module, &records, began.elapsed()))
    }

    /// Read every form of `known` through the service, with the command set of the module on each form (what `allowed` asks).
    pub(super) fn read(&self, known: &Known, forms: &[ModuleForm]) -> Result<FormRecords, Coded> {
        let mut records = service::fetch(known, forms, &self.limits, self.end_started)?;
        let keys = self.command_keys(&known.name);
        for data in &mut records.forms {
            data.commands.clone_from(&keys);
        }
        Ok(records)
    }

    /// The forms of `module` read again (after an accepted call): what the port `Writer` gives the window to show.
    ///
    /// # Errors
    /// `host-notice-source-unknown`, else the codes of the connection.
    pub(super) fn read_forms(
        &self,
        module: &str,
        forms: &[ModuleForm],
    ) -> Result<Vec<FormData>, Coded> {
        let known = self
            .modules
            .find(module)
            .ok_or_else(|| Coded::new("host-notice-source-unknown").with("contract", module))?;
        Ok(self.read(known, forms)?.forms)
    }

    /// The raw contract keys of the command set of `module` (what a form may bind a command channel to).
    #[must_use]
    pub fn command_keys(&self, module: &str) -> Vec<String> {
        self.modules
            .commands(module)
            .filter_map(|e| e.key.clone())
            .collect()
    }

    /// Run a command of a module on a record. Only a command of the OWN name space of the module that is in its set (`commands.json`)
    /// goes through (border 6); the shared `app.*` commands, another module's and a name the module never listed are refused.
    /// `args` are the arguments by the FIELD names of the module (`args_of` made them); `record` is empty for a command without one.
    ///
    /// # Errors
    /// `host-notice-invoke-foreign` for a command that is not the module's own; the kind of a refusal of the service as its code
    /// (`form-source-bad-arguments` ...); else the codes of the connection.
    pub fn invoke(
        &self,
        module: &str,
        command: &str,
        record: &str,
        args: &[(String, String)],
    ) -> Result<Written, Coded> {
        let known = self
            .modules
            .find(module)
            .ok_or_else(|| Coded::new("host-notice-source-unknown").with("contract", module))?;
        // only a command of the OWN name space that is in the set goes through; the service gets the raw contract key, never the id
        let key = self
            .modules
            .commands(module)
            .find(|e| e.command.id.as_str() == command)
            .and_then(|e| e.key.as_deref());
        let own = command
            .strip_prefix(module)
            .is_some_and(|r| r.starts_with('.'));
        let Some(key) = key.filter(|_| own) else {
            return Err(Coded::new("host-notice-invoke-foreign")
                .with("module", module)
                .with("command", command));
        };
        invoke::call(known, key, record, args, &self.limits, self.end_started)
    }
}

/// What a read gave, one line per form: `fs-starter: read module=M form=F records=N ms=T provider=P` (+ ` empty` when the service answered
/// with no record: the service is there, there is just nothing in it).
#[must_use]
pub fn read_lines(module: &str, records: &FormRecords, took: std::time::Duration) -> Vec<String> {
    if records.forms.is_empty() {
        return vec![format!(
            "fs-starter: read module={module} form=- records=0 ms={} provider={:?} no-form",
            took.as_millis(),
            records.provider
        )];
    }
    records
        .forms
        .iter()
        .map(|f| {
            let n = f.rows.len();
            format!(
                "fs-starter: read module={module} form={} records={n} ms={} provider={:?}{}",
                f.declared.form.name,
                took.as_millis(),
                records.provider,
                if n == 0 { " empty" } else { "" }
            )
        })
        .collect()
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
            .map(|declared| FormData::read_only(declared, Vec::new()))
            .collect(),
        ..FormRecords::default()
    }
}

#[cfg(test)]
#[path = "starter_tests.rs"]
mod read_lines_tests;
