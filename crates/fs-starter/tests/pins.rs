//! The pins of the lifted libraries, read from `Cargo.lock` (K4b, mutant P4: `fs-conformance` back on the `rev` of fs-types main 339aa24
//! has no `fs-types` of its own and so keeps "one source" green; only the exact tag shows it).

/// The doubles that are KNOWN and wait for a follow-up commit of their owner (E3, 24.11.): fs-forms 687998fa still pins fs-rpc 7f214f93,
/// and the dev-dependency fs-modules-planning e567cd79 still pins fs-rpc 11cdf714 (tests only). fs-rpc 0120f9af is the one this repo uses
/// (it holds `fs-settings-rpc` on fs-components d53126ba). Nothing else is allowed twice; a line here is deleted as soon as its owner
/// moved (the test below fails when it is no longer needed).
const KNOWN_OLD: [(&str, &str); 4] = [
    ("fs-rpc", "?rev=7f214f935e9913e8cca5e275ab1fcd5b9dcfc170#"),
    ("fs-rpc", "?rev=11cdf71482cfcd8de14038c4e7fc796f2611e02c#"),
    (
        "fs-trusted-path",
        "?rev=7f214f935e9913e8cca5e275ab1fcd5b9dcfc170#",
    ),
    (
        "fs-trusted-path",
        "?rev=11cdf71482cfcd8de14038c4e7fc796f2611e02c#",
    ),
];

/// The `source` line of the package `name` in the workspace lock (there must be exactly one, apart from the `KNOWN_OLD` doubles).
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
    let old: Vec<&str> = KNOWN_OLD
        .iter()
        .filter(|(n, _)| *n == name)
        .map(|(_, s)| *s)
        .collect();
    found.retain(|s| !old.iter().any(|o| s.contains(o)));
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
            "fs-settings-rpc",
            "REV:0120f9afadf5b74540e8df3f0dfdd1b2edb4e1d4",
        ),
        (
            "fs-settings",
            "REV:d53126ba3948288cb965095a7e6a8d7189f02df1",
        ),
        (
            "fs-components-commands",
            "REV:d53126ba3948288cb965095a7e6a8d7189f02df1",
        ),
        ("fs-rpc", "REV:0120f9afadf5b74540e8df3f0dfdd1b2edb4e1d4"),
        (
            "fs-components-kanban",
            "REV:d53126ba3948288cb965095a7e6a8d7189f02df1",
        ),
        (
            "fs-trusted-path",
            "REV:0120f9afadf5b74540e8df3f0dfdd1b2edb4e1d4",
        ),
        (
            "fs-forms-run",
            "REV:687998faf266cd5ffc2ff47feba2a195b1ed4aae",
        ),
        (
            "fs-forms-bind",
            "REV:687998faf266cd5ffc2ff47feba2a195b1ed4aae",
        ),
        (
            "fs-components",
            "REV:d53126ba3948288cb965095a7e6a8d7189f02df1",
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

/// A `KNOWN_OLD` line that the lock no longer has is removed (the double is gone: the one-source rule is strict again).
#[test]
fn every_known_old_double_is_still_in_the_lock() {
    let lock = include_str!("../../../Cargo.lock");
    for (name, source) in KNOWN_OLD {
        let has = lock.split("[[package]]").any(|b| {
            b.lines().any(|l| l.trim() == format!("name = \"{name}\"")) && b.contains(source)
        });
        assert!(
            has,
            "{name} {source} is gone from the lock: delete the line of KNOWN_OLD"
        );
    }
}
