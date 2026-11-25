//! The channels a sample set leaves out of its map: a real module says in its declaration (`map`) which type fills a records channel
//! (`cards` -> `Task`) and which key runs a command channel (`move` -> `card.move`); the sample set leaves both out, and a block locks
//! (`form-channel-unmapped`). The host binds a records channel to the data type of the source and a command channel to its own name,
//! the name `SampleModule` runs.
//!
//! Pattern: none - one function that completes a declaration in place (it wraps no trait, so no Decorator). Card for C2: the set should declare it itself.

use fs_forms_testdata::SampleSet;
use fs_interfaces::forms::{Art, Channel, catalog};

/// For every form of `set`: each records and each command channel of its declaration is bound (see the module).
pub(super) fn commands(set: &mut SampleSet) {
    for declared in &mut set.declared {
        let form = &declared.form;
        let Some(def) = catalog()
            .iter()
            .filter(|f| {
                f.name == form.name
                    && f.version.major == u64::from(form.major)
                    && f.version.minor == u64::from(form.minor)
            })
            .max_by_key(|f| f.version.patch)
        else {
            continue;
        };
        let wanted = |c: &&Channel| {
            c.arts
                .iter()
                .any(|a| matches!(a, Art::Command | Art::Records))
        };
        for channel in def.channels.iter().filter(wanted) {
            if declared.map.iter().any(|(c, _)| *c == channel.name) {
                continue;
            }
            let field = if channel.arts.contains(&Art::Records) {
                declared.data_type.clone()
            } else {
                channel.name.clone()
            };
            declared.map.push((channel.name.clone(), vec![field]));
        }
    }
}
