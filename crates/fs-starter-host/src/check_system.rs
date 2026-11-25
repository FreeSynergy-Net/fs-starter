//! `--check-system [--modules DIR] [--program NAME]... [--description PATH]...`: what is installed and whether it runs (SB-2). A word of
//! the host like `--list-commands`, read before `fs_forms_run::parse`; it draws nothing and runs the same on Linux and Windows.
//!
//! One line per finding, `ok|yellow|red <code> key=value ...`. It uses only the roads the window takes: the registry (bundled and
//! system modules), `ensure_single` + the heartbeat of fs-rpc (`StarterSources::probe`), the MCP switch, and the `--check` match for
//! every program. A service this run started is ended again (one-time command); one that was up before is left alone. A part that is
//! not built is `yellow ... not-there-yet`, never red and never silently green. Exit: 0 (ok and yellow only), 3 (any red), 2 (wrong words).
//!
//! Pattern: Adapter (registry, sources and the `--check` match in, lines and an exit code out).

use std::path::PathBuf;
use std::process::ExitCode;

use fs_forms_run::{Command, FileLink, LayoutLink, session_locale};
use fs_starter::{Exit, SystemPlaces};

use crate::check_sources::{Finding, findings};
use crate::sources::{Limits, Modules, StarterSources};
use crate::{pending, shell};

/// The words of `--check-system`, read.
#[derive(Debug, Default, PartialEq, Eq)]
struct Words {
    modules: Option<PathBuf>,
    programs: Vec<String>,
    descriptions: Vec<String>,
}

fn words(args: &[String]) -> Result<Words, String> {
    let mut w = Words::default();
    let mut it = args.iter().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match flag.as_str() {
            "--modules" if w.modules.is_none() => w.modules = Some(value()?.into()),
            "--program" => w.programs.push(value()?),
            "--description" => w.descriptions.push(value()?),
            _ => return Err(format!("--check-system does not take {flag:?}")),
        }
    }
    Ok(w)
}

/// The lines, counted.
#[derive(Default)]
struct Report {
    red: usize,
    yellow: usize,
}

impl Report {
    fn say(&mut self, level: &str, line: &str) {
        match level {
            "red" => self.red += 1,
            "yellow" => self.yellow += 1,
            _ => {}
        }
        println!("{level} {line}");
    }
}

/// A free text as one value of a line (quoted, so it can never start a second line or a second pair).
fn val(text: &str) -> String {
    format!("{text:?}")
}

/// Run `--check-system` (`args[0]` is the word itself).
pub(crate) fn run(args: &[String]) -> ExitCode {
    let w = match words(args) {
        Ok(w) => w,
        Err(why) => {
            eprintln!("fs-starter: {why}\nTry `fs-starter --help`.");
            return ExitCode::from(Exit::USAGE.code());
        }
    };
    let modules = w
        .modules
        .as_deref()
        .map_or_else(Modules::standard, Modules::standard_with_bundled);
    let sources = StarterSources::new(modules, Limits::default()).ending_what_it_started();
    let mut r = Report::default();
    check_modules(&sources, &mut r);
    check_mcp(&mut r);
    check_pending(&mut r);
    check_programs(&w, &sources, &mut r);
    let level = if r.red > 0 { "red" } else { "ok" };
    println!(
        "{level} check-system-done red={} yellow={}",
        r.red, r.yellow
    );
    ExitCode::from(
        if r.red > 0 {
            Exit::CONTRACT
        } else {
            Exit::DONE
        }
        .code(),
    )
}

