//! The id of a command: derived by the HOST, never taken from the file (SW-5, ENTSCHEID-schnitt section 1.1).
//!
//! The file names a command by its contract key (`capability`, a line of `capabilities.txt`: `card.move`). The host binds
//! `<module>.<key>`, where `<module>` is the folder the file was read from (the identity of the module that supplies it), never a
//! field the module sets itself. A key that already starts with the own prefix is the same command written out (`planning.card.move`
//! = `card.move`). An `id` in the file is optional; if it is there it must be the derived one.
//!
//! Pattern: Specification + Factory (small pure functions over text; one place that makes the id).

use fs_components_commands::standard::PREFIX;
use serde_json::Value;

use super::model::Refusal;

/// The id the host binds for the key `key` of the module `module`: the key under the own prefix (the own prefix is not doubled).
pub(super) fn derive(module: &str, key: &str) -> String {
    let space = format!("{module}.");
    if key.starts_with(&space) {
        key.to_owned()
    } else {
        format!("{space}{key}")
    }
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
    fn the_host_puts_the_own_prefix_before_a_key_and_never_doubles_it() {
        assert_eq!(derive("planning", "card.move"), "planning.card.move");
        assert_eq!(
            derive("planning", "planning.card.move"),
            "planning.card.move"
        );
        // a key that looks like another module's id stays INSIDE the own name space
        assert_eq!(derive("planning", "tasks.create"), "planning.tasks.create");
        assert_eq!(derive("plan", "planning.x"), "plan.planning.x");
    }
}
