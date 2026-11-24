//! The pins of the lifted libraries, read from `Cargo.lock` (K4b, mutant P4: `fs-conformance` back on the `rev` of fs-types main 339aa24
//! has no `fs-types` of its own and so keeps "one source" green; only the exact tag shows it).

/// The `source` lines of the package `name` in the workspace lock.
fn sources_of(name: &str) -> Vec<String> {
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
    found
}

/// The `source` line of the package `name` (there must be exactly one).
fn source_of(name: &str) -> String {
    let mut found = sources_of(name);
    assert_eq!(
        found.len(),
        1,
        "{name}: one source in the lock, found {found:?}"
    );
    found.remove(0)
}

/// The `source` line of `name` at the tag the host pins itself. `fs-forms` v0.1.0 still pins `fs-rpc` v0.1.0, so the lock holds
/// that copy too until fs-forms is re-pinned (open point of this bundle); no other source is allowed.
fn own_source_of(name: &str, tag: &str) -> String {
    let found = sources_of(name);
    let own: Vec<_> = found
        .iter()
        .filter(|s| s.contains(&format!("?tag={tag}#")))
        .collect();
    assert_eq!(
        own.len(),
        1,
        "{name}: no single source at {tag} in {found:?}"
    );
    assert!(
        found.len() <= 2,
        "{name}: more than the own tag and the one of fs-forms: {found:?}"
    );
    own[0].clone()
}

/// Each lifted library is the tag, nothing else: no `rev`, no branch.
#[test]
fn the_lifted_libraries_are_pinned_by_their_tags() {
    for (name, tag) in [
        ("fs-types", "v0.0.6"),
        ("fs-conformance", "v0.0.6"),
        ("fs-interfaces", "v0.0.35"),
        ("fs-objects", "v0.0.10"),
        ("fs-settings", "v0.1.0"),
        ("fs-components-commands", "v0.1.0"),
        ("fs-rpc", "v0.1.1"),
        ("fs-trusted-path", "v0.1.1"),
        ("fs-forms-run", "v0.1.0"),
        ("fs-forms-bind", "v0.1.0"),
        ("fs-components", "v0.1.0"),
    ] {
        let source = if name == "fs-rpc" || name == "fs-trusted-path" {
            own_source_of(name, tag)
        } else {
            source_of(name)
        };
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

/// `fs-rpc` and `fs-trusted-path` are two crates of one repository: the same repository URL and the same commit (one source each).
#[test]
fn the_rpc_and_the_trusted_path_come_from_one_repository_at_one_commit() {
    assert_eq!(
        own_source_of("fs-rpc", "v0.1.1"),
        own_source_of("fs-trusted-path", "v0.1.1")
    );
    assert!(own_source_of("fs-rpc", "v0.1.1").contains("github.com/FreeSynergy-Net/fs-rpc?tag="));
}
