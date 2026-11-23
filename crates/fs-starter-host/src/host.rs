//! Running a command line: the words are read by `fs-forms-run` (one reader, no second table of options); the host adds its own
//! name, version and help, the exit codes of the starter (`fs_starter::Exit`) and the line of the parts that are pending.
//!
//! Pattern: Adapter: the command of `fs-forms-run` in, an exit code of the starter out.

use std::process::ExitCode;

use fs_forms_run::{Command, Config, FileLink, LayoutLink, SocketLink, session_locale};
use fs_starter::Exit;

use crate::check_sources::check_sources;
use crate::instance::{self, Claim, InstanceTimes};
use crate::program_arg::with_program;
use crate::sources::StarterSources;
use crate::{package, pending, shell};

/// The help text of the host (the words of the window host follow it).
pub const HELP: &str = "fs-starter (--description PATH | --program NAME | PATH) [--frontend ID] [--style NAME] [--sample-data] [--picture FILE.png]\n\
    Opens the window of a program from its description, without building the program.\n\
    --program NAME takes the installed data package /usr/share/freesynergy/programs/NAME (only a place that passes the system check).\n\
fs-starter --list-commands [--modules DIR]\n\
    Lists the commands the host knows: the shared app.* and those of the bundled modules (id, module, version).\n\
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
    if args.first().map(String::as_str) == Some("--list-commands") {
        return list_commands(args);
    }
    let places = fs_starter::SystemPlaces::standard();
    let args = &match with_program(args, &places) {
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
            let sources = StarterSources::standard();
            shell::around_window(&c.frontend, c.resumed, || {
                fs_forms_run::run_code_with(args, &sources)
            })
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
    let (dir, named) = match args {
        [_] => (
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("modules")))
                .unwrap_or_default(),
            false,
        ),
        [_, flag, dir] if flag == "--modules" => (dir.into(), true),
        _ => {
            eprintln!(
                "fs-starter: --list-commands takes only --modules DIR\nTry `fs-starter --help`."
            );
            return ExitCode::from(Exit::USAGE.code());
        }
    };
    let mut registry = crate::registry::Registry::new();
    let mut refused = match registry.load_bundled(&dir) {
        Ok(refused) => refused,
        Err(e) if !named && e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => {
            eprintln!(
                "fs-starter: modules folder {:?}: {e}",
                dir.to_string_lossy()
            );
            return ExitCode::from(Exit::FAILED.code());
        }
    };
    // The modules of the package manager (system folder): only what `SystemPackaged` accepts; a missing folder is fine.
    match registry.load_packages(
        std::path::Path::new(fs_starter::MODULES_DIR),
        &crate::registry::SystemPackaged::new(),
    ) {
        Ok(more) => refused.extend(more),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
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