/// The registry, then every module: service up, heartbeat, forms read (one `read` line per form, as the window writes it).
fn check_modules(sources: &StarterSources, r: &mut Report) {
    let modules = sources.modules();
    for (name, why) in modules.refused() {
        let why: Vec<String> = why.iter().map(ToString::to_string).collect();
        r.say(
            "red",
            &format!(
                "module-refused module={} why={}",
                val(name),
                val(&why.join("; "))
            ),
        );
    }
    if modules.known().is_empty() && modules.refused().is_empty() {
        r.say("yellow", "no-module-installed");
    }
    for known in modules.known() {
        let began = std::time::Instant::now();
        match sources.probe(&known.name) {
            Ok(read) => {
                r.say(
                    "ok",
                    &format!(
                        "module-up module={} version={} heartbeat=ok ms={}",
                        known.name,
                        known.version,
                        began.elapsed().as_millis()
                    ),
                );
                for line in read {
                    r.say("ok", line.strip_prefix("fs-starter: ").unwrap_or(&line));
                }
            }
            Err(code) => r.say(
                "red",
                &format!(
                    "module-down module={} {}",
                    known.name,
                    crate::sources::line(&code)
                ),
            ),
        }
    }
}

/// MCP must be off (ADR 0097): the host's own switch is asked, the same one `mcp --as <actor>` asks.
fn check_mcp(r: &mut Report) {
    if shell::mcp_enabled() {
        r.say(
            "red",
            "mcp-enabled note=\"MCP must be off after installation\"",
        );
    } else {
        r.say("ok", "mcp-disabled");
    }
}

/// The parts of the starter shell that are not built: yellow, said, never silent and never red.
fn check_pending(r: &mut Report) {
    for part in pending::PENDING {
        r.say(
            "yellow",
            &format!("{} part={part} note=\"not there yet\"", pending::CODE),
        );
    }
    r.say("yellow", crate::update::ACTIVE_NOTE);
}

/// Every program the words name, else every installed data package: the `--check` match, no second start.
fn check_programs(w: &Words, sources: &StarterSources, r: &mut Report) {
    let places = SystemPlaces::standard();
    let mut targets: Vec<(String, Result<String, String>)> = Vec::new();
    for d in &w.descriptions {
        targets.push((d.clone(), Ok(d.clone())));
    }
    let names: Vec<String> = if w.programs.is_empty() && w.descriptions.is_empty() {
        installed_programs()
    } else {
        w.programs.clone()
    };
    for name in names {
        let dir = places
            .program_dir(&name)
            .map(|p| p.to_string_lossy().into_owned())
            .map_err(|why| why.to_string());
        targets.push((name, dir));
    }
    if targets.is_empty() {
        r.say("yellow", "no-program-installed");
    }
    for (name, dir) in targets {
        match dir {
            Err(why) => r.say(
                "red",
                &format!("program-refused program={} why={}", val(&name), val(&why)),
            ),
            Ok(dir) => check_program(&name, &dir, sources, r),
        }
    }
}

/// The names of the folders in the data package place (a place that is not there has none).
fn installed_programs() -> Vec<String> {
    let Some(dir) = fs_starter::programs_dir() else {
        return Vec::new();
    };
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

fn check_program(name: &str, dir: &str, sources: &StarterSources, r: &mut Report) {
    let words = ["--check", "--description", dir].map(String::from);
    let Ok(Command::Check(c)) = fs_forms_run::parse(&words) else {
        r.say(
            "red",
            &format!("program-unreadable program={} why=\"words\"", val(name)),
        );
        return;
    };
    let lang = session_locale().primary().to_owned();
    match FileLink::new(dir, lang).resolve(&c.extends, &c.frontend) {
        Err(e) => r.say(
            "red",
            &format!(
                "program-unreadable program={} why={}",
                val(name),
                val(&e.to_string())
            ),
        ),
        Ok(view) => {
            let mut red = false;
            for f in findings(&view.sources, sources) {
                match f {
                    Finding::Red(l) => {
                        red = true;
                        r.say("red", &format!("program={} {l}", val(name)));
                    }
                    Finding::Yellow(l) => r.say("yellow", &format!("program={} {l}", val(name))),
                    Finding::Checked(m, v, forms, commands) => r.say(
                        "ok",
                        &format!(
                            "source-checked program={} module={m} version={v} forms={forms} commands={commands}",
                            val(name)
                        ),
                    ),
                }
            }
            if !red {
                r.say("ok", &format!("program-checked program={}", val(name)));
            }
        }
    }
}

#[cfg(test)]
#[path = "check_system_tests.rs"]
mod tests;
