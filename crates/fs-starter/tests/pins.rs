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
        (
            "fs-interfaces",
            "REV:e6f69213b68b47e139fc8038c8a095f9ed1fef3c",
        ),
        ("fs-objects", "REV:f6409f2a5378ba1464e69ae216efd4d27d249182"),
        (
            "fs-settings",
            "REV:8d52388ae8b0ce425fe93309f846cb70c571c12b",
        ),
        (
            "fs-components-commands",
            "REV:8d52388ae8b0ce425fe93309f846cb70c571c12b",
        ),
        ("fs-rpc", "REV:11cdf71482cfcd8de14038c4e7fc796f2611e02c"),
        (
            "fs-components-kanban",
            "REV:8d52388ae8b0ce425fe93309f846cb70c571c12b",
        ),
        (
            "fs-trusted-path",
            "REV:11cdf71482cfcd8de14038c4e7fc796f2611e02c",
        ),
        (
            "fs-forms-run",
            "REV:3130014fa0f1a5794785aa7902b9cec7568fab37",
        ),
        (
            "fs-forms-bind",
            "REV:3130014fa0f1a5794785aa7902b9cec7568fab37",
        ),
        (
            "fs-components",
            "REV:8d52388ae8b0ce425fe93309f846cb70c571c12b",
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
    assert!(source_of("fs-rpc").contains("github.com/FreeSynergy-Net/fs-rpc?rev="));
}
