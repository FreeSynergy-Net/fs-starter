//! Running a command line: the words are read by `fs-forms-run` (one reader, no second table of options); the host adds its own
//! name, version and help, the exit codes of the starter (`fs_starter::Exit`) and the line of the parts that are pending.
//!
//! Pattern: Adapter: the command of `fs-forms-run` in, an exit code of the starter out.

use std::path::PathBuf;
use std::process::ExitCode;

use fs_forms_run::{Command, Config, FileLink, LayoutLink, SocketLink, session_locale};
use fs_starter::Exit;

use crate::check_sources::check_sources;
use crate::program_arg::with_program;
use crate::slots;
use crate::sources::StarterSources;
use crate::{package, pending, shell};

/// The help text of the host (the words of the window host follow it).
pub const HELP: &str = "fs-starter (--description PATH | --program NAME | PATH) [--frontend ID] [--style NAME] [--sample-data] [--new-instance] [--picture FILE.png [--steps FILE]]\n\
    Opens the window of a program from its description, without building the program.\n\
    --steps FILE (only with --picture) plays the steps of FILE before the picture is taken.\n\
    --program NAME takes the installed data package /usr/share/freesynergy/programs/NAME (only a place that passes the system check).\n\
    --new-instance opens one more window of the program next to the running ones (window-2 .. window-9); without it a second start hands its words to the first window. With --picture it takes no place.\n\
fs-starter --list-commands [--modules DIR]\n\
    Lists the commands the host knows: the shared app.* and those of the bundled modules (id, module, version).\n\
fs-starter --check-system [--modules DIR] [--program NAME]... [--description PATH]...\n\
    Checks what is installed and whether it runs: one line per finding, `ok|yellow|red code key=value`; exit 0 (ok, yellow) or 3 (red).\n\
    Starts each module service (and ends it again if it started it), asks its heartbeat, checks that MCP is off.\n\
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
    if args.first().map(String::as_str) == Some("--check-system") {
        return crate::check_system::run(args);
    }
    if args.first().map(String::as_str) == Some("--list-commands") {
        return list_commands(args);
    }
    let (args, new_window) = slots::strip(args);
    let places = fs_starter::SystemPlaces::standard();
    let args = &match with_program(&args, &places) {
        Ok(args) => args,
        Err(line) => {
            eprintln!("fs-starter: {line}");
            return ExitCode::from(Exit::FAILED.code());
        }
    };
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
        // the list of blocks this build does not draw: the line of fs-forms-run itself (no copy here)
        Ok(Command::ListBlocks) => {
            ExitCode::from(Exit::of_frontend(fs_forms_run::run_code(args)).code())
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
            let (_place, slot) =
                match slots::single(program.as_deref(), args, c.picture.is_some(), new_window) {
                    Ok(place) => place,
                    Err(code) => return code,
                };
            // E3: the first reader of the settings. The value waits next to the start until `decide` (O4) takes it; without the service the
            // default `ask` holds and one line says so. A one-time `--picture` starts no service.
            if c.picture.is_none()
                && let Some(line) = program.as_deref().and_then(crate::open_setting::line_for)
            {
                eprintln!("{line}");
            }
            let sources = StarterSources::standard();
            let sources = match program.as_deref() {
                Some(name) => sources.named(name),
                None => sources,
            };
            let sources = sources.slot(slot);
            let sources = if c.picture.is_some() {
                // `--picture` is a one-time command: a service it started itself is ended again
                sources.ending_what_it_started()
            } else {
                sources
            };
            shell::around_window(&c.frontend, c.resumed, || {
                fs_forms_run::run_code_with(args, &sources)
            })
        }
        // `Command` grows (`#[non_exhaustive]`): a command this host has no code for is said, never ignored
        Ok(other) => not_built(&other),
    }
}

/// The exit code of a command of `fs-forms-run` this host does not build: said on stderr (never silent), exit `Exit::USAGE`.
fn not_built(command: &impl std::fmt::Debug) -> ExitCode {
    eprintln!("{}", not_built_line(command));
    ExitCode::from(NOT_BUILT.code())
}

/// The exit code of a command this host does not build: the one of wrong words (the person asked for what this host cannot do).
const NOT_BUILT: Exit = Exit::USAGE;

/// The line that says it.
fn not_built_line(command: &impl std::fmt::Debug) -> String {
    format!(
        "fs-starter: command-not-built: {command:?} is not built in this host\nTry `fs-starter --help`."
    )
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
            check_sources(&view.sources, &StarterSources::standard())
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

/// `--list-commands [--modules DIR]`: the registry (`app.*` plus the bundled modules' sets) as lines, one per command. The folder
/// of the bundled modules is `--modules DIR`, else `modules/` next to the binary. A set that is refused is said on stderr and left
/// out; the exit code is then 1.
fn list_commands(args: &[String]) -> ExitCode {
    // Only the default folder next to the binary may be missing; a folder that was named must be there (never silent).
    let system = fs_starter::system_modules_dir();
    let (dir, named): (Option<PathBuf>, bool) = match args {
        // no folder next to the binary is `None` (no empty path stands for it); where it is the system folder too (Windows install) it
        // is read once, as a package
        [_] => (
            fs_starter::bundled_modules_dir().filter(|b| {
                !system
                    .as_deref()
                    .is_some_and(|s| crate::registry::is_same_folder(b, s))
            }),
            false,
        ),
        [_, flag, dir] if flag == "--modules" => (Some(dir.into()), true),
        _ => {
            eprintln!(
                "fs-starter: --list-commands takes only --modules DIR\nTry `fs-starter --help`."
            );
            return ExitCode::from(Exit::USAGE.code());
        }
    };
    let mut registry = crate::registry::Registry::new();
    let mut refused = match dir.as_deref().map(|d| registry.load_bundled(d)) {
        None => Vec::new(),
        Some(Ok(refused)) => refused,
        Some(Err(e)) if !named && e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Some(Err(e)) => {
            eprintln!(
                "fs-starter: modules folder {:?}: {e}",
                dir.unwrap_or_default().to_string_lossy()
            );
            return ExitCode::from(Exit::FAILED.code());
        }
    };
    // The modules of the package manager (system folder): only what `SystemPackaged` accepts; a missing folder is fine.
    let more = system
        .as_deref()
        .map(|s| registry.load_packages(s, &crate::registry::SystemPackaged::new()));
    match more {
        None => {}
        Some(Ok(more)) => refused.extend(more),
        Some(Err(e)) if e.kind() == std::io::ErrorKind::NotFound => {}
        Some(Err(e)) => {
            eprintln!("fs-starter: system modules folder: {e}");
            return ExitCode::from(Exit::FAILED.code());
        }
    }
    for line in registry.lines() {
        println!("{line}");
    }
    for (module, why) in &refused {
        for r in why {
            eprintln!("fs-starter: module {module:?}: {r}");
        }
    }
    ExitCode::from(
        if refused.is_empty() {
            Exit::DONE
        } else {
            Exit::FAILED
        }
        .code(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_this_host_does_not_build_is_said_and_ends_with_the_code_of_wrong_words() {
        // the arm `Ok(other)` of `run`: `Command` grows (`#[non_exhaustive]`), a new command is never ignored or a panic
        let line = not_built_line(&"Frobnicate");
        assert!(
            line.starts_with("fs-starter: command-not-built: "),
            "{line}"
        );
        assert!(
            line.contains("Frobnicate") && line.contains("--help"),
            "{line}"
        );
        assert_eq!(NOT_BUILT.code(), Exit::USAGE.code());
        assert_ne!(NOT_BUILT.code(), Exit::DONE.code());
    }
}
