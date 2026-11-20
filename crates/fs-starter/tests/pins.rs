//! The pins of the lifted libraries, read from `Cargo.lock` (K4b, mutant P4: `fs-conformance` back on the `rev` of fs-types main 339aa24
//! has no `fs-types` of its own and so keeps "one source" green; only the exact tag shows it).

/// The `source` line of the package `name` in the workspace lock (there must be exactly one).
fn source_of(name: &str) -> String {
    let lock = include_str!("../../../Cargo.lock");
    let mut found = Vec::new();
    for block in lock.split("[[package]]") {
        if block
            .lines()
            .any(|l| l.trim() == format!("name = \"{name}\""))
        {
            let source = block
                .lines()
                .find_map(|l| l.strip_prefix("source = "))
                .unwrap_or("")
                .trim_matches('"')
                .to_owned();
            found.push(source);
        }
    }
    assert_eq!(
        found.len(),
        1,
        "{name}: one source in the lock, found {found:?}"
    );
    found.remove(0)
}

/// Each lifted library is the tag, nothing else: no `rev`, no branch.
#[test]
fn the_lifted_libraries_are_pinned_by_their_tags() {
    for (name, tag) in [
        ("fs-types", "v0.0.4"),
        ("fs-conformance", "v0.0.4"),
        ("fs-interfaces", "v0.0.29"),
        ("fs-objects", "v0.0.8"),
        ("fs-settings", "v0.0.33"),
    ] {
        let source = source_of(name);
        assert!(
            source.contains(&format!("?tag={tag}#")),
            "{name}: {source} is not the tag {tag}"
        );
    }
}

/// `fs-conformance` lives in the repo fs-types: the same commit as `fs-types` (one source, and the right one).
#[test]
fn the_conformance_is_the_commit_of_fs_types() {
    assert_eq!(source_of("fs-conformance"), source_of("fs-types"));
}
