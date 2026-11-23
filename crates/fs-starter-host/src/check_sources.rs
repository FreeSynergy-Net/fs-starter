//! `--check` of the host for the sources a description names: the match of the forms, without a service.
//!
//! Pattern: Adapter (the registry and `form_report` of fs-forms in, lines and an exit code out).

use std::process::ExitCode;

use fs_starter::Exit;

use crate::sources::{StarterSources, bare};

/// The match of every source the description names, WITHOUT starting a service: the module is looked up in the registry, its
/// contract and version are compared, its `forms.json` goes through the one match of fs-forms (`form_report`). Red (a form that does not
/// fit, a version outside the range) ends with the contract code 3; yellow (a form no component draws, a module that is not there)
/// is said and ends with 0.
pub(crate) fn check_sources(ranges: &[String], sources: &StarterSources) -> ExitCode {
    let mut red = false;
    for range in ranges {
        match sources.declared(range) {
            Err(code) => {
                // a module that is not there is said (the window shows "unknown source"); a module that does not fit is red
                let bad = code.code != "host-notice-source-unknown";
                red |= bad;
                println!("fs-starter: {}", line(&code, bad));
            }
            Ok((known, forms)) => {
                for f in fs_forms_run::form_report(&bare(forms)) {
                    red |= f.red;
                    println!("fs-starter: {}", line(&f.code, f.red));
                }
                println!(
                    "fs-starter: source {} {} checked",
                    known.name, known.version
                );
            }
        }
    }
    ExitCode::from(if red { Exit::CONTRACT } else { Exit::DONE }.code())
}

/// A finding as one line: `red|yellow code key=value ...`.
fn line(c: &fs_forms_run::Coded, red: bool) -> String {
    format!(
        "{} {}",
        if red { "red" } else { "yellow" },
        crate::sources::line(c)
    )
}
