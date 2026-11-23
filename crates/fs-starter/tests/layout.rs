//! `program_file` and friends: the one place that makes the name of a program file. Runs on every system; the Windows ending is passed
//! explicitly so Linux checks the Windows form too.

use std::path::Path;

use fs_starter::{Launcher, ProcessLauncher, program_file, program_file_with};

#[test]
fn the_program_file_has_the_ending_of_the_system() {
    let d = Path::new("dir");
    assert_eq!(
        program_file_with(d, "service", ".exe"),
        d.join("service.exe")
    );
    assert_eq!(program_file_with(d, "service", ""), d.join("service"));
    // a dot in the stem is not an ending
    assert_eq!(
        program_file_with(d, "fs.tasks", ".exe"),
        d.join("fs.tasks.exe")
    );
    assert_eq!(
        program_file(d, "x"),
        d.join(format!("x{}", std::env::consts::EXE_SUFFIX))
    );
}

#[test]
fn an_exec_word_with_the_ending_is_refused_where_there_is_an_ending() {
    assert!(fs_starter::has_suffix("tool.exe", ".exe"));
    assert!(fs_starter::has_suffix("TOOL.EXE", ".exe"));
    assert!(!fs_starter::has_suffix("tool", ".exe"));
    assert!(
        !fs_starter::has_suffix("tool.exe", ""),
        "no ending, nothing to refuse"
    );
    // the launcher finds the bare name, and not a word that already has the ending of this system
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(program_file(tmp.path(), "tool"), "x").unwrap();
    let l = ProcessLauncher::new(vec![tmp.path().to_owned()]);
    assert_eq!(l.find("tool"), Some(program_file(tmp.path(), "tool")));
}

#[test]
fn a_word_with_the_ending_of_this_system_is_not_found() {
    if std::env::consts::EXE_SUFFIX.is_empty() {
        return; // Unix: no ending exists to refuse
    }
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(program_file(tmp.path(), "tool"), "x").unwrap();
    let l = ProcessLauncher::new(vec![tmp.path().to_owned()]);
    assert_eq!(l.find("tool.exe"), None);
}
