//! Shared by the registry tests: a set file as text, a parsed set, the ids of a registry and a modules folder on disk.

#![allow(dead_code)] // each test file uses a part

use fs_starter_host::registry::{ModuleSet, Registry, parse};

/// The text of a set file with `module` and one command per id (all fields the format requires).
pub fn json(module: &str, ids: &[&str]) -> String {
    let commands: Vec<String> = ids
        .iter()
        .map(|id| {
            format!(
                r#"{{"id":{id:?},"call":"x","capability":"{module}.cap","access":"read","help":{{"topic":{:?},"text":{{"en":"e","de":"d"}}}}}}"#,
                id.replace('.', "-")
            )
        })
        .collect();
    format!(
        r#"{{"module":"{module}","version":"0.1.0","contract":"{module}","scope":"own-contract","commands":[{}]}}"#,
        commands.join(",")
    )
}

pub fn set(module: &str, ids: &[&str]) -> ModuleSet {
    parse(&json(module, ids), module).unwrap()
}

pub fn ids(r: &Registry) -> Vec<String> {
    r.entries()
        .iter()
        .map(|e| e.command.id.as_str().to_owned())
        .collect()
}

pub fn modules_dir(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (module, body) in files {
        let catalog = dir.path().join(module).join("catalog");
        std::fs::create_dir_all(&catalog).unwrap();
        std::fs::write(catalog.join("commands.json"), body).unwrap();
    }
    dir
}

pub fn capture() -> String {
    json("capture", &["capture.shot"])
}
