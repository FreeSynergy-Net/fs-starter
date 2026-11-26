//! `--check` of the host for the sources a description names: the match of the forms, without a service.
//!
//! Pattern: Adapter (the registry and `form_report` of fs-forms in, lines and an exit code out).

use std::process::ExitCode;

use fs_forms_run::{Coded, FormData, command_key};
use fs_starter::Exit;

use crate::sources::{StarterSources, bare, types, words};

/// One finding of the match: what `--check` prints and what `--check-system` turns into its own lines.
pub(crate) enum Finding {
    /// A mismatch (the contract code 3).
    Red(String),
    /// Said, and the exit code stays 0.
    Yellow(String),
    /// A source that was checked: `(module, version, forms, commands)`.
    Checked(String, String, usize, usize),
}

/// The match of every source the description names, WITHOUT starting a service: the module is looked up in the registry, its
/// contract and version are compared, its `forms.json` goes through the one match of fs-forms (`form_report`). Red (a form that does not
/// fit, a version outside the range) ends with the contract code 3; yellow (a form no component draws, a module that is not there)
/// is said and ends with 0.
pub(crate) fn check_sources(ranges: &[String], sources: &StarterSources) -> ExitCode {
    let mut red = false;
    for f in findings(ranges, sources) {
        match f {
            Finding::Red(l) => {
                red = true;
                println!("fs-starter: red {l}");
            }
            Finding::Yellow(l) => println!("fs-starter: yellow {l}"),
            Finding::Checked(name, version, forms, commands) => println!(
                "fs-starter: source {name} {version} checked forms={forms} commands={commands} (no service started)"
            ),
        }
    }
    ExitCode::from(if red { Exit::CONTRACT } else { Exit::DONE }.code())
}

/// The findings of the match for every range, in the order they are found.
pub(crate) fn findings(ranges: &[String], sources: &StarterSources) -> Vec<Finding> {
    let mut out = Vec::new();
    for range in ranges {
        match sources.declared(range) {
            Err(code) => {
                // a module that is not there is said (the window shows "unknown source"); a module that does not fit is red
                push(&mut out, code.code != "host-notice-source-unknown", &code);
            }
            Ok((known, forms)) => {
                let mut records = bare(forms);
                let keys = sources.command_keys(&known.name);
                for data in &mut records.forms {
                    data.commands.clone_from(&keys);
                }
                for f in fs_forms_run::form_report(&records) {
                    push(&mut out, f.red, &f.code);
                }
                // a command channel must be bound to a key of the command set of the module (`task.steal`, `planning.task.create`:
                // not in it, red)
                for c in unknown_commands(&records.forms, &keys) {
                    push(&mut out, true, &c);
                }
                // the words of the heads come from the type catalog of the module: none is said, a broken one is red with its cause
                match types::read(&known.folder) {
                    Ok(Some(_)) => match words::check(&known.folder) {
                        Ok(true) => {}
                        // heads would be the bare word ids of the type catalog (`person.emails`)
                        Ok(false) => out.push(Finding::Yellow(format!(
                            "no-module-texts module={}",
                            known.name
                        ))),
                        Err(why) => out.push(Finding::Red(format!(
                            "module-texts-broken module={} cause={why:?}",
                            known.name
                        ))),
                    },
                    Ok(None) => out.push(Finding::Yellow(format!(
                        "no-type-catalog module={}",
                        known.name
                    ))),
                    Err(b) => out.push(Finding::Red(format!(
                        "type-catalog-broken module={} cause={} reason={:?}",
                        known.name, b.cause, b.reason
                    ))),
                }
                out.push(Finding::Checked(
                    known.name.clone(),
                    known.version.to_string(),
                    records.forms.len(),
                    keys.len(),
                ));
            }
        }
    }
    out
}

fn push(out: &mut Vec<Finding>, bad: bool, c: &Coded) {
    let l = crate::sources::line(c);
    out.push(if bad {
        Finding::Red(l)
    } else {
        Finding::Yellow(l)
    });
}

/// The command channels of the forms bound to a key that is not in `keys` (the command set of the module): `form-command-unknown`.
fn unknown_commands(forms: &[FormData], keys: &[String]) -> Vec<Coded> {
    let mut out = Vec::new();
    for data in forms {
        for (channel, _) in &data.declared.map {
            if let Some(key) = command_key(data, channel).filter(|k| !keys.iter().any(|x| x == k)) {
                out.push(
                    Coded::new("form-command-unknown")
                        .with("form", data.declared.form.name.clone())
                        .with("channel", channel.clone())
                        .with("command", key),
                );
            }
        }
    }
    out
}
