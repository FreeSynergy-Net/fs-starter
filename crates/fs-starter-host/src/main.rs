//! `fs-starter`: the host as a process. Pattern: Facade; everything is in the library.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|w| w.to_string_lossy().into_owned())
        .collect();
    fs_starter_host::run(&args)
}
