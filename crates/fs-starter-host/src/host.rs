//! Running a command line: the words are read by `fs-forms-run` (one reader, no second table of options); the host adds its own
//! name, version and help, the exit codes of the starter (`fs_starter::Exit`) and the line of the parts that are pending.
//!
//! Pattern: Adapter: the command of `fs-forms-run` in, an exit code of the starter out.

use std::process::ExitCode;

use fs_forms_run::{Command, Config, FileLink, LayoutLink, SocketLink, session_locale};
use fs_starter::Exit;

use crate::{package, pending};

/// The help text of the host (the words of the window host follow it).
pub const HELP: &str = "fs-starter (--description PATH | PATH) [--frontend ID] [--style NAME] [--sample-data] [--picture FILE.png]\n\
    Opens the window of a program from its description, without building the program.\n\
    The words below are read by the window host (`fs-forms-run`), the one reader of them.\n";

/// The line `--version` prints.
#[must_use]
pub fn version_line() -> String {
    format!("fs-starter {}", env!("CARGO_PKG_VERSION"))
}

/// Run the words after the program name and return the exit code (0 done, 1 failed, 2 wrong words, 3 contract).
#[must_use]
pub fn run(args: &[String]) -> ExitCode {
    match fs_forms_run::parse(args) {
        Err(message) => {
            eprintln!("fs-starter: {message}\nTry `fs-starter --help`.");
            ExitCode::from(Exit::USAGE.code())
        }
        Ok(Command::Version) => {
            println!("{}", version_line());
            ExitCode::from(Exit::DONE.code())
        }
        Ok(Command::Help) => {
            println!("{HELP}{}", fs_forms_run::HELP);
            ExitCode::from(Exit::DONE.code())
        }
        Ok(Command::Check(c)) => {
            if let Err(code) = announce(&c) {
                return code;
            }
            check(&c)
        }
        Ok(Command::Window(c)) => {
            if let Err(code) = announce(&c) {
                return code;
            }
            fs_forms_run::run(args)
        }
    }
}

/// Say what the path is (a data package: its program, skeleton and layout) and which parts are pending. A package whose
/// `scaffold.toml` cannot be read ends here with the code of a failure, it is never opened as an empty window.
fn announce(c: &Config) -> Result<(), ExitCode> {
    if let Some(path) = &c.description {
        match package::read(path) {
            Ok(Some(p)) => eprintln!("{}", package::line(&p)),
            Ok(None) => {}
            Err(e) => {
                eprintln!("fs-starter: {e}");
                return Err(ExitCode::from(Exit::FAILED.code()));
            }
        }
    }
    eprintln!("{}", pending::note());
    Ok(())
}

/// `--check`: only fetch the description, say whether it is there. Fatal (the contract is not there) is the contract code.
fn check(c: &Config) -> ExitCode {
    let found = match &c.description {
        Some(path) => {
            let lang = session_locale().primary().to_owned();
            FileLink::new(path, lang).resolve(&c.extends, &c.frontend)
        }
        None => SocketLink::open(c.forms_socket.as_deref())
            .and_then(|link| link.resolve(&c.extends, &c.frontend)),
    };
    match found {
        Ok(view) => {
            println!(
                "fs-starter: the description of the frontend {} is there",
                view.frontend
            );
            ExitCode::from(Exit::DONE.code())
        }
        Err(e) => {
            eprintln!("fs-starter: {e}");
            let exit = if e.fatal() {
                Exit::CONTRACT
            } else {
                Exit::FAILED
            };
            ExitCode::from(exit.code())
        }
    }
}
