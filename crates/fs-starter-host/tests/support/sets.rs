//! Shared by the registry tests: a set file as text, a parsed set, the ids of a registry and a modules folder on disk.

#![allow(dead_code)] // each test file uses a part

use fs_starter_host::registry::{ModuleSet, Registry, parse};

/// The text of a set file with `module` and one command per id (all fields the format requires).
/// The id is written out; the capability is the contract key (the id without `<module>.`; a foreign id stays as it is, so the file
/// carries a foreign id the host refuses).
pub fn json(module: &str, ids: &[&str]) -> String {
    let commands: Vec<String> = ids
        .iter()
        .map(|id| {
            let key = id.strip_prefix(&format!("{module}.")).unwrap_or(id);
            format!(
                r#"{{"id":{id:?},"call":"x","capability":{key:?},"access":"read","help":{{"topic":{:?},"text":{{"en":"e","de":"d"}}}}}}"#,
                id.replace('.', "-")
            )
        })
        .collect();
    format!(
        r#"{{"module":"{module}","version":"0.1.0","contract":"{module}","scope":"own-contract","commands":[{}]}}"#,
        commands.join(",")
    )
}

/// A set with exactly these ids, also ones the host would never derive (a forged set, to test what `add` still refuses): the file is
/// read with bare keys, then the ids are put in by hand.
pub fn set(module: &str, ids: &[&str]) -> ModuleSet {
    let keys: Vec<String> = (0..ids.len()).map(|i| format!("k{i}")).collect();
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    let mut set = parse(&json_keys(module, &keys), module).unwrap();
    for (item, id) in set.items.iter_mut().zip(ids) {
        item.command.id = (*id).into();
    }
    set
}

/// The text of a set file with bare contract keys and NO id (what a module ships since SW-5).
pub fn json_keys(module: &str, keys: &[&str]) -> String {
    let commands: Vec<String> = keys
        .iter()
        .map(|k| {
            format!(
                r#"{{"capability":{k:?},"call":"x","access":"read","help":{{"topic":{:?},"text":{{"en":"e","de":"d"}}}}}}"#,
                k.replace('.', "-")
            )
        })
        .collect();
    format!(
        r#"{{"module":"{module}","version":"0.1.0","contract":"{module}","scope":"own-contract","commands":[{}]}}"#,
        commands.join(",")
    )
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
    json("capture", &["capture.task.create"])
}
