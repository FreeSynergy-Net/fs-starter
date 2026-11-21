//! Running a command line: the words are read by `fs-forms-run` (one reader, no second table of options); the host adds its own
//! name, version and help, the exit codes of the starter (`fs_starter::Exit`) and the line of the parts that are pending.
//!
//! Pattern: Adapter: the command of `fs-forms-run` in, an exit code of the starter out.

use std::process::ExitCode;

use fs_forms_run::{Command, Config, FileLink, LayoutLink, SocketLink, session_locale};
use fs_starter::Exit;

use crate::instance::{self, Claim, InstanceTimes};
use crate::{package, pending, shell};

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
    // `mcp --as <actor>` is a word of the starter library, not of the window host: it ends in `mcp-disabled` (MCP is off).
    if args.first().map(String::as_str) == Some("mcp") {
        let words: Vec<std::ffi::OsString> = args.iter().map(std::ffi::OsString::from).collect();
        return shell::run_library(&words, &mut std::io::stdout().lock());
    }
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
            let program = match announce(&c) {
                Ok(program) => program,
                Err(code) => return code,
            };
            // held until the window is closed; a second start of the same program ends here (`HandedOver`)
            let _place = match single(program.as_deref(), args, c.picture.is_some()) {
                Ok(place) => place,
                Err(code) => return code,
            };
            shell::around_window(&c.frontend, c.resumed, || fs_forms_run::run(args))
        }
    }
}

/// Say what the path is (a data package: its program, skeleton and layout) and which parts are pending. A package whose
/// `scaffold.toml` cannot be read ends here with the code of a failure, it is never opened as an empty window. Answers the
/// program name of a package (`None`: another kind of description, which has no program of its own).
fn announce(c: &Config) -> Result<Option<String>, ExitCode> {
    let mut program = None;
    if let Some(path) = &c.description {
        match package::read(path) {
            Ok(Some(p)) => {
                eprintln!("{}", package::line(&p));
                program = Some(p.program);
            }
            Ok(None) => {}
            Err(e) => {
                eprintln!("fs-starter: {e}");
                return Err(ExitCode::from(Exit::FAILED.code()));
            }
        }
    }
    eprintln!("{}", pending::note());
    Ok(program)
}

/// The single instance of the program: `Ok(Some(guard))` the first window (hold it), `Ok(None)` nothing to hold (no program, no
/// endpoint, or a picture, which has no window to protect), `Err(code)` this start ends (handed over: success; else a failure).
/// A start that draws a picture takes the place only to learn who has it: a running window gets the words, else it lets go at once.
fn single(
    program: Option<&str>,
    args: &[String],
    picture: bool,
) -> Result<Option<instance::Guard>, ExitCode> {
    let Some(name) = program else {
        return Ok(None);
    };
    let failed = |line: &str| {
        eprintln!("fs-starter: {line}");
        ExitCode::from(Exit::FAILED.code())
    };
    let endpoint = instance::endpoint_for(name).map_err(|e| failed(&e))?;
    match instance::claim(endpoint.as_deref(), args, InstanceTimes::default()) {
        Ok(Claim::First(guard)) => Ok((!picture).then_some(guard)),
        Ok(Claim::HandedOver) => {
            eprintln!("fs-starter: {}", instance::HANDED_OVER);
            Err(ExitCode::from(Exit::DONE.code()))
        }
        Ok(Claim::Off(code)) => {
            eprintln!("fs-starter: {code}");
            Ok(None)
        }
        Err(e) => Err(failed(&e)),
    }
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
