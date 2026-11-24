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

/// Each lifted library is its tag, or (value "REV:<sha>") the exact commit of a main head not yet tagged (fs-forms, fs-components).
#[test]
fn the_lifted_libraries_are_pinned_by_their_tags_or_commits() {
    for (name, tag) in [
        ("fs-types", "v0.0.6"),
        ("fs-conformance", "v0.0.6"),
        ("fs-interfaces", "v0.1.0"),
        ("fs-objects", "v0.0.10"),
        (
            "fs-settings",
            "REV:9cea35b045d3fff994358b2c118d693f0cf20246",
        ),
        (
            "fs-components-commands",
            "REV:9cea35b045d3fff994358b2c118d693f0cf20246",
        ),
        ("fs-rpc", "v0.1.3"),
        (
            "fs-components-kanban",
            "REV:9cea35b045d3fff994358b2c118d693f0cf20246",
        ),
        ("fs-trusted-path", "v0.1.3"),
        (
            "fs-forms-run",
            "REV:3fe3cf3bf07babda7b9eb7215d6daa938f7633f6",
        ),
        (
            "fs-forms-bind",
            "REV:3fe3cf3bf07babda7b9eb7215d6daa938f7633f6",
        ),
        (
            "fs-components",
            "REV:9cea35b045d3fff994358b2c118d693f0cf20246",
        ),
    ] {
        let source = source_of(name);
        let want = match tag.strip_prefix("REV:") {
            Some(sha) => format!("?rev={sha}#{sha}"),
            None => format!("?tag={tag}#"),
        };
        assert!(source.contains(&want), "{name}: {source} is not {tag}");
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
    assert_eq!(source_of("fs-rpc"), source_of("fs-trusted-path"));
    assert!(source_of("fs-rpc").contains("github.com/FreeSynergy-Net/fs-rpc?tag="));
}
