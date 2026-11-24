//! The id of a command: derived by the HOST, never taken from the file (SW-5, ENTSCHEID-schnitt section 1.1).
//!
//! The file names a command by its contract key (`capability`, a line of `capabilities.txt`: `card.move`). The host binds
//! `<module>.<key>`, where `<module>` is the folder the file was read from (the identity of the module that supplies it), never a
//! field the module sets itself. An `id` in the file is optional; if it is there it must be the derived one.
//!
//! Pattern: Specification + Factory (small pure functions over text; one place that makes the id).

use fs_components_commands::standard::PREFIX;
use serde_json::Value;

use super::model::Refusal;

/// The id the host binds for the key `key` of the module `module`: ALWAYS `<module>.<key>`. The host never interprets the key: a key
/// that starts with the own module name is taken literally (module `people`, key `people.search` = `people.people.search`). The raw key
/// stays in the item for `invoke` (the service gets the contract key, not the id).
pub(super) fn derive(module: &str, key: &str) -> String {
    format!("{module}.{key}")
}

/// Check the `id` the file may carry against the derived id `derived` of the module `module`.
///
/// No `id`: fine. An `id` that is not text: unreadable. An `id` with the shell's prefix: [`Refusal::ReservedPrefix`]; outside the own
/// name space: [`Refusal::OutsideNamespace`]; inside it but not the derived one: unreadable (`command-id-differs`).
pub(super) fn check_explicit(command: &Value, module: &str, derived: &str) -> Result<(), Refusal> {
    let Some(given) = command.get("id") else {
        return Ok(());
    };
    let given = given
        .as_str()
        .ok_or_else(|| Refusal::Unreadable("command-id".to_owned()))?;
    if given == derived {
        Ok(())
    } else if given.starts_with(PREFIX) {
        Err(Refusal::ReservedPrefix(given.into()))
    } else if !given.starts_with(&format!("{module}.")) {
        Err(Refusal::OutsideNamespace(given.into()))
    } else {
        Err(Refusal::Unreadable("command-id-differs".to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_host_puts_the_module_name_before_every_key_and_never_reads_it() {
        assert_eq!(derive("planning", "card.move"), "planning.card.move");
        // no alias: a key that starts with the own module name is taken literally
        assert_eq!(
            derive("planning", "planning.card.move"),
            "planning.planning.card.move"
        );
        assert_eq!(derive("people", "people.search"), "people.people.search");
        assert_eq!(derive("planning", "tasks.create"), "planning.tasks.create");
        assert_eq!(derive("plan", "planning.x"), "plan.planning.x");
    }
}
