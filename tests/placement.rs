//! Condition A4 of the fs-conformance review: `fs-conformance` never lands in the normal graph of `fs-starter`.
//!
//! It reads the manifests by hand (a JSON reader for `cargo metadata` would be a new dependency). The graph itself is checked
//! by the gate `cargo tree -p fs-starter -e normal,build -i fs-conformance` (without the feature: no hit).

use std::fs;
use std::path::PathBuf;

/// One line of a normal-dependency table: the name and the rest of the line.
fn normal_deps(manifest: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut table = String::new();
    for line in manifest.lines().map(str::trim) {
        if let Some(head) = line.strip_prefix('[') {
            head.trim_end_matches(']').clone_into(&mut table);
            // `[dependencies.NAME]` style: the whole table belongs to NAME.
            if let Some(name) = table
                .strip_prefix("dependencies.")
                .or_else(|| table.strip_prefix("build-dependencies."))
            {
                out.push((name.to_owned(), String::new()));
            }
        } else if (table == "dependencies"
            || table == "build-dependencies"
            || table.ends_with(".dependencies"))
            && !line.starts_with('#')
            && let Some((name, rest)) = line.split_once('=')
        {
            out.push((name.trim().to_owned(), rest.to_owned()));
        } else if let Some(name) = table.strip_prefix("dependencies.")
            && line.starts_with("optional")
        {
            out.push((format!("{name}#optional"), line.to_owned()));
        }
    }
    out
}

fn own_manifest() -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("own manifest")
}

/// `fs-conformance` is a normal dependency only as an optional one (feature `conformance`), and no `default` feature names it.
#[test]
fn conformance_only_optional_and_not_default() {
    let text = own_manifest();
    let deps = normal_deps(&text);
    let hits: Vec<_> = deps.iter().filter(|(n, _)| n == "fs-conformance").collect();
    assert_eq!(hits.len(), 1, "exactly one normal entry: {hits:?}");
    assert!(
        hits[0].1.contains("optional = true"),
        "fs-conformance must be optional: {hits:?}"
    );
    // No default feature turns it on.
    let mut in_features = false;
    for line in text.lines().map(str::trim) {
        if let Some(head) = line.strip_prefix('[') {
            in_features = head.trim_end_matches(']') == "features";
        } else if in_features && line.starts_with("default") {
            assert!(
                !line.contains("conformance"),
                "default must not turn on conformance: {line}"
            );
        }
    }
}

/// No other member of the workspace has `fs-conformance` as a normal dependency.
#[test]
fn no_sibling_depends_on_it() {
    let crates = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    for entry in fs::read_dir(crates).expect("crates dir") {
        let dir = entry.expect("entry").path();
        let Ok(text) = fs::read_to_string(dir.join("Cargo.toml")) else {
            continue;
        };
        if dir.file_name().is_some_and(|n| n == "fs-starter") {
            continue;
        }
        assert!(
            !normal_deps(&text)
                .iter()
                .any(|(n, _)| n.starts_with("fs-conformance")),
            "{} depends on fs-conformance",
            dir.display()
        );
    }
}

/// The scanner finds normal dependencies in all three forms and ignores dev ones.
#[test]
fn scanner_self_test() {
    let names = |t: &str| {
        normal_deps(t)
            .into_iter()
            .map(|(n, _)| n)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        names("[dependencies]\nfs-conformance = \"1\"\n"),
        ["fs-conformance"]
    );
    assert_eq!(
        names("[dependencies.fs-conformance]\nversion = \"1\"\n"),
        ["fs-conformance"]
    );
    assert_eq!(
        names("[target.'cfg(unix)'.dependencies]\nfs-conformance = \"1\"\n"),
        ["fs-conformance"]
    );
    assert_eq!(
        names("[dev-dependencies]\nfs-conformance = \"1\"\n"),
        Vec::<String>::new()
    );
}
