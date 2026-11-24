//! The way of a gesture to the module (SW-5): the port `Writer` of `fs-forms-run` on the service of one module. A gesture comes as ONE
//! call (contract key, record, arguments by field name, made by `args_of`); the host only turns the key into the id of its own name space
//! (the border of [`StarterSources::invoke`] stays the same), sends it, and after an accepted call reads the forms again.
//!
//! Pattern: Adapter - the port `fs_forms_run::Writer` on `FormSource.invoke` (args, problem, result).

use fs_forms_bind::ModuleForm;
use fs_forms_run::{Coded, FormData, Invocation, Writer};

use super::starter::StarterSources;

/// The connection of the window to one module: what a gesture writes and what the form shows afterwards.
#[derive(Debug)]
pub(super) struct ModuleWriter {
    sources: StarterSources,
    module: String,
    forms: Vec<ModuleForm>,
}

impl ModuleWriter {
    pub(super) fn new(sources: StarterSources, module: &str, forms: Vec<ModuleForm>) -> Self {
        Self {
            sources,
            module: module.to_owned(),
            forms,
        }
    }
}

impl Writer for ModuleWriter {
    fn invoke(&self, call: &Invocation) -> Result<(), Coded> {
        // the key of the contract (`card.move`) lives in the name space of the module: the id the host bound for it
        let id = format!("{}.{}", self.module, call.command);
        let record = call.record.as_deref().unwrap_or("");
        self.sources
            .invoke(&self.module, &id, record, &call.args)
            .map(|_| ())
    }

    fn reload(&self) -> Result<Vec<FormData>, Coded> {
        self.sources.read_forms(&self.module, &self.forms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::SystemPackaged;
    use crate::sources::{Limits, Modules};
    use std::path::Path;

    /// The module `groups` with the one command `task.create` (file `commands.json`, as the registry reads it).
    fn writer() -> (tempfile::TempDir, ModuleWriter) {
        let b = tempfile::tempdir().unwrap();
        let catalog = b.path().join("groups").join("catalog");
        std::fs::create_dir_all(&catalog).unwrap();
        std::fs::write(
            catalog.join("commands.json"),
            r#"{"module":"groups","version":"0.1.0","contract":"groups","scope":"own-contract","commands":[{"id":"groups.task.create","call":"x","capability":"task.create","access":"write","help":{"topic":"add","text":{"en":"e","de":"d"}}}]}"#,
        )
        .unwrap();
        let m = Modules::load(b.path(), Path::new("/nonexistent"), &SystemPackaged::new());
        let w = ModuleWriter::new(
            StarterSources::new(m, Limits::default()),
            "groups",
            Vec::new(),
        );
        (b, w)
    }

    fn call(command: &str) -> Invocation {
        Invocation {
            form: "tree".into(),
            command: command.into(),
            record: Some("r1".into()),
            args: vec![("name".into(), "x".into())],
        }
    }

    #[test]
    fn a_key_of_the_command_set_is_bound_to_the_name_space_and_any_other_key_is_foreign() {
        let (_b, w) = writer();
        // `task.create` is `groups.task.create`: it passes the border (and then fails: this module has no program file)
        assert_eq!(
            w.invoke(&call("task.create")).unwrap_err().code,
            "host-notice-data-unreachable"
        );
        assert_eq!(
            w.invoke(&call("task.delete")).unwrap_err().code,
            "host-notice-invoke-foreign"
        );
        // a key that already carries the module is no alias: `groups.task.create` is `groups.groups.task.create`
        assert_eq!(
            w.invoke(&call("groups.task.create")).unwrap_err().code,
            "host-notice-invoke-foreign"
        );
    }

    #[test]
    fn the_forms_are_read_again_through_the_same_module_and_a_missing_module_is_unknown() {
        let (_b, w) = writer();
        // the module is known (its service is asked, and this one has no program file); another name is unknown
        let (_b2, mut other) = writer();
        other.module = "tasks".into();
        assert_eq!(
            other.reload().unwrap_err().code,
            "host-notice-source-unknown"
        );
        assert_eq!(w.reload().unwrap_err().code, "host-notice-data-unreachable");
    }
}
